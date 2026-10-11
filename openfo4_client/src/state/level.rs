mod data_ext;
pub mod items;
pub mod leveled_list;
pub mod loader;
mod navmesh;
pub mod partial;
mod record;
mod streaming;
mod terrain;

use std::collections::HashSet;
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use game_common::collections::slab::Slab;
use game_common::define_slab_key;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::{Vec2, Vec3};
use hashbrown::HashMap;
use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::record::RecordId;
use openfo4_esp::{RecordChildren, RecordKind};
use parking_lot::{Mutex, RwLock};

use crate::cell::CellId;
use crate::components::{NiTransform, WorldSpace};
use crate::config;
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::esp::{self};
use crate::state::level::data_ext::RecordDataExt;
use crate::state::level::loader::EntityLoader;
use crate::state::level::partial::{PartialEntityFlagCounts, PartialEntityFlags};
use crate::state::level::record::SpawnEntity;
use crate::state::level::streaming::StreamingManager;
use crate::state::terrain::TerrainManager;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::loader::ScriptLoader;

use super::esp::EspLoader;

const INTERIOR_CELL: CellId = CellId::new(0, 0);

/// A streamer is an entity that causes a region around it to be loaded (i.e. streamed in).
#[derive(Copy, Clone, Debug)]
pub struct Streamer;

pub struct Level {
    events: Arc<SegQueue<Event>>,
    terrain_manager: TerrainManager,
    loader: Arc<EntityLoader>,
    streaming: StreamingManager,
}

impl Level {
    pub fn new(
        loader: Arc<EspLoader>,
        asset_loader: AssetLoader,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
        script_loader: &Arc<ScriptLoader>,
        config: config::Streaming,
    ) -> Self {
        let terrain_manager = TerrainManager::new(shared.entities.clone());

        let events = Arc::new(SegQueue::new());

        let state = Arc::new(LoaderState {
            persistent: futures_lite::future::block_on(PersistentRecordStore::load(&loader))
                .unwrap(),
            worldspaces: RwLock::default(),
            esp_loader: loader.clone(),
            pool: pool.clone(),
            script_loader: script_loader.clone(),
            loader: asset_loader.clone(),
            events: events.clone(),
            file_loader: file_loader.clone(),
            shared: shared.clone(),
            entities: Mutex::default(),
        });

        let loader = Arc::new(EntityLoader::new(state.clone()));

        Self {
            events: events.clone(),
            terrain_manager,
            loader: loader.clone(),
            streaming: StreamingManager::new(state, config, loader),
        }
    }

    /// Returns the [`EntityLoader`] that can be used to selectively load entities.
    pub fn loader(&self) -> &Arc<EntityLoader> {
        &self.loader
    }

    pub fn update(
        &mut self,
        world: &mut World,
        event_bus: &mut EventBus,
        script_entities: &ScriptEntities,
    ) {
        let _span = trace_span!("Level::update").entered();

        self.streaming.update(world);

        self.sync_entities(world, event_bus, script_entities);

        self.terrain_manager.update(world);
        self.streaming.update_frozen_entities(world);
    }

    fn sync_entities(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        script_entities: &ScriptEntities,
    ) {
        let _span = trace_span!("Level::sync_entities").entered();

        let mut entities = self.loader.state.entities.lock();
        while let Some(event) = self.events.pop() {
            match event {
                Event::Spawn { id, entity, index } => {
                    let state = entities.entities.get_mut(id).unwrap();

                    let (flags, event) = state.loading.remove(index).unwrap();

                    let root = *state.entity.get_or_insert_with(|| world.reserve_entity());
                    // FIXME: We need to keep some data here, like script modified
                    // state or animation playback/times.
                    if let Ok(children) = world.remove_one::<Children>(root) {
                        for e in children.0 {
                            world.despawn_recursive(e);
                        }
                    }

                    entity.instantiate(
                        world,
                        events,
                        &mut self.terrain_manager,
                        script_entities,
                        root,
                    );

                    state.flags += flags;
                    event.notify(event.total_listeners());
                }
                Event::Despawn { id } => {
                    let state = entities.entities.remove(id).unwrap();
                    debug_assert_eq!(state.ref_count, 0);

                    if let Some(entity) = state.entity {
                        world.despawn_recursive(entity);
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct CellKey {
    /// The ID of the `WorldSpace` or standalone `Cell` record.
    worldspace: RecordId,
    cell: CellId,
}

#[derive(Debug)]
struct LoaderState {
    esp_loader: Arc<EspLoader>,
    loader: AssetLoader,
    script_loader: Arc<ScriptLoader>,
    pool: Arc<TaskPool>,
    events: Arc<SegQueue<Event>>,
    shared: Arc<SharedState>,
    file_loader: Arc<FileLoader>,
    worldspaces: RwLock<HashMap<RecordId, WorldspaceState>>,
    persistent: PersistentRecordStore,
    entities: Mutex<Entities>,
}

impl LoaderState {
    fn worldspace(&self, mut id: RecordId) -> Option<WorldSpace> {
        while let Some(parent) = self.esp_loader.parent(id) {
            if self.persistent.interiors.contains(&parent) {
                return Some(WorldSpace {
                    record: parent,
                    is_interior: true,
                });
            }

            if self.persistent.worldspaces.contains(&parent) {
                return Some(WorldSpace {
                    record: parent,
                    is_interior: false,
                });
            }

            id = parent;
        }

        None
    }
}

#[derive(Clone, Debug, Default)]
struct WorldspaceState {
    cells: HashMap<CellId, CellState>,
}

#[derive(Clone, Debug)]
struct CellState {
    /// ID of the cell record.
    record: RecordId,
}

#[derive(Copy, Clone, Debug)]
struct LoadContext<'a> {
    cell: CellId,
    loader: &'a AssetLoader,
    esp_loader: &'a EspLoader,
    file_loader: &'a FileLoader,
    script_loader: &'a ScriptLoader,
    shared: &'a SharedState,
}

impl<'a> LoadContext<'a> {
    fn new(cell: CellId, state: &'a LoaderState) -> Self {
        Self {
            cell,
            loader: &state.loader,
            esp_loader: &state.esp_loader,
            file_loader: &state.file_loader,
            script_loader: &state.script_loader,
            shared: &state.shared,
        }
    }

    fn grid_offset(&self) -> Vec2 {
        self.cell.min()
    }
}

#[derive(Debug)]
enum Event {
    Spawn {
        id: EntityId,
        entity: SpawnEntity,
        index: usize,
    },
    Despawn {
        id: EntityId,
    },
}

#[derive(Clone, Debug, Default)]
struct PersistentRecordStore {
    persistent: HashMap<CellKey, Vec<RecordId>>,
    /// Standalone Cell records without a parent Worldspace record.
    interiors: HashSet<RecordId>,
    worldspaces: HashSet<RecordId>,
}

impl PersistentRecordStore {
    pub async fn load(loader: &EspLoader) -> Result<Self, esp::Error> {
        let mut cells = Vec::new();
        let mut worldspace_cells = HashSet::new();

        let mut worldspaces = HashSet::new();
        for worldspace in loader.lookup_all(Some(RecordKind::Worldspace)).await? {
            for cell in loader
                .children(RecordChildren::Cells, worldspace.id)
                .await?
            {
                cells.push((worldspace.id, false, cell.id));
                worldspace_cells.insert(cell.id);
            }

            worldspaces.insert(worldspace.id);
        }

        let mut interiors = HashSet::new();
        for cell in loader.lookup_all(Some(RecordKind::Cell)).await? {
            // Ignore all cells that are children of a worldspace records,
            // those have already been queued.
            if worldspace_cells.contains(&cell.id) {
                continue;
            }

            cells.push((cell.id, true, cell.id));
            interiors.insert(cell.id);
        }

        let mut persistent: HashMap<CellKey, Vec<RecordId>> = HashMap::new();
        for (worldspace, is_interior, cell) in cells {
            for record in loader
                .children(RecordChildren::PersistentEntities, cell)
                .await?
            {
                let Some(position) = record.local.data.position() else {
                    continue;
                };
                let translation = Vec3::new(
                    position.position_x,
                    position.position_y,
                    position.position_z,
                );

                let cell = if is_interior {
                    INTERIOR_CELL
                } else {
                    CellId::from(NiTransform::from_translation(translation))
                };

                persistent
                    .entry(CellKey { worldspace, cell })
                    .or_default()
                    .push(record.id);
            }
        }

        Ok(Self {
            persistent,
            interiors,
            worldspaces,
        })
    }
}

define_slab_key! {
    /// A unique ID for a entity in the world, including placed object from ESP files and entities
    /// spawned at runtime.
    struct EntityId: usize;
}

#[derive(Debug, Default)]
struct Entities {
    entities: Slab<EntityId, EntityState>,
    records: HashMap<RecordId, EntityId>,
}

#[derive(Debug)]
struct EntityState {
    flags: PartialEntityFlagCounts,
    entity: Option<Entity>,
    ref_count: usize,
    loading: Slab<usize, (PartialEntityFlags, event_listener::Event)>,
    record: Option<RecordId>,
}
