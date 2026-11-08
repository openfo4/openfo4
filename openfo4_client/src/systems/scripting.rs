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

use async_io::Timer;
use futures::{FutureExt, select};
use futures_lite::future::block_on;
use hashbrown::HashSet;
use openfo4_common::collections::bimap::BiMap;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_pex::Module;
use openfo4_script::RegisterNativeError;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::compiler::resources::TypeId;
use openfo4_script::engine::{Engine, Instance, Options};
use openfo4_script::runtime::IntoFunc;
use openfo4_script::runtime::object::ObjectHandle;
use openfo4_tasks::TaskPool;
use openfo4_tracing::trace_span;
use parking_lot::Mutex;

use crate::components::{BaseObject, PlacedReference};
use crate::config;
use crate::events::combat::{BleedoutStartEvent, BodyPartCrippleEvent};
use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent};
use crate::events::trigger::{TriggerEnter, TriggerLeave};
use crate::loader::FileLoader;
use crate::state::esp::EspLoader;
use crate::state::level::loader::EntityLoader;
use crate::systems::DeathStartEvent;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::events::queue::EventQueueProducer;
use crate::systems::scripting::invocations::EventInvocation;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject, ScriptObjectId};
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

pub struct ExecutorBuilder {
    loader: Arc<ScriptLoader>,
    entities: Arc<ScriptEntities>,
    pool: Arc<TaskPool>,
}

impl ExecutorBuilder {
    pub async fn new(
        esp_loader: Arc<EspLoader>,
        file_loader: Arc<FileLoader>,
        pool: Arc<TaskPool>,
        config: config::Scripting,
    ) -> Self {
        let _span = trace_span!("ExecutorBuilder::new").entered();

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
                engine.register(&module).await;
            }
        }

        classes::register_builtins(&mut engine);

        let engine = Arc::new(engine);
        let objects = Arc::new(ScriptObjects::default());
        let registration = Arc::new(RegistrationSet::default());

        let loader = Arc::new(ScriptLoader::new(
            engine.clone(),
            esp_loader.clone(),
            objects.clone(),
            registration.clone(),
        ));

        Self {
            loader,
            pool,
            entities: Arc::default(),
        }
    }

    pub fn loader(&self) -> &Arc<ScriptLoader> {
        &self.loader
    }

    pub fn entities(&self) -> &Arc<ScriptEntities> {
        &self.entities
    }

    pub fn build(
        self,
        globals: Arc<GlobalProperties>,
        entity_loader: Arc<EntityLoader>,
    ) -> Executor {
        let state = core_actor_values(
            &self.loader.engine,
            self.loader.objects.clone(),
            self.loader.registration.clone(),
            self.loader.clone(),
            globals,
            self.loader.esp_loader.clone(),
            entity_loader,
            self.entities.clone(),
        );

        let (mut runner, handle) = Runner::new(state, self.pool.clone(), self.entities.clone());
        self.pool.spawn(async move { runner.run().await });

        Executor {
            loader: self.loader,
            handle,
            entities: self.entities,
        }
    }
}

pub struct Executor {
    loader: Arc<ScriptLoader>,
    handle: RunnerHandle,
    entities: Arc<ScriptEntities>,
}

impl Executor {
    pub fn loader(&self) -> &Arc<ScriptLoader> {
        &self.loader
    }

    pub fn entities(&self) -> &Arc<ScriptEntities> {
        &self.entities
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
        quests: &mut QuestManager,
        budget: Duration,
    ) {
        let _span = trace_span!("Executor::update").entered();

        self.handle.tick();

        self.handle.cell.set(BorrowedState {
            world: world,
            quests: quests,
            events,
        });

        block_on(async {
            let timer = Timer::after(budget);

            select! {
                _ = self.handle.done().fuse() => {}
                _ = timer.fuse() => {
                    // Once our budget runs out we will wait until at least one
                    // task has made progress.
                    // This prevents complete starvation that can happen when our
                    // budget is low.
                    self.handle.progress().await;
                }
            }
        });

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

    pub fn objects(&self) -> &ScriptObjects {
        &self.loader().objects
    }

    pub fn engine(&self) -> &Engine {
        &self.loader().engine
    }

    pub async fn get_or_load(&self, world: &World, entity: Entity) -> ScriptObjectId {
        if let Some(id) = self.entities.get(entity) {
            return id;
        }

        if let Ok(refr) = world.get::<&PlacedReference>(entity) {
            if let Some(object) = self.objects().get_by_record(refr.0) {
                return object.id();
            }

            let res = self.loader().load(refr.0).await.unwrap();
            self.entities().set(entity, res.object.id()).unwrap();
            return res.object.id();
        }

        if let Ok(base) = world.get::<&BaseObject>(entity) {
            let obj = self.loader().load_instance(base.record).await.unwrap();
            self.entities().set(entity, obj.id()).unwrap();
            return obj.id();
        }

        unreachable!("entity has no `BaseObject`")
    }
}

fn core_actor_values(
    engine: &Engine,
    objects: Arc<ScriptObjects>,
    registration: Arc<RegistrationSet>,
    loader: Arc<ScriptLoader>,
    globals: Arc<GlobalProperties>,
    esp_loader: Arc<EspLoader>,
    entity_loader: Arc<EntityLoader>,
    entities: Arc<ScriptEntities>,
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
    ] = std::array::from_fn(|_| engine.object_pool().insert(actor_value.id()));

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
        esp_loader,
        deletion: HashSet::new(),
        entity_loader,
        entities,
    }
}

#[derive(Clone, Debug)]
pub struct ScriptInstance {
    instance: Instance,
    pub layout: TypeId,
    pub object: ObjectHandle,
}

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum LoadRecordKind {
    /// The [`RecordId`] points directly to the record to load.
    Base(RecordId),
    /// The [`RecordId`] points to the base record of the wanted record.
    Instance(RecordId),
}

impl LoadRecordKind {
    fn extract(world: &World, entity: Entity) -> Self {
        match world.get::<&PlacedReference>(entity) {
            Ok(ref_) => Self::Base(ref_.0),
            Err(_) => match world.get::<&BaseObject>(entity) {
                Ok(base) => Self::Instance(base.record),
                Err(_) => {
                    unreachable!("entity has neither `PlacedReference` nor `BaseObject`")
                }
            },
        }
    }

    async fn load(&self, loader: &ScriptLoader) -> ScriptObject {
        match self {
            Self::Base(id) => loader.load(*id).await.unwrap().object,
            Self::Instance(id) => loader.load_instance(*id).await.unwrap(),
        }
    }
}

/// Maps [`ScriptObject`]s to game world [`Entity`].
#[derive(Debug, Default)]
pub struct ScriptEntities(Mutex<BiMap<ScriptObjectId, Entity>>);

impl ScriptEntities {
    pub fn get(&self, entity: Entity) -> Option<ScriptObjectId> {
        self.0.lock().get_right(&entity).copied()
    }

    pub fn set(&self, entity: Entity, id: ScriptObjectId) -> Result<(), ScriptObjectId> {
        let mut lock = self.0.lock();
        match lock.get_right(&entity) {
            Some(id) => Err(*id),
            None => {
                lock.insert(id, entity);
                Ok(())
            }
        }
    }

    pub fn resolve(&self, id: ScriptObjectId) -> Option<Entity> {
        self.0.lock().get_left(&id).copied()
    }
}
