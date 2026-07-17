pub mod invocations;
pub mod loader;

mod classes;
mod effects;
mod events;
mod objects;
mod registration;
mod runner;

use std::sync::Arc;
use std::time::Duration;

use futures_lite::future::block_on;
use game_common::collections::arena::{Arena, Key};
use game_common::collections::bimap::BiMap;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use hashbrown::HashMap;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_pex::Module;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::compiler::resources::TypeId;
use openfo4_script::engine::{Engine, Instance, Options};
use openfo4_script::runtime::{ObjectRef, Stack};

use crate::config;
use crate::events::combat::{BleedoutStartEvent, BodyPartCrippleEvent};
use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent};
use crate::events::trigger::{TriggerEnter, TriggerLeave};
use crate::loader::FileLoader;
use crate::state::esp::EspLoader;
use crate::systems::DeathStartEvent;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::classes::{AliasSet, BorrowedState, CoreObjects, SharedBorrowCell};
use crate::systems::scripting::effects::EffectId;
use crate::systems::scripting::events::queue::EventQueueProducer;
use crate::systems::scripting::invocations::EventInvocation;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObjectId};
use crate::systems::scripting::objects::ScriptObjects;
use crate::systems::scripting::registration::RegistrationSet;
use crate::systems::scripting::runner::{Runner, RunnerHandle};

use classes::State;

pub use runner::Invocation;

const COMPILER_OPTIONS: Options = Options {
    skip_native: true,
    skip_empty: true,
    use_native_stubs: false,
};

pub struct Executor {
    loader: Arc<ScriptLoader>,
    invocations: Arena<InvocationState>,
    handle: RunnerHandle,
}

impl Executor {
    pub async fn new(
        esp_loader: Arc<EspLoader>,
        file_loader: Arc<FileLoader>,
        pool: Arc<TaskPool>,
        config: config::Scripting,
    ) -> Self {
        let _span = trace_span!("Executor::new").entered();

        let mut engine = Engine::new(CompilerOptions {
            validation: config.validation,
            debug: config.debug,
        });

        for path in file_loader.files() {
            if path.components().next().is_some_and(|v| v == "scripts")
                && path.extension().is_some_and(|v| v == "pex")
            {
                let buf = file_loader.read(path).await.unwrap();

                let module = Module::decode(&buf[..]).unwrap();
                engine.register(&module);
            }
        }

        classes::register_builtins(&mut engine);

        let engine = Arc::new(engine);
        let objects = Arc::new(ScriptObjects::default());
        let registration = Arc::new(RegistrationSet::default());

        let loader = Arc::new(ScriptLoader::new(
            engine.clone(),
            esp_loader,
            objects.clone(),
            registration.clone(),
        ));

        let state = core_actor_values(&engine, objects, registration, loader.clone());

        let mut runner = Runner::new(state, pool.clone());
        let handle = runner.handle();
        pool.spawn(async move { runner.run().await });

        Self {
            loader,
            invocations: Arena::new(),
            handle,
        }
    }

    pub fn loader(&self) -> &Arc<ScriptLoader> {
        &self.loader
    }

    /// Spawns a new script invocation on the `Executor`.
    ///
    /// The script will be invoked when ready.
    pub fn invoke<T>(&mut self, invocation: T) -> Invocation
    where
        T: Into<EventInvocation>,
    {
        self.handle.push_invocation(invocation.into())
    }

    pub fn update(
        &mut self,
        world: &mut World,
        globals: &mut GlobalProperties,
        script_entities: &mut ScriptEntities,
        quests: &mut QuestManager,
    ) {
        let _span = trace_span!("Executor::update").entered();

        self.handle.tick();

        self.handle.cell.set(BorrowedState {
            world: world,
            quests: quests,
            entities: script_entities,
            globals: globals,
        });

        std::thread::sleep(Duration::from_micros(100));

        let taken = self.handle.cell.take();
        debug_assert!(taken.is_some());
    }

    /// Gathers all scripting events into the `Executor`.
    ///
    /// This should be called late in the frame, after all gameplay systems have emitted their
    /// events. The `Executor` will dispatch the events to scripts the next time [`update`] is
    /// called in the next frame.
    ///
    /// [`update`]: Self::update
    pub fn pump_events(&mut self, world: &mut World, events: &mut EventBus) {
        let _span = trace_span!("Executor::pump_events").entered();

        fn collect_event_dispatches<T: events::ExtractEvent + openfo4_core::events::Event>(
            queue: &mut EventQueueProducer,
            events: &EventBus,
            ctx: events::Context<'_>,
        ) {
            for event in events.read::<T>() {
                for event in event.extract(ctx) {
                    queue.push(ctx.world, event);
                }
            }
        }

        let ctx = events::Context { world: &world };

        collect_event_dispatches::<ItemAddedEvent>(&mut self.handle.events, events, ctx);
        collect_event_dispatches::<ItemEquippedEvent>(&mut self.handle.events, events, ctx);
        collect_event_dispatches::<TriggerEnter>(&mut self.handle.events, events, ctx);
        collect_event_dispatches::<TriggerLeave>(&mut self.handle.events, events, ctx);
        collect_event_dispatches::<DeathStartEvent>(&mut self.handle.events, events, ctx);
        collect_event_dispatches::<BleedoutStartEvent>(&mut self.handle.events, events, ctx);
        collect_event_dispatches::<BodyPartCrippleEvent>(&mut self.handle.events, events, ctx);
    }
}

/// Unique identifier for each script call.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct TaskId(usize);

#[derive(Clone, Debug, Default)]
struct TaskState {
    stack: Option<Arc<parking_lot::Mutex<Stack>>>,
    suspended_on: Option<EffectId>,
}

#[derive(Clone, Debug)]
struct InvocationState {
    /// Tasks that are not completed yet.
    running_tasks: usize,
}

fn core_actor_values(
    engine: &Engine,
    objects: Arc<ScriptObjects>,
    registration: Arc<RegistrationSet>,
    loader: Arc<ScriptLoader>,
) -> State {
    let actor = block_on(engine.compile("Actor", &COMPILER_OPTIONS));
    block_on(engine.compile("ActorValue", &COMPILER_OPTIONS));

    let actor_layout = engine.get_type("Actor").unwrap();
    let actor_value = engine.get_type("ActorValue").unwrap();

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
        objects,
        registration,
        aliases: AliasSet::default(),
        effects: Arc::default(),
        wait_on_effect: None,
        object_data: HashMap::new(),
        borrowed: Arc::new(SharedBorrowCell::new()),
        loader,
    }
}

#[derive(Clone, Debug)]
struct ScriptInstance {
    instance: Instance,
    layout: TypeId,
    object: ObjectRef,
    stacks: HashMap<String, Arc<parking_lot::Mutex<Stack>>>,
}

/// Maps [`ScriptObject`]s to game world [`Entity`].
#[derive(Clone, Debug, Default)]
pub struct ScriptEntities(pub BiMap<ScriptObjectId, Entity>);
