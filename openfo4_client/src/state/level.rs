mod data_ext;
pub mod items;
pub mod leveled_list;
pub mod loader;
mod navmesh;
mod partial;
mod record;
mod terrain;

use std::collections::HashSet;
use std::sync::Arc;

use async_channel::Receiver;
use crossbeam::queue::SegQueue;
use event_listener::IntoNotification;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use game_common::components::Transform;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::{Vec2, Vec3};
use hashbrown::HashMap;
use openfo4_asset::AssetLoader;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::record::RecordId;
use openfo4_core::{Frozen, WorldExt};
use openfo4_esp::records::Cell;
use openfo4_esp::{RecordChildren, RecordData, RecordKind};
use parking_lot::RwLock;

use crate::cell::CellId;
use crate::components::{NiTransform, Root, WorldSpace};
use crate::config;
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::esp;
use crate::state::level::data_ext::RecordDataExt;
use crate::state::level::loader::{EntityLoader, EntityRefInner};
use crate::state::level::partial::{LodLevel, PartialEntityFlags};
use crate::state::level::record::SpawnEntity;
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
    tx: async_channel::Sender<Request>,
    cells_loaded: HashMap<CellKey, CellActivityMode>,
    terrain_manager: TerrainManager,
    loader: Arc<EntityLoader>,
    alive_entities: HashSet<Entity>,
    config: config::Streaming,
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

        let (tx, rx) = async_channel::unbounded::<Request>();
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
        });

        pool.spawn({
            let state = state.clone();
            async move {
                load_cells_task(rx, state).await;
            }
        })
        .deatch();

        Self {
            events: events.clone(),
            tx,
            cells_loaded: HashMap::new(),
            terrain_manager,
            alive_entities: HashSet::new(),
            loader: Arc::new(EntityLoader::new(state)),
            config,
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
        object_ref_set: &mut ObjectReferenceSet,
        script_entities: &ScriptEntities,
    ) {
        let _span = trace_span!("Level::update").entered();

        let radius = self.config.active.max(self.config.visible);

        let mut cells_to_unload = self.cells_loaded.keys().copied().collect::<HashSet<_>>();
        let mut cells_to_load = HashMap::new();

        for (transform, worldspace, _) in world.query_mut::<(&Transform, &WorldSpace, &Streamer)>()
        {
            let (center, cells) = if worldspace.is_interior {
                (INTERIOR_CELL, vec![INTERIOR_CELL])
            } else {
                let cell = CellId::from(NiTransform::from(*transform));
                (cell, cell.square(radius))
            };

            for cell in cells {
                let key = CellKey {
                    worldspace: worldspace.record,
                    cell,
                };

                cells_to_unload.remove(&key);

                let distance = cell.distance_max(center);
                let new_mode = if distance <= self.config.active {
                    CellActivityMode::Active
                } else if distance <= self.config.visible {
                    CellActivityMode::VisibleDistant
                } else {
                    todo!();
                };

                let entry = cells_to_load.entry(key).or_insert(new_mode);
                *entry = (*entry).max(new_mode);
            }
        }

        for cell in &cells_to_unload {
            self.cells_loaded.remove(cell);
        }

        for (cell, new_mode) in cells_to_load {
            if let Some(mode) = self.cells_loaded.get(&cell) {
                if *mode == new_mode {
                    continue;
                }

                // Reload the cell with a different mode.
                cells_to_unload.insert(cell);
            }

            tracing::debug!("loading cell {:?}", cell);

            self.cells_loaded.insert(cell, new_mode);
            self.tx
                .try_send(Request {
                    worldspace: cell.worldspace,
                    cell: cell.cell,
                    mode: new_mode,
                })
                .unwrap();
        }

        self.unload_cells(world, &cells_to_unload);
        self.instantiate_new_entities(world, event_bus, object_ref_set, script_entities);

        self.terrain_manager.update(world);
        self.update_frozen_entities(world);
    }

    fn instantiate_new_entities(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        object_ref_set: &mut ObjectReferenceSet,
        script_entities: &ScriptEntities,
    ) {
        let _span = trace_span!("Level::instantiate_new_entities").entered();

        while let Some(event) = self.events.pop() {
            match event {
                Event::SpawnEntity {
                    record,
                    entity,
                    make_ref,
                    signal,
                } => {
                    let worldspace = entity.worldspace.unwrap();
                    let cell = if worldspace.is_interior {
                        INTERIOR_CELL
                    } else {
                        CellId::from(entity.transform)
                    };

                    // It is possible for the event to arrive after the cell
                    // was already unloaded again.
                    // In this case we must not spawn the new entity to prevent
                    // duplicating entities when the cell gets loaded again.
                    if !self.cells_loaded.contains_key(&CellKey {
                        worldspace: worldspace.record,
                        cell,
                    }) && !make_ref
                    {
                        continue;
                    }

                    {
                        let mut entities = self.loader.entities.write();

                        // Since the EntityLoader and automatic cell loader are not synchronized
                        // with each other, a race condition is possible where both of them think
                        // the entity does not exist yet, and both start loading it at the same time.
                        // Since the process order is total, there are two possible cases:
                        // 1. The `EntityLoader` event comes before the cell loader event.
                        //    In this case `make_ref` is set on the first event and the cell loader
                        //    (second) event can be dropped.
                        // 2. The cell loader event comes before the `EntityLoader` event.
                        //    In this case `make_ref` is set of the second event and the `EntityLoader`
                        //    expects a ref-count of one to exist on the entity, i.e. we must create
                        //    the ref-count, but with the enity of the first event.
                        if let Some(record) = record
                            && let Some(entity) = entities.entities.get_left(&record).copied()
                        {
                            if !make_ref {
                                continue;
                            }

                            debug_assert!(!entities.ref_counts.contains_key(&entity));
                            entities
                                .ref_counts
                                .insert(entity, Arc::new(EntityRefInner::new_with_one_rc()));

                            continue;
                        }

                        // Entity creation cannot be undone, so this must happen in the critical section.
                        let entity = entity.instantiate(
                            world,
                            events,
                            object_ref_set,
                            &mut self.terrain_manager,
                            script_entities,
                        );

                        if let Some(record) = record {
                            entities.entities.insert(record, entity);
                        }

                        if make_ref {
                            entities
                                .ref_counts
                                .insert(entity, Arc::new(EntityRefInner::new_with_one_rc()));
                        }

                        if let Some(signal) = signal {
                            signal.notify(1.tag(entity));
                        }

                        self.alive_entities.insert(entity);
                    }
                }
                Event::CellLoadDone {
                    worldspace,
                    cell,
                    mode,
                } => {
                    if let Some(loaded) = self.cells_loaded.get_mut(&CellKey { worldspace, cell }) {
                        *loaded = mode;
                    }
                }
                Event::CreateRef(entity) => {
                    if self.alive_entities.contains(&entity) {
                        let mut entities = self.loader.entities.write();
                        entities
                            .ref_counts
                            .insert(entity, Arc::new(EntityRefInner::new_with_one_rc()));
                    }
                }
                Event::DropLastRefByRecord(id) => {
                    let mut entities = self.loader.entities.write();
                    let entity = entities.entities.remove_left(&id).unwrap();
                    entities.ref_counts.remove(&entity);
                }
                Event::DropLastRefByEntity(entity) => {
                    let mut entities = self.loader.entities.write();
                    entities.ref_counts.remove(&entity);
                }
            }
        }

        // Notify the EntityLoader that we're done touching entities for this
        // interation.
        // Note that it is valid for `total_listeners` to be outdated by the
        // time `notify` is called, however in this case some listeners may
        // have to wait for an additional tick, which is valid.
        self.loader.tick.notify(self.loader.tick.total_listeners());
    }

    fn unload_cells(&mut self, world: &mut World, cells: &HashSet<CellKey>) {
        let _span = trace_span!("Level::unload_cells").entered();

        if cells.is_empty() {
            return;
        }

        for cell in cells {
            tracing::debug!("unloading cell {:?}", cell);
        }

        let mut entities = Vec::new();

        let mut loader_entities = self.loader.entities.write();

        for (entity, transform, worldspace, _) in
            world.query_mut::<(Entity, &Transform, &WorldSpace, &Root)>()
        {
            self.alive_entities.insert(entity);

            let cell = if worldspace.is_interior {
                INTERIOR_CELL
            } else {
                CellId::from(NiTransform::from(*transform))
            };

            if !cells.contains(&CellKey {
                worldspace: worldspace.record,
                cell,
            }) {
                continue;
            }

            // Don't despawn entities that are explicitly marked as referenced
            // by other systems.
            if loader_entities.ref_counts.contains_key(&entity) {
                continue;
            }

            loader_entities.entities.remove_right(&entity);

            entities.push(entity);
            self.alive_entities.remove(&entity);
        }

        for entity in entities {
            world.despawn_recursive(entity);
        }
    }

    fn update_frozen_entities(&self, world: &mut World) {
        let _span = trace_span!("Level::update_frozen_entities").entered();

        let mut freeze = Vec::new();
        let mut unfreeze = Vec::new();

        for (entity, transform, worldspace, frozen, _) in world
            .query::<(Entity, &Transform, &WorldSpace, &Frozen, &Root)>()
            .iter()
        {
            let cell = if worldspace.is_interior {
                INTERIOR_CELL
            } else {
                CellId::from(NiTransform::from(*transform))
            };

            if self
                .cells_loaded
                .get(&CellKey {
                    worldspace: worldspace.record,
                    cell,
                })
                .is_some_and(|mode| *mode == CellActivityMode::Active)
            {
                // Unfreeze
                if !frozen.is_any_frozen() {
                    continue;
                }

                unfreeze.push(entity);
            } else {
                // Freeze
                if frozen.is_all_frozen() {
                    continue;
                }

                freeze.push(entity);
            }
        }

        while let Some(entity) = unfreeze.pop() {
            world.insert_one(entity, Frozen::default()).unwrap();

            if let Ok(children) = world.get::<&Children>(entity) {
                unfreeze.extend(&children.0);
            }
        }

        while let Some(entity) = freeze.pop() {
            world
                .insert_one(
                    entity,
                    Frozen {
                        physics: true,
                        controls: true,
                    },
                )
                .unwrap();

            if let Ok(children) = world.get::<&Children>(entity) {
                freeze.extend(&children.0);
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CellActivityMode {
    VisibleDistant,
    /// The cell is fully loaded.
    Active,
}

#[derive(Copy, Clone, Debug)]
struct Request {
    worldspace: RecordId,
    cell: CellId,
    mode: CellActivityMode,
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

async fn resolve_cell_to_record(
    esp_loader: &EspLoader,
    state: &LoaderState,
    req: Request,
) -> RecordId {
    // To resolve a CellId to a RecordId we need to load the worldspace record.
    // Loading the worldspace record for each cell is costly, we want to avoid that.
    // The first time we see a new worldspace we will build a CellId -> RecordId
    // cache that we can reuse.
    {
        let worldspaces = state.worldspaces.read();
        if let Some(ws) = worldspaces.get(&req.worldspace)
            && let Some(s) = ws.cells.get(&req.cell)
        {
            return s.record;
        }
    }

    let cells = esp_loader
        .children(RecordChildren::Cells, req.worldspace)
        .await
        .unwrap();

    let mut worldspaces = state.worldspaces.write();
    let worldspace = worldspaces.entry(req.worldspace).or_default();

    let mut searched_id = None;

    for record in cells {
        let RecordData::Cell(Cell {
            grid: Some(grid), ..
        }) = record.local.data
        else {
            continue;
        };

        let id = CellId::new(grid.x, grid.y);

        if id == req.cell {
            searched_id = Some(record.id);
        }

        // Don't overwrite cell state.
        if worldspace.cells.contains_key(&id) {
            continue;
        }

        worldspace.cells.insert(id, CellState { record: record.id });
    }

    searched_id.unwrap()
}

async fn load_cells_task(rx: Receiver<Request>, state: Arc<LoaderState>) {
    while let Ok(req) = rx.recv().await {
        let is_interior = state.persistent.interiors.contains(&req.worldspace);

        let cell = if !is_interior {
            resolve_cell_to_record(&state.esp_loader, &state, req).await
        } else {
            req.worldspace
        };

        let pool = state.pool.clone();
        let state = state.clone();
        pool.spawn(async move {
            load_cell(&state, cell, req.worldspace, is_interior, req.mode).await;

            for id in state
                .persistent
                .persistent
                .get(&CellKey {
                    worldspace: req.worldspace,
                    cell: req.cell,
                })
                .into_iter()
                .flatten()
            {
                let ctx = LoadContext::new(req.cell, &state);

                let record = state.esp_loader.lookup(None, *id).await.unwrap();

                let flags = match req.mode {
                    CellActivityMode::Active => PartialEntityFlags {
                        graphics: true,
                        physics: true,
                        actor: true,
                        scene: true,
                        script: true,
                        require_distant_lod: false,
                        lod: LodLevel::Normal,
                    },
                    CellActivityMode::VisibleDistant => PartialEntityFlags {
                        graphics: true,
                        scene: true,
                        physics: false,
                        actor: false,
                        script: false,
                        require_distant_lod: true,
                        lod: LodLevel::Distant0,
                    },
                };

                if let Some(mut entity) = SpawnEntity::load(&ctx, &record, flags).await {
                    entity.worldspace = Some(WorldSpace {
                        record: req.worldspace,
                        is_interior,
                    });
                    entity.form = Some(record.id);

                    state.events.push(Event::SpawnEntity {
                        record: Some(record.id),
                        entity,
                        make_ref: false,
                        signal: None,
                    });
                }
            }

            state.events.push(Event::CellLoadDone {
                worldspace: req.worldspace,
                cell: req.cell,
                mode: req.mode,
            });
        });
    }
}

async fn load_cell(
    state: &Arc<LoaderState>,
    cell_form: RecordId,
    worldspace: RecordId,
    is_interior: bool,
    mode: CellActivityMode,
) {
    let cell = match state
        .esp_loader
        .lookup(Some(RecordKind::Cell), cell_form)
        .await
    {
        Ok(record) => match record.local.data {
            RecordData::Cell(cell) => cell,
            _ => {
                tracing::error!("bad record type expected cell");
                return;
            }
        },
        Err(err) => {
            tracing::error!("invalid cell {:?}: {}", cell_form, err);
            return;
        }
    };

    let mut cell_id = CellId::new(0, 0);
    if let Some(grid) = cell.grid {
        cell_id = CellId::new(grid.x, grid.y);
    }

    let records = state
        .esp_loader
        .children(RecordChildren::TemporaryEntities, cell_form)
        .await
        .unwrap();

    let flags = match mode {
        CellActivityMode::Active => PartialEntityFlags {
            graphics: true,
            physics: true,
            actor: true,
            scene: true,
            script: true,
            require_distant_lod: false,
            lod: LodLevel::Normal,
        },
        CellActivityMode::VisibleDistant => PartialEntityFlags {
            graphics: true,
            scene: true,
            physics: false,
            actor: false,
            script: false,
            require_distant_lod: true,
            lod: LodLevel::Distant0,
        },
    };

    let mut tasks = FuturesUnordered::new();
    for record in records {
        // Skip loading these if we're loading distant objects,
        // they are either too inconsenquential or too expensive
        // to have loaded at long distance.
        if mode == CellActivityMode::VisibleDistant
            && matches!(
                record.local.data,
                RecordData::PlacedGrenade(_)
                    | RecordData::PlacedHazard(_)
                    | RecordData::PlacedMissile(_)
                    | RecordData::PlacedNpc(_)
            )
        {
            continue;
        }

        let pool = state.pool.clone();
        let state = state.clone();
        let task = pool.spawn(async move {
            let ctx = LoadContext::new(cell_id, &state);

            if let Some(mut entity) = SpawnEntity::load(&ctx, &record, flags).await {
                entity.worldspace = Some(WorldSpace {
                    record: worldspace,
                    is_interior,
                });
                entity.form = Some(record.id);

                state.events.push(Event::SpawnEntity {
                    record: Some(record.id),
                    entity,
                    make_ref: false,
                    signal: None,
                });
            }
        });

        tasks.push(task);
    }

    while let Some(()) = tasks.next().await {}
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
    SpawnEntity {
        record: Option<RecordId>,
        entity: SpawnEntity,
        // Make a ref count with value 1 in the EntitySet.
        make_ref: bool,
        signal: Option<event_listener::Event<Entity>>,
    },
    /// Cell is fully loaded.
    CellLoadDone {
        worldspace: RecordId,
        cell: CellId,
        mode: CellActivityMode,
    },
    /// Create a ref count for an existing entity.
    CreateRef(Entity),
    DropLastRefByRecord(RecordId),
    DropLastRefByEntity(Entity),
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
