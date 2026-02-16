pub mod invocations;
pub mod loader;

mod classes;
mod effects;
mod events;
mod objects;
mod registration;

use std::collections::VecDeque;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use async_lock::Mutex;
use futures::future::BoxFuture;
use futures_lite::FutureExt;
use game_common::collections::arena::{Arena, Key};
use game_common::collections::bimap::BiMap;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_pex::Module;
use openfo4_script::compiler::TypeLayout;
use openfo4_script::engine::{Engine, Instance, Options};
use openfo4_script::runtime::{Context, Exception, ObjectAddr, ObjectRef, Stack, Val};

use crate::components::Player;
use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent};
use crate::events::trigger::{TriggerEnter, TriggerLeave};
use crate::loader::{FileLoader, fix_path};
use crate::state::esp::EspLoader;
use crate::systems::TransferItemEvent;
use crate::systems::global::GlobalProperties;
use crate::systems::inventory::Inventory;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::classes::{AliasSet, CoreObjects};
use crate::systems::scripting::effects::{EffectId, Effects};
use crate::systems::scripting::invocations::{EventInvocation, Value};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObjectId};
use crate::systems::scripting::objects::ScriptObjects;
use crate::systems::scripting::registration::{EventKind, EventSender, RegistrationSet};

use classes::State;

const COMPILER_OPTIONS: Options = Options {
    skip_native: true,
    skip_empty: true,
    only_events: true,
    use_native_stubs: false,
};

pub struct Executor {
    loader: Arc<ScriptLoader>,
    invocations: Arena<InvocationState>,
    suspended_calls: HashMap<EffectId, ScheduledTask>,
    queue: VecDeque<ScheduledTask>,
}

impl Executor {
    pub async fn new(
        esp_loader: Arc<EspLoader>,
        file_loader: Arc<FileLoader>,
        pool: Arc<TaskPool>,
    ) -> Self {
        let _span = trace_span!("Executor::new").entered();

        let mut engine = Engine::new();

        for path in file_loader.files() {
            let path = fix_path(path);
            if path.starts_with("scripts/") && path.ends_with(".pex") {
                let mut reader = file_loader.open(&path).await.unwrap();
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf).unwrap();

                let module = Module::decode(&buf[..]).unwrap();
                engine.register(&module);
            }
        }

        classes::register_builtins(&mut engine);

        let state = core_actor_values(&mut engine);

        let engine = Arc::new(Mutex::new(engine));

        let loader = Arc::new(ScriptLoader {
            engine,
            classes: Mutex::default(),
            esp_loader,
            state: Mutex::new(Some(state)),
        });

        Self {
            loader,
            invocations: Arena::new(),
            suspended_calls: HashMap::new(),
            queue: VecDeque::new(),
        }
    }

    pub fn loader(&self) -> &Arc<ScriptLoader> {
        &self.loader
    }

    /// Spawns a new script invocation on the `Executor`.
    ///
    /// The script will be invoked when ready.
    pub fn invoke<T>(&mut self, invocation: T) -> InvocationId
    where
        T: Into<EventInvocation>,
    {
        self.invoke_inner(invocation.into())
    }

    fn invoke_inner(&mut self, invocation: EventInvocation) -> InvocationId {
        let state = futures_lite::future::block_on(self.loader.state.lock());
        let state = state.as_ref().unwrap();

        let scripts = state.objects.get_by_id(invocation.receiver.id()).unwrap();

        let id = self.invocations.insert(InvocationState {
            running_tasks: scripts.len(),
        });

        for script in scripts.iter() {
            self.queue.push_back(ScheduledTask {
                object: invocation.receiver.id(),
                script: script.name().to_owned(),
                invocation_id: Some(InvocationId(id)),
                func_name: invocation.name.to_string(),
                args: invocation.args.clone(),
            });
        }

        InvocationId(id)
    }

    /// Returns `true` if the previously spawned invocation via [`invoke`] has complted execution.
    ///
    /// [`invoke`]: Self::invoke
    pub fn is_invocation_done(&self, id: InvocationId) -> bool {
        !self.invocations.contains_key(id.0)
    }

    pub fn update(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        globals: &mut GlobalProperties,
        script_entities: &mut ScriptEntities,
        quests: &mut QuestManager,
        pool: &TaskPool,
        esp_loader: &Arc<EspLoader>,
        delta: Duration,
    ) {
        let _span = trace_span!("Executor::update").entered();

        self.collect_game_events(world, events, script_entities);

        let engine = futures_lite::future::block_on(self.loader.engine.lock());
        let mut state = futures_lite::future::block_on(self.loader.state.lock());

        for (entity, _) in world.query::<&Player>().iter() {
            let addr = state.as_mut().unwrap().core_objects.player.addr();
            let script = state.as_mut().unwrap().objects.get_by_addr(addr).unwrap();
            script_entities.0.insert(script.id(), entity);
        }

        // FIXME: Don't recompute this every update.
        let mut entities = HashMap::new();
        for scripts in state.as_ref().unwrap().objects.iter() {
            for script in scripts.iter() {
                entities.insert(script.get().object.addr(), scripts.id());
            }
        }

        state.as_mut().unwrap().world = world;
        state.as_mut().unwrap().globals = globals;
        state.as_mut().unwrap().game_entities = entities;
        state.as_mut().unwrap().entities = script_entities;
        state.as_mut().unwrap().quests = quests;

        while let Some(task) = self.queue.pop_front() {
            let scripts = state
                .as_mut()
                .unwrap()
                .objects
                .get_by_id(task.object)
                .unwrap();
            let script = scripts.get(&task.script).unwrap();

            if let Some(id) = call_event_invocation_single(
                &engine,
                script.id(),
                &script.name().to_owned(),
                &mut state,
                &task.func_name,
                &task.args,
            ) {
                self.suspended_calls.insert(id, task);
                continue;
            }

            if let Some(invocation_id) = task.invocation_id {
                let invocation = self.invocations.get_mut(invocation_id.0).unwrap();
                invocation.running_tasks -= 1;
                if invocation.running_tasks == 0 {
                    self.invocations.remove(invocation_id.0).unwrap();
                }
            }
        }

        let state = state.as_mut().unwrap();
        let completed_effects = state
            .effects
            .run(pool, world, events, esp_loader, self.loader());

        for effect in completed_effects {
            if let Some(task) = self.suspended_calls.remove(&effect) {
                self.queue.push_back(task);
            }
        }

        // FIXME: Not necessarly required, we can leave pointers dangling.
        // state.world = core::ptr::null_mut();
        // state.quests = core::ptr::null_mut();
        // state.entities = core::ptr::null();

        // for entry in real_timers.tick(delta) {
        //     self.invocations.insert(Invocation::Timer {
        //         object: entry.object,
        //         timer: entry.id,
        //     });
        // }
    }

    fn collect_game_events(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        entities: &ScriptEntities,
    ) {
        let _span = trace_span!("Executor::collect_game_events").entered();

        let mut state = futures_lite::future::block_on(self.loader.state.lock());
        let state = state.as_mut().unwrap();

        fn collect_event_dispatches<T: events::Event + openfo4_core::events::Event>(
            dst: &mut Vec<events::DispatchEvent>,
            events: &EventBus,
            ctx: events::Context<'_>,
        ) {
            for event in events.read::<T>() {
                dst.extend(event.extract(ctx));
            }
        }

        let ctx = events::Context {
            state: &state,
            world: &world,
            entities,
        };

        let mut dispatches = Vec::new();
        collect_event_dispatches::<ItemAddedEvent>(&mut dispatches, events, ctx);
        collect_event_dispatches::<ItemEquippedEvent>(&mut dispatches, events, ctx);
        collect_event_dispatches::<TriggerEnter>(&mut dispatches, events, ctx);
        collect_event_dispatches::<TriggerLeave>(&mut dispatches, events, ctx);

        for dispatch in dispatches {
            tracing::debug!(
                "dispatching event {:?} for {:?}",
                dispatch.kind,
                dispatch.receiver,
            );

            // Dispatch one event for every script object.
            let scripts = state.objects.get_by_id(dispatch.receiver).unwrap();
            for script in scripts.iter() {
                for (receiver, event) in state.registration.get(
                    &dispatch.kind,
                    &EventSender::Object(script.get().object.addr()),
                ) {
                    let receiver = state.objects.get_by_addr(*receiver).unwrap();

                    if receiver.get().object.addr() == script.get().object.addr() {
                        self.queue.push_back(ScheduledTask {
                            object: receiver.id(),
                            script: receiver.name().to_owned(),
                            invocation_id: None,
                            func_name: event.kind.func_name().to_owned(),
                            args: dispatch.args.to_vec(),
                        });
                    } else {
                        let mut args = dispatch.args.to_vec();
                        args.insert(0, Value::ObjectRef(script.get().object.clone()));

                        self.queue.push_back(ScheduledTask {
                            object: receiver.id(),
                            script: receiver.name().to_owned(),
                            invocation_id: None,
                            func_name: event.kind.remote_func_name().to_owned(),
                            args,
                        });
                    }
                }

                for alias in state
                    .aliases
                    .get_alises_for_object(script.get().object.addr())
                {
                    let alias = state.objects.get_by_addr(alias).unwrap();

                    let mut args = dispatch.args.to_vec();
                    args.insert(0, Value::ObjectRef(script.get().object.clone()));

                    self.queue.push_back(ScheduledTask {
                        object: alias.id(),
                        script: alias.name().to_owned(),
                        invocation_id: None,
                        func_name: dispatch.kind.remote_ref_alias_func_name().to_owned(),
                        args,
                    });
                }
            }
        }
    }

    fn poll_timers(&mut self, delta: Duration) {
        let _span = trace_span!("Executor::poll_timers").entered();

        // let state = self.state.as_mut().unwrap();
    }
}

#[derive(Clone, Debug)]
struct ScheduledTask {
    object: ScriptObjectId,
    script: String,
    invocation_id: Option<InvocationId>,
    func_name: String,
    args: Vec<Value>,
}

#[derive(Clone, Debug)]
struct InvocationState {
    /// Tasks that are not completed yet.
    running_tasks: usize,
}

fn call_event_invocation_single(
    engine: &Engine,
    id: ScriptObjectId,
    script_name: &str,
    state: &mut Option<State>,
    func_name: &str,
    args: &[Value],
) -> Option<EffectId> {
    let mut script = {
        let state = state.as_mut().unwrap();
        let scripts = state.objects.instances.get(id.0).unwrap();
        let index = *scripts.class_names.get(script_name).unwrap();
        scripts.scripts[index].clone()
    };

    let Some(func) = script.instance.get_func(func_name) else {
        return None;
    };

    let stack = script
        .stacks
        .entry(func_name.to_string())
        .or_insert_with(|| Stack::for_func(func));

    let mut args = args
        .iter()
        .map(|value| match value {
            Value::Int(v) => Val::I32(*v),
            Value::ObjectRef(object) => Val::Ptr(object.addr().to_raw()),
            Value::ObjectAddr(addr) => Val::Ptr(addr.to_raw()),
        })
        .collect::<Vec<_>>();
    args.insert(0, Val::Ptr(script.object.addr().to_raw()));

    let s = state.take().unwrap();
    let mut ctx = Context::new(engine, stack, s);
    tracing::debug!(
        "calling {}.{} ({:?})",
        script_name,
        func_name,
        script.object.addr()
    );
    let res = func.call(&mut ctx, &args);

    let mut data = ctx.into_data();
    let mut wait_on_effect = None;
    match res {
        Ok(_) | Err(Exception::Error) => {
            // Execution is done, reset the stack.
            script.stacks.remove(func_name);
        }
        Err(Exception::Suspend) => {
            debug_assert!(
                data.wait_on_effect.is_some(),
                "suspended script without waiting on effect"
            );

            wait_on_effect = data.wait_on_effect.take();
        }
    }

    *state = Some(data);

    // FIXME: Avoid this hack.
    {
        let state = state.as_mut().unwrap();
        let scripts = state.objects.instances.get_mut(id.0).unwrap();
        let index = *scripts.class_names.get_mut(script_name).unwrap();
        scripts.scripts[index] = script;
    }

    wait_on_effect
}

struct SuspendedCall {
    task: ScheduledTask,
    future: BoxFuture<'static, ()>,
}

fn core_actor_values(engine: &mut Engine) -> State {
    let actor = engine.compile("Actor", &COMPILER_OPTIONS);
    engine.compile("ActorValue", &COMPILER_OPTIONS);

    let actor_layout = engine.get_class_layout("Actor").unwrap();
    let actor_value = engine.get_class_layout("ActorValue").unwrap();

    let [
        health,
        strength,
        perception,
        endurance,
        charisma,
        intelligence,
        agility,
        luck,
        aggression,
        confidence,
        suspicious,
    ] = std::array::from_fn(|_| engine.object_pool().insert(actor_value.clone()).finalize());

    let player = ScriptInstance {
        layout: actor_layout.clone(),
        instance: actor,
        object: engine.object_pool().insert(actor_layout.clone()).finalize(),
        stacks: HashMap::new(),
    };

    let mut objects = ScriptObjects::default();
    objects.insert(vec![player.clone()], None);

    State {
        core_objects: CoreObjects {
            player: player.object.clone(),
            health,
            strength,
            perception,
            endurance,
            charisma,
            intelligence,
            agility,
            luck,
            aggression,
            confidence,
            suspicious,
        },
        real_timers: Default::default(),
        game_timers: Default::default(),
        world: core::ptr::null_mut(),
        quests: core::ptr::null_mut(),
        objects,
        globals: core::ptr::null_mut(),
        registration: RegistrationSet::default(),
        entities: core::ptr::null(),
        game_entities: HashMap::new(),
        aliases: AliasSet::default(),
        effects: Effects::default(),
        wait_on_effect: None,
    }
}

#[derive(Clone, Debug)]
struct ScriptInstance {
    instance: Instance,
    layout: TypeLayout,
    object: ObjectRef,
    stacks: HashMap<String, Stack>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(Key);

/// Maps [`ScriptObject`]s to game world [`Entity`].
#[derive(Clone, Debug, Default)]
pub struct ScriptEntities(pub BiMap<ScriptObjectId, Entity>);
