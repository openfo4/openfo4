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
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use game_common::components::Transform;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::Vec2;
use hashbrown::HashMap;
use hecs::{CommandBuffer, Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::record::RecordId;
use openfo4_core::{Frozen, WorldExt};
use openfo4_esp::records::Cell;
use openfo4_esp::{RecordChildren, RecordData, RecordHeaderFlags, RecordKind};
use parking_lot::RwLock;
use tracing::instrument;

use crate::cell::CellId;
use crate::components::{NiTransform, Persistent, Root, WorldSpace};
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::level::loader::{EntityLoader, EntityRefInner};
use crate::state::level::partial::PartialEntityFlags;
use crate::state::level::record::SpawnEntity;
use crate::state::terrain::TerrainManager;
use crate::systems::scripting::loader::ScriptLoader;
use crate::systems::scripting::{Executor, ScriptEntities};

use super::esp::EspLoader;

const INTERIOR_CELL: CellId = CellId::new(0, 0);

/// A streamer is an entity that causes a region around it to be loaded (i.e. streamed in).
#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub struct Level {
    events: Arc<SegQueue<Event>>,
    tx: async_channel::Sender<Request>,
    // Worldspace, Cell, fully loaded
    cells_loaded: HashMap<(RecordId, CellId), bool>,
    // Worldspace, Cell
    cells_to_unload_next_tick: HashSet<(RecordId, CellId)>,
    terrain_manager: TerrainManager,
    loader: Arc<EntityLoader>,
    alive_entities: HashSet<Entity>,
}

impl Level {
    pub fn new(
        loader: Arc<EspLoader>,
        asset_loader: AssetLoader,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
        executor: &mut Executor,
    ) -> Self {
        let terrain_manager = TerrainManager::new(pool.clone(), shared.entities.clone());

        let (tx, rx) = async_channel::unbounded::<Request>();
        let events = Arc::new(SegQueue::new());
        let state = Arc::new(LoaderState::default());

        // pool.spawn({
        //     let asset_loader = asset_loader.clone();
        //     let loader = loader.clone();
        //     let pool = pool.clone();
        //     let events = events.clone();
        //     let shared = shared.clone();
        //     let file_loader = file_loader.clone();
        //     let state = state.clone();
        //     let script_loader = executor.loader().clone();

        //     async move {
        //         load_persistent(
        //             &asset_loader,
        //             &loader,
        //             &events,
        //             &pool,
        //             &file_loader,
        //             &shared,
        //             &state,
        //             &script_loader,
        //         )
        //         .await;
        //     }
        // });

        pool.spawn({
            let events = events.clone();
            let pool = pool.clone();
            let loader = loader.clone();
            let file_loader = file_loader.clone();
            let shared = shared.clone();
            let asset_loader = asset_loader.clone();
            let script_loader = executor.loader().clone();

            async move {
                load_cells_task(
                    rx,
                    &loader,
                    &asset_loader,
                    &file_loader,
                    &events,
                    &pool,
                    &shared,
                    &state,
                    &script_loader,
                )
                .await;
            }
        })
        .deatch();

        Self {
            events: events.clone(),
            tx,
            cells_loaded: HashMap::new(),
            cells_to_unload_next_tick: HashSet::new(),
            terrain_manager,
            alive_entities: HashSet::new(),
            loader: Arc::new(EntityLoader::new(
                loader,
                file_loader,
                shared,
                asset_loader,
                executor.loader().clone(),
                events,
            )),
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
        script_entities: &mut ScriptEntities,
    ) {
        let _span = trace_span!("Level::update").entered();

        self.unload_cells(world);

        let mut prev_cells_loaded = core::mem::take(&mut self.cells_loaded);
        for (_, (transform, streamer, worldspace)) in
            world.query_mut::<(&Transform, &Streamer, &WorldSpace)>()
        {
            let cells = if worldspace.is_interior {
                vec![INTERIOR_CELL]
            } else {
                let cell = CellId::from(NiTransform::from(*transform));
                cell.square(streamer.distance)
            };

            for cell in cells {
                // Start streaming in the cell if it was not already
                // loaded in the previous tick and we have not already started
                // loading it this tick.
                // This should happen at most once in the current tick.
                if !prev_cells_loaded.contains_key(&(worldspace.record, cell))
                    && !self.cells_loaded.contains_key(&(worldspace.record, cell))
                {
                    tracing::debug!("loading cell {:?}", cell);
                    self.tx
                        .try_send(Request {
                            worldspace: worldspace.record,
                            cell,
                        })
                        .unwrap();
                }

                let is_fully_loaded = prev_cells_loaded
                    .get(&(worldspace.record, cell))
                    .is_some_and(|v| *v);

                self.cells_loaded
                    .insert((worldspace.record, cell), is_fully_loaded);
                prev_cells_loaded.remove(&(worldspace.record, cell));
            }
        }

        while let Some(event) = self.events.pop() {
            match event {
                Event::SpawnEntity {
                    record,
                    entity,
                    make_ref,
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
                    if !self.cells_loaded.contains_key(&(worldspace.record, cell))
                        && !entity.persistent
                        && !make_ref
                    {
                        continue;
                    }

                    let entity = entity.instantiate(
                        world,
                        event_bus,
                        object_ref_set,
                        &mut self.terrain_manager,
                        script_entities,
                    );

                    let mut entities = self.loader.entities.write();
                    entities.entities.insert(record, entity);
                    if make_ref {
                        entities
                            .ref_counts
                            .insert(entity, Arc::new(EntityRefInner::new_with_one_rc()));
                    }

                    self.alive_entities.insert(entity);
                }
                Event::CellLoadDone { worldspace, cell } => {
                    if let Some(loaded) = self.cells_loaded.get_mut(&(worldspace, cell)) {
                        *loaded = true;
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
                    let entity = entities.entities.remove(&id).unwrap();
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

        for cell in prev_cells_loaded.into_keys() {
            tracing::debug!("unloading cell {:?}", cell);

            // Mark the cell for removal in the next tick.
            // We cannot immediately unload the cell so that
            // entities can be notified that they are about to be unloaded.
            self.cells_to_unload_next_tick.insert(cell);
        }

        self.terrain_manager.update(world, event_bus);
        self.update_frozen_entities(world);
    }

    #[instrument(name = "Level::unload_cells", skip_all)]
    fn unload_cells(&mut self, world: &mut World) {
        if self.cells_to_unload_next_tick.is_empty() {
            return;
        }

        let mut entities = Vec::new();

        let loader_entities = self.loader.entities.read();

        for (entity, (transform, worldspace, _, persistent)) in
            world.query_mut::<(&Transform, &WorldSpace, &Root, Option<&Persistent>)>()
        {
            self.alive_entities.insert(entity);

            if persistent.is_some() {
                continue;
            }

            let cell = if worldspace.is_interior {
                INTERIOR_CELL
            } else {
                CellId::from(NiTransform::from(*transform))
            };

            if !self
                .cells_to_unload_next_tick
                .contains(&(worldspace.record, cell))
            {
                continue;
            }

            // Don't despawn entities that are explicitly marked as referenced
            // by other systems.
            if loader_entities.ref_counts.contains_key(&entity) {
                continue;
            }

            entities.push(entity);
            self.alive_entities.remove(&entity);
        }

        for entity in entities {
            world.despawn_recursive(entity);
        }

        self.cells_to_unload_next_tick.clear();
    }

    fn update_frozen_entities(&self, world: &mut World) {
        let _span = trace_span!("Level::update_frozen_entities").entered();

        let mut commands = CommandBuffer::new();
        for (entity, (transform, worldspace)) in world.query::<(&Transform, &WorldSpace)>().iter() {
            let cell = CellId::from(NiTransform::from(*transform));

            if self
                .cells_loaded
                .get(&(worldspace.record, cell))
                .is_some_and(|v| *v)
            {
                // Unfreeze
                commands.insert_one(entity, Frozen::default());
            } else {
                // Freeze
                commands.insert_one(
                    entity,
                    Frozen {
                        physics: true,
                        controls: true,
                    },
                );
            }
        }

        commands.run_on(world);
    }
}

#[derive(Copy, Clone, Debug)]
struct Request {
    worldspace: RecordId,
    cell: CellId,
}

#[derive(Debug, Default)]
struct LoaderState {
    worldspaces: RwLock<HashMap<RecordId, WorldspaceState>>,
    interiors: RwLock<HashSet<RecordId>>,
}

#[derive(Clone, Debug, Default)]
struct WorldspaceState {
    cells: HashMap<CellId, CellState>,
}

#[derive(Clone, Debug, Default)]
struct CellState {
    persistent: Vec<RecordId>,
}

async fn load_cells_task(
    rx: Receiver<Request>,
    esp_loader: &Arc<EspLoader>,
    loader: &AssetLoader,
    file_loader: &Arc<FileLoader>,
    events: &Arc<SegQueue<Event>>,
    pool: &Arc<TaskPool>,
    shared: &Arc<SharedState>,
    state: &Arc<LoaderState>,
    script_loader: &Arc<ScriptLoader>,
) {
    while let Ok(req) = rx.recv().await {
        let is_interior = state.interiors.read().contains(&req.worldspace);

        let cell = if !is_interior {
            let cells = esp_loader
                .children(RecordChildren::Cells, req.worldspace)
                .await
                .unwrap();

            cells
                .iter()
                .find_map(|record| match &record.local.data {
                    RecordData::Cell(Cell {
                        grid: Some(grid), ..
                    }) if req.cell.0.x == grid.x && req.cell.0.y == grid.y => Some(record.id),
                    _ => None,
                })
                .unwrap()
        } else {
            req.worldspace
        };

        load_cell(
            loader,
            esp_loader,
            events,
            pool,
            file_loader,
            shared,
            cell,
            req.worldspace,
            false,
            state,
            is_interior,
            script_loader,
        )
        .await;
    }
}

#[instrument(skip_all)]
async fn load_persistent(
    asset_loader: &AssetLoader,
    loader: &Arc<EspLoader>,
    events: &Arc<SegQueue<Event>>,
    pool: &Arc<TaskPool>,
    file_loader: &Arc<FileLoader>,
    shared: &Arc<SharedState>,
    state: &Arc<LoaderState>,
    script_loader: &Arc<ScriptLoader>,
) {
    let mut worldspace_cells = HashSet::new();

    for worldspace in loader
        .lookup_all(Some(RecordKind::Worldspace))
        .await
        .unwrap()
    {
        for cell in loader
            .children(RecordChildren::Cells, worldspace.id)
            .await
            .unwrap()
        {
            if !cell.local.flags.contains(RecordHeaderFlags::PERSISTENT) {
                continue;
            }

            worldspace_cells.insert(cell.id);

            load_cell(
                asset_loader,
                loader,
                events,
                pool,
                file_loader,
                shared,
                cell.id,
                worldspace.id,
                true,
                state,
                false,
                script_loader,
            )
            .await;
        }
    }

    for cell in loader.lookup_all(Some(RecordKind::Cell)).await.unwrap() {
        if worldspace_cells.contains(&cell.id) {
            continue;
        }

        state.interiors.write().insert(cell.id);

        load_cell(
            asset_loader,
            loader,
            events,
            pool,
            file_loader,
            shared,
            cell.id,
            cell.id,
            true,
            state,
            true,
            script_loader,
        )
        .await;
    }
}

async fn load_cell(
    asset_loader: &AssetLoader,
    loader: &Arc<EspLoader>,
    events: &Arc<SegQueue<Event>>,
    pool: &TaskPool,
    file_loader: &Arc<FileLoader>,
    shared: &Arc<SharedState>,
    cell_form: RecordId,
    worldspace: RecordId,
    persistent_only: bool,
    state: &Arc<LoaderState>,
    is_interior: bool,
    script_loader: &Arc<ScriptLoader>,
) {
    let cell = match loader.lookup(Some(RecordKind::Cell), cell_form).await {
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

    let records = if persistent_only {
        let persistent = loader
            .children(RecordChildren::PersistentEntities, cell_form)
            .await
            .unwrap();

        persistent
    } else {
        let persistent_forms = {
            let worldspaces = state.worldspaces.read();
            worldspaces
                .get(&worldspace)
                .and_then(|wp| wp.cells.get(&cell_id))
                .map(|cell| cell.persistent.to_vec())
                .unwrap_or_default()
        };

        let mut persistent = Vec::new();
        for form in persistent_forms {
            persistent.push(loader.lookup(None, form).await.unwrap());
        }

        let temporary = loader
            .children(RecordChildren::TemporaryEntities, cell_form)
            .await
            .unwrap();

        persistent.extend(temporary);
        persistent
    };

    let mut tasks = FuturesUnordered::new();
    for record in records {
        let loader = loader.clone();
        let file_loader = file_loader.clone();
        let shared = shared.clone();
        let events = events.clone();
        let asset_loader = asset_loader.clone();
        let state = state.clone();
        let script_loader = script_loader.clone();

        let task = pool.spawn(async move {
            let ctx = LoadContext {
                cell: cell_id,
                loader: &asset_loader,
                esp_loader: &loader,
                file_loader: &file_loader,
                script_loader: &script_loader,
                shared: &shared,
            };

            let flags = if persistent_only {
                PartialEntityFlags::empty()
            } else {
                PartialEntityFlags::all()
            };

            if let Some(mut entity) = SpawnEntity::load(&ctx, &record, flags).await {
                entity.persistent |= record.local.flags.contains(RecordHeaderFlags::PERSISTENT);
                entity.worldspace = Some(WorldSpace {
                    record: worldspace,
                    is_interior,
                });
                entity.form = Some(record.id);

                if persistent_only && entity.persistent {
                    let cell = if is_interior {
                        CellId::new(0, 0)
                    } else {
                        CellId::from(entity.transform)
                    };

                    let mut worldspaces = state.worldspaces.write();
                    worldspaces
                        .entry(worldspace)
                        .or_default()
                        .cells
                        .entry(cell)
                        .or_default()
                        .persistent
                        .push(record.id);
                }

                events.push(Event::SpawnEntity {
                    record: record.id,
                    entity,
                    make_ref: false,
                });
            }
        });

        tasks.push(task);
    }

    while let Some(()) = tasks.next().await {}
    events.push(Event::CellLoadDone {
        worldspace,
        cell: cell_id,
    });
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

impl LoadContext<'_> {
    fn grid_offset(&self) -> Vec2 {
        self.cell.min()
    }
}

#[derive(Debug)]
enum Event {
    SpawnEntity {
        record: RecordId,
        entity: SpawnEntity,
        // Make a ref count with value 1 in the EntitySet.
        make_ref: bool,
    },
    /// Cell is fully loaded.
    CellLoadDone {
        worldspace: RecordId,
        cell: CellId,
    },
    /// Create a ref count for an existing entity.
    CreateRef(Entity),
    DropLastRefByRecord(RecordId),
    DropLastRefByEntity(Entity),
}
