pub mod invocations;
pub mod loader;

mod classes;
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
use game_egui::egui::TextBuffer;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_pex::Module;
use openfo4_script::compiler::TypeLayout;
use openfo4_script::engine::{Engine, Instance, Options};
use openfo4_script::runtime::{Context, Exception, ObjectAddr, ObjectRef, Stack, Val};

use crate::loader::{FileLoader, fix_path};
use crate::state::esp::EspLoader;
use crate::systems::global::GlobalProperties;
use crate::systems::scripting::classes::CoreObjects;
use crate::systems::scripting::invocations::{EventInvocation, Value};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject, ScriptObjectId};
use crate::systems::scripting::registration::RegistrationSet;

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
    suspended_calls: Vec<SuspendedCall>,
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
            instances: Mutex::default(),
            base_records: Mutex::default(),
            esp_loader,
            state: Mutex::new(Some(state)),
        });

        Self {
            loader,
            invocations: Arena::new(),
            suspended_calls: Vec::new(),
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
        let instances = futures_lite::future::block_on(self.loader.instances.lock());

        let scripts = instances.get(invocation.receiver.0.id.0).unwrap();

        let id = self.invocations.insert(InvocationState {
            running_tasks: scripts.len(),
        });

        for script in scripts.keys() {
            self.queue.push_back(ScheduledTask {
                invocation: invocation.clone(),
                object: invocation.receiver.clone(),
                script: script.clone(),
                invocation_id: InvocationId(id),
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
        script_entities: &ScriptEntities,
        delta: Duration,
    ) {
        let _span = trace_span!("Executor::update").entered();

        self.resume_suspended_tasks();

        let engine = futures_lite::future::block_on(self.loader.engine.lock());
        let mut instances = futures_lite::future::block_on(self.loader.instances.lock());
        let mut state = futures_lite::future::block_on(self.loader.state.lock());

        // FIXME: Don't recompute this every update.
        let mut entities = HashMap::new();
        for (id, scripts) in &*instances {
            let id = ScriptObjectId(id);

            for script in scripts.values() {
                entities.insert(script.object.addr(), id);
            }
        }

        state.as_mut().unwrap().world = world;
        state.as_mut().unwrap().globals = globals;
        state.as_mut().unwrap().objects.entities = entities;
        state.as_mut().unwrap().entities = script_entities;

        while let Some(task) = self.queue.pop_front() {
            let scripts = instances.get_mut(task.object.0.id.0).unwrap();
            let script = scripts.get_mut(&task.script).unwrap();

            if let Some(future) =
                call_event_invocation_single(&engine, script, &mut state, &task.invocation)
            {
                self.suspended_calls.push(SuspendedCall { task, future });
                continue;
            }

            let invocation = self.invocations.get_mut(task.invocation_id.0).unwrap();
            invocation.running_tasks -= 1;
            if invocation.running_tasks == 0 {
                self.invocations.remove(task.invocation_id.0).unwrap();
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

    fn resume_suspended_tasks(&mut self) {
        let _span = trace_span!("Executor::resume_suspended_tasks").entered();

        let mut ctx = std::task::Context::from_waker(std::task::Waker::noop());
        self.suspended_calls.retain_mut(|call| {
            if call.future.poll(&mut ctx).is_pending() {
                return true;
            }

            self.queue.push_back(call.task.clone());
            false
        });
    }

    fn poll_timers(&mut self, delta: Duration) {
        let _span = trace_span!("Executor::poll_timers").entered();

        // let state = self.state.as_mut().unwrap();
    }
}

#[derive(Clone, Debug)]
struct ScheduledTask {
    invocation: EventInvocation,
    object: ScriptObject,
    script: String,
    invocation_id: InvocationId,
}

#[derive(Clone, Debug)]
struct InvocationState {
    /// Tasks that are not completed yet.
    running_tasks: usize,
}

fn call_event_invocation_single(
    engine: &Engine,
    script: &mut ScriptInstance,
    state: &mut Option<State>,
    invocation: &EventInvocation,
) -> Option<BoxFuture<'static, ()>> {
    let Some(func) = script.instance.get_func(&invocation.name) else {
        return None;
    };

    let stack = script
        .stacks
        .entry(invocation.name.to_string())
        .or_insert_with(|| Stack::for_func(func));

    let mut args = invocation
        .args
        .iter()
        .map(|value| match value {
            Value::Int(v) => Val::I32(*v),
        })
        .collect::<Vec<_>>();
    args.insert(0, Val::Ptr(script.object.addr().to_raw()));

    let s = state.take().unwrap();
    let mut ctx = Context::new(engine, stack, s);
    tracing::debug!("calling {} on {:?}", invocation.name, script.object.addr());
    let res = func.call(&mut ctx, &args);

    let mut data = ctx.into_data();
    let mut waker = None;
    match res {
        Ok(_) | Err(Exception::Error) => {
            // Execution is done, reset the stack.
            script.stacks.remove(invocation.name.as_str());
        }
        Err(Exception::Suspend) => {
            debug_assert!(data.waker.is_some(), "suspended script without waker");
            waker = data.waker.take();
        }
    }

    *state = Some(data);
    waker
}

struct SuspendedCall {
    task: ScheduledTask,
    future: BoxFuture<'static, ()>,
}

fn core_actor_values(engine: &mut Engine) -> State {
    engine.compile("Actor", &COMPILER_OPTIONS);
    engine.compile("ActorValue", &COMPILER_OPTIONS);

    let actor = engine.get_class_layout("Actor").unwrap();
    let actor_value = engine.get_class_layout("ActorValue").unwrap();

    let player = engine.object_pool().insert(actor.clone()).finalize();

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

    State {
        core_objects: CoreObjects {
            player,
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
        objects: Default::default(),
        waker: None,
        globals: core::ptr::null_mut(),
        registration: RegistrationSet::default(),
        entities: core::ptr::null(),
    }
}

#[derive(Clone, Debug)]
struct ScriptInstance {
    instance: Instance,
    layout: TypeLayout,
    object: ObjectRef,
    stacks: HashMap<String, Stack>,
}

#[derive(Clone, Debug, Default)]
struct Objects {
    globals: HashMap<ObjectAddr, RecordId>,
    keywords: HashMap<ObjectAddr, RecordId>,
    objects: HashMap<ObjectAddr, Entity>,
    actor_values: HashMap<ObjectAddr, RecordId>,
    quests: HashMap<ObjectAddr, RecordId>,
    perks: HashMap<ObjectAddr, RecordId>,
    form_lists: HashMap<ObjectAddr, RecordId>,
    /// Entities that exist in the game world.
    entities: HashMap<ObjectAddr, ScriptObjectId>,
}

impl Objects {
    pub fn global(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.globals.get(&addr).copied()
    }

    pub fn quest(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.quests.get(&addr).copied()
    }

    pub fn perk(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.perks.get(&addr).copied()
    }

    pub fn keyword(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.keywords.get(&addr).copied()
    }

    /// Look up an item-like record (weapon, armor, etc..), i.e. any item that can be contained
    /// inside an inventory.
    pub fn item(&self, addr: ObjectAddr) -> Option<RecordId> {
        todo!()
    }

    pub fn form(&self, addr: ObjectAddr) -> Option<RecordId> {
        todo!()
    }

    pub fn form_list(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.form_lists.get(&addr).copied()
    }

    pub fn addr(&self, id: RecordId) -> Option<ObjectAddr> {
        todo!()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(Key);

/// Maps [`ScriptObject`]s to game world [`Entity`].
#[derive(Clone, Debug, Default)]
pub struct ScriptEntities(pub BiMap<ScriptObjectId, Entity>);
