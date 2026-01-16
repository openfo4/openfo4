mod items;
pub mod leveled_list;
mod partial;
mod record;
mod terrain;

use std::collections::HashSet;
use std::sync::Arc;

use async_channel::Receiver;
use crossbeam::queue::SegQueue;
use game_common::components::Transform;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::Vec2;
use hashbrown::HashMap;
use hecs::World;
use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::record::RecordId;
use openfo4_esp::records::Cell;
use openfo4_esp::{FormId, RecordChildren, RecordData, RecordHeaderFlags, RecordKind};
use parking_lot::RwLock;
use tracing::instrument;

use crate::cell::CellId;
use crate::components::{NiTransform, Persistent, Root, WorldSpace};
use crate::events::{Event, UnloadCell};
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::level::partial::PartialEntityFlags;
use crate::state::level::record::SpawnEntity;
use crate::state::terrain::TerrainManager;

use super::esp::EspLoader;

const INTERIOR_CELL: CellId = CellId::new(0, 0);

/// A streamer is an entity that causes a region around it to be loaded (i.e. streamed in).
#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub struct Level {
    events: Arc<SegQueue<SpawnEntity>>,
    tx: async_channel::Sender<Request>,
    // Worldspace, Cell
    cells_loaded: HashSet<(RecordId, CellId)>,
    // Worldspace, Cell
    cells_to_unload_next_tick: HashSet<(RecordId, CellId)>,
    terrain_manager: TerrainManager,
}

impl Level {
    pub fn new(
        loader: Arc<EspLoader>,
        asset_loader: AssetLoader,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
    ) -> Self {
        let terrain_manager = TerrainManager::new(pool.clone(), shared.entities.clone());

        let (tx, rx) = async_channel::unbounded::<Request>();
        let events = Arc::new(SegQueue::new());
        let state = Arc::new(LoaderState::default());

        pool.spawn({
            let asset_loader = asset_loader.clone();
            let loader = loader.clone();
            let pool = pool.clone();
            let events = events.clone();
            let shared = shared.clone();
            let file_loader = file_loader.clone();
            let state = state.clone();

            async move {
                load_persistent(
                    &asset_loader,
                    &loader,
                    &events,
                    &pool,
                    &file_loader,
                    &shared,
                    &state,
                )
                .await;
            }
        });

        pool.spawn({
            let events = events.clone();
            let pool = pool.clone();

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
                )
                .await;
            }
        })
        .deatch();

        Self {
            events,
            tx,
            cells_loaded: HashSet::new(),
            cells_to_unload_next_tick: HashSet::new(),
            terrain_manager,
        }
    }

    pub fn update(
        &mut self,
        world: &mut World,
        events: &mut Vec<Event>,
        event_bus: &mut EventBus,
        object_ref_set: &mut ObjectReferenceSet,
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
                if !prev_cells_loaded.contains(&(worldspace.record, cell))
                    && !self.cells_loaded.contains(&(worldspace.record, cell))
                {
                    tracing::debug!("loading cell {:?}", cell);
                    self.tx
                        .try_send(Request {
                            worldspace: worldspace.record,
                            cell,
                        })
                        .unwrap();
                }

                self.cells_loaded.insert((worldspace.record, cell));
                prev_cells_loaded.remove(&(worldspace.record, cell));
            }
        }

        while let Some(entity) = self.events.pop() {
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
            if !self.cells_loaded.contains(&(worldspace.record, cell)) && !entity.persistent {
                continue;
            }

            entity.instantiate(world, event_bus, object_ref_set, &mut self.terrain_manager);
        }

        for cell in prev_cells_loaded {
            tracing::debug!("unloading cell {:?}", cell);

            // Mark the cell for removal in the next tick.
            // We cannot immediately unload the cell so that
            // entities can be notified that they are about to be unloaded.
            self.cells_to_unload_next_tick.insert(cell);
        }

        self.terrain_manager.update(world, event_bus);
    }

    #[instrument(name = "Level::unload_cells", skip_all)]
    fn unload_cells(&mut self, world: &mut World) {
        if self.cells_to_unload_next_tick.is_empty() {
            return;
        }

        let mut entities = Vec::new();

        for (entity, (transform, worldspace, _, persistent)) in
            world.query_mut::<(&Transform, &WorldSpace, &Root, Option<&Persistent>)>()
        {
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

            entities.push(entity);
        }

        for entity in entities {
            world.despawn_recursive(entity);
        }

        self.cells_to_unload_next_tick.clear();
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
    events: &Arc<SegQueue<SpawnEntity>>,
    pool: &Arc<TaskPool>,
    shared: &Arc<SharedState>,
    state: &Arc<LoaderState>,
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
        )
        .await;
    }
}

#[instrument(skip_all)]
async fn load_persistent(
    asset_loader: &AssetLoader,
    loader: &Arc<EspLoader>,
    events: &Arc<SegQueue<SpawnEntity>>,
    pool: &Arc<TaskPool>,
    file_loader: &Arc<FileLoader>,
    shared: &Arc<SharedState>,
    state: &Arc<LoaderState>,
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
        )
        .await;
    }
}

async fn load_cell(
    asset_loader: &AssetLoader,
    loader: &Arc<EspLoader>,
    events: &Arc<SegQueue<SpawnEntity>>,
    pool: &TaskPool,
    file_loader: &Arc<FileLoader>,
    shared: &Arc<SharedState>,
    cell_form: RecordId,
    worldspace: RecordId,
    persistent_only: bool,
    state: &Arc<LoaderState>,
    is_interior: bool,
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

    for record in records {
        let loader = loader.clone();
        let file_loader = file_loader.clone();
        let shared = shared.clone();
        let events = events.clone();
        let asset_loader = asset_loader.clone();
        let state = state.clone();

        pool.spawn(async move {
            let ctx = LoadContext {
                cell: cell_id,
                loader: &asset_loader,
                esp_loader: &loader,
                file_loader: &file_loader,
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

                events.push(entity);
            }
        });
    }
}

#[derive(Copy, Clone, Debug)]
struct LoadContext<'a> {
    cell: CellId,
    loader: &'a AssetLoader,
    esp_loader: &'a EspLoader,
    file_loader: &'a FileLoader,
    shared: &'a SharedState,
}

impl LoadContext<'_> {
    fn grid_offset(&self) -> Vec2 {
        self.cell.min()
    }
}
