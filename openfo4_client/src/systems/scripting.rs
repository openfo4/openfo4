pub mod invocations;
pub mod loader;

mod classes;

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use async_lock::Mutex;
use futures::future::BoxFuture;
use futures_lite::FutureExt;
use game_common::collections::arena::{Arena, Key};
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
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};

use classes::State;

const COMPILER_OPTIONS: Options = Options {
    skip_native: true,
    skip_empty: true,
    only_events: true,
    use_native_stubs: false,
};

pub struct Executor {
    loader: Arc<ScriptLoader>,
    invocations: Arena<EventInvocation>,
    state: Option<State>,
    suspended_calls: Vec<SuspendedCall>,
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
        });

        Self {
            loader,
            invocations: Arena::new(),
            state: Some(state),
            suspended_calls: Vec::new(),
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
        let id = self.invocations.insert(invocation.into());
        InvocationId(id)
    }

    pub fn update(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        globals: &mut GlobalProperties,
        delta: Duration,
    ) {
        let _span = trace_span!("Executor::update").entered();

        let state = self.state.as_mut().unwrap();
        state.world = world;
        state.globals = globals;

        self.resume_suspended_tasks();

        let engine = futures_lite::future::block_on(self.loader.engine.lock());
        let mut instances = futures_lite::future::block_on(self.loader.instances.lock());

        for invocation in self.invocations.values() {
            let object = &invocation.receiver;

            let scripts = instances.get_mut(object.0.id.0).unwrap();

            let suspended_calls =
                call_event_invocation(&engine, scripts, &mut self.state, invocation);
            for (name, future) in suspended_calls {
                self.suspended_calls.push(SuspendedCall {
                    object: object.clone(),
                    script: name,
                    invocation: invocation.clone(),
                    future,
                });
            }
        }

        let state = self.state.as_mut().unwrap();
        // FIXME: Not necessarly required, we can leave pointers dangling.
        state.world = core::ptr::null_mut();
        state.quests = core::ptr::null_mut();

        // for entry in real_timers.tick(delta) {
        //     self.invocations.insert(Invocation::Timer {
        //         object: entry.object,
        //         timer: entry.id,
        //     });
        // }
    }

    fn resume_suspended_tasks(&mut self) {
        let _span = trace_span!("Executor::resume_suspended_tasks").entered();

        let engine = futures_lite::future::block_on(self.loader.engine.lock());
        let mut instances = futures_lite::future::block_on(self.loader.instances.lock());

        let mut ctx = std::task::Context::from_waker(std::task::Waker::noop());
        self.suspended_calls.retain_mut(|call| {
            if call.future.poll(&mut ctx).is_pending() {
                return true;
            }

            let scripts = instances.get_mut(call.object.0.id.0).unwrap();
            let script = scripts.get_mut(&call.script).unwrap();

            if let Some(waker) =
                call_event_invocation_single(&engine, script, &mut self.state, &call.invocation)
            {
                call.future = waker;
                // Back to suspension...
                return true;
            }

            false
        });
    }

    fn poll_timers(&mut self, delta: Duration) {
        let _span = trace_span!("Executor::poll_timers").entered();

        let state = self.state.as_mut().unwrap();
    }
}

fn call_event_invocation(
    engine: &Engine,
    scripts: &mut HashMap<String, ScriptInstance>,
    state: &mut Option<State>,
    invocation: &EventInvocation,
) -> HashMap<String, BoxFuture<'static, ()>> {
    let mut suspended_calls = HashMap::new();
    for (script_name, script) in scripts {
        if let Some(future) = call_event_invocation_single(engine, script, state, invocation) {
            suspended_calls.insert(script_name.clone(), future);
        }
    }
    suspended_calls
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
    object: ScriptObject,
    script: String,
    invocation: EventInvocation,
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
}

impl Objects {
    pub fn quest(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.quests.get(&addr).copied()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(Key);
