pub mod invocations;
pub mod loader;

mod classes;
mod effects;
mod events;
mod object_builder;
mod object_data;
mod objects;
mod registration;
mod runner;
mod state;

use std::sync::Arc;
use std::time::Duration;

use futures_lite::future::block_on;
use game_common::collections::bimap::BiMap;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_pex::Module;
use openfo4_script::RegisterNativeError;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::compiler::resources::TypeId;
use openfo4_script::engine::{Engine, Instance, Options};
use openfo4_script::runtime::{IntoFunc, ObjectRef};

use crate::config;
use crate::events::combat::{BleedoutStartEvent, BodyPartCrippleEvent};
use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent};
use crate::events::trigger::{TriggerEnter, TriggerLeave};
use crate::loader::FileLoader;
use crate::state::esp::EspLoader;
use crate::systems::DeathStartEvent;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::events::queue::EventQueueProducer;
use crate::systems::scripting::invocations::EventInvocation;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObjectId};
use crate::systems::scripting::objects::ScriptObjects;
use crate::systems::scripting::registration::RegistrationSet;
use crate::systems::scripting::runner::{Runner, RunnerHandle};
use crate::systems::scripting::state::{
    AliasSet, BorrowedState, CoreObjects, SharedBorrowCell, State,
};

pub use runner::Invocation;

const COMPILER_OPTIONS: Options = Options {
    skip_native: true,
    skip_empty: true,
    use_native_stubs: false,
};

pub struct Executor {
    loader: Arc<ScriptLoader>,
    handle: RunnerHandle,
}

impl Executor {
    pub async fn new(
        esp_loader: Arc<EspLoader>,
        file_loader: Arc<FileLoader>,
        pool: Arc<TaskPool>,
        config: config::Scripting,
        globals: Arc<GlobalProperties>,
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

        let state = core_actor_values(&engine, objects, registration, loader.clone(), globals);

        let mut runner = Runner::new(state, pool.clone());
        let handle = runner.handle();
        pool.spawn(async move { runner.run().await });

        Self { loader, handle }
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
        events: &mut EventBus,
        script_entities: &mut ScriptEntities,
        quests: &mut QuestManager,
    ) {
        let _span = trace_span!("Executor::update").entered();

        self.handle.tick();

        self.handle.cell.set(BorrowedState {
            world: world,
            quests: quests,
            entities: script_entities,
            events,
        });

        std::thread::sleep(Duration::from_millis(1));

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

fn core_actor_values(
    engine: &Engine,
    objects: Arc<ScriptObjects>,
    registration: Arc<RegistrationSet>,
    loader: Arc<ScriptLoader>,
    globals: Arc<GlobalProperties>,
) -> State {
    block_on(engine.compile("Actor", &COMPILER_OPTIONS)).unwrap();
    block_on(engine.compile("ActorValue", &COMPILER_OPTIONS)).unwrap();

    engine.get_type("Actor").unwrap();
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
        borrowed: Arc::new(SharedBorrowCell::new()),
        loader,
        globals,
        wait_for_effects: Arc::default(),
    }
}

#[derive(Clone, Debug)]
struct ScriptInstance {
    instance: Instance,
    layout: TypeId,
    object: ObjectRef,
}

/// Maps [`ScriptObject`]s to game world [`Entity`].
#[derive(Clone, Debug, Default)]
pub struct ScriptEntities(pub BiMap<ScriptObjectId, Entity>);

trait EngineExt {
    fn register_if_exists<F, T, Marker, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Marker, Params, Returns>;
}

impl EngineExt for Engine {
    fn register_if_exists<F, T, Marker, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Marker, Params, Returns>,
    {
        match self.register_native_function(class, func, f) {
            Ok(()) => {}
            Err(RegisterNativeError::UnknownFunction(_)) => {
                tracing::debug!("not registering {}.{}: not found", class, func);
            }
            Err(RegisterNativeError::IncompatibleSignature { expected, found }) => {
                tracing::warn!(
                    "not registering {}.{}: incompatible signature: {found:?}, expected {expected:?}",
                    class,
                    func,
                );
            }
        }
    }
}
