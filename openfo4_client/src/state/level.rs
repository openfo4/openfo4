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
use hecs::World;
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_esp::records::Cell;
use openfo4_esp::{FormId, RecordChildren, RecordData, RecordHeaderFlags, RecordKind};
use tracing::instrument;

use crate::cell::CellId;
use crate::components::{NiTransform, Persistent, Root, WorldSpace};
use crate::events::{Event, UnloadCell};
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::level::partial::PartialEntityFlags;
use crate::state::level::record::SpawnEntity;

use super::esp::EspLoader;

/// A streamer is an entity that causes a region around it to be loaded (i.e. streamed in).
#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub struct Level {
    events: Arc<SegQueue<SpawnEntity>>,
    tx: async_channel::Sender<Request>,
    cells_loaded: HashSet<CellId>,
    cells_to_unload_next_tick: HashSet<CellId>,
}

impl Level {
    pub fn new(
        loader: Arc<EspLoader>,
        asset_loader: AssetLoader,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
    ) -> Self {
        let (tx, rx) = async_channel::unbounded::<Request>();
        let events = Arc::new(SegQueue::new());

        pool.spawn({
            let asset_loader = asset_loader.clone();
            let loader = loader.clone();
            let pool = pool.clone();
            let events = events.clone();
            let shared = shared.clone();
            let file_loader = file_loader.clone();

            async move {
                load_persistent(
                    &asset_loader,
                    &loader,
                    &events,
                    &pool,
                    &file_loader,
                    &shared,
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
        for (_, (transform, streamer)) in world.query_mut::<(&Transform, &Streamer)>() {
            let ni_transform = NiTransform::from(*transform);
            let cell = CellId::from(ni_transform);
            let cells = cell.square(streamer.distance);

            for cell in cells {
                // Start streaming in the cell if it was not already
                // loaded in the previous tick and we have not already started
                // loading it this tick.
                // This should happen at most once in the current tick.
                if !prev_cells_loaded.contains(&cell) && !self.cells_loaded.contains(&cell) {
                    tracing::debug!("loading cell {:?}", cell);
                    self.tx
                        .try_send(Request::Exterior {
                            worldspace: FormId(0x3c),
                            cell,
                        })
                        .unwrap();
                }

                self.cells_loaded.insert(cell);
                prev_cells_loaded.remove(&cell);
            }
        }

        while let Some(entity) = self.events.pop() {
            // It is possible for the event to arrive after the cell
            // was already unloaded again.
            // In this case we must not spawn the new entity to prevent
            // duplicating entities when the cell gets loaded again.
            let cell = CellId::from(entity.transform);
            if !self.cells_loaded.contains(&cell) && !entity.persistent {
                continue;
            }

            entity.instantiate(world, event_bus, object_ref_set);
        }

        for cell in prev_cells_loaded {
            tracing::debug!("unloading cell {:?}", cell);

            events.push(Event::UnloadCell(UnloadCell { cell }));

            // Mark the cell for removal in the next tick.
            // We cannot immediately unload the cell so that
            // entities can be notified that they are about to be unloaded.
            self.cells_to_unload_next_tick.insert(cell);
        }
    }

    #[instrument(name = "Level::unload_cells", skip_all)]
    fn unload_cells(&mut self, world: &mut World) {
        if self.cells_to_unload_next_tick.is_empty() {
            return;
        }

        let mut entities = Vec::new();
        let mut queue = Vec::new();

        for (entity, (cell, children, _, persistent)) in
            world.query_mut::<(&CellId, Option<&Children>, &Root, Option<&Persistent>)>()
        {
            if persistent.is_some() {
                continue;
            }

            if !self.cells_to_unload_next_tick.contains(cell) {
                continue;
            }

            entities.push(entity);

            if let Some(children) = children {
                for children in &children.0 {
                    queue.push(*children);
                }
            }
        }

        while let Some(entity) = queue.pop() {
            entities.push(entity);

            if let Ok(children) = world.get::<&Children>(entity) {
                for children in &children.0 {
                    queue.push(*children);
                }
            }
        }

        for entity in entities {
            world.despawn(entity).unwrap();
        }

        self.cells_to_unload_next_tick.clear();
    }
}

#[derive(Copy, Clone, Debug)]
enum Request {
    /// An exterior cell.
    Exterior { worldspace: FormId, cell: CellId },
    /// An interior cell.
    Interior { cell: FormId },
}

async fn load_cells_task(
    rx: Receiver<Request>,
    esp_loader: &Arc<EspLoader>,
    loader: &AssetLoader,
    file_loader: &Arc<FileLoader>,
    events: &Arc<SegQueue<SpawnEntity>>,
    pool: &Arc<TaskPool>,
    shared: &Arc<SharedState>,
) {
    while let Ok(req) = rx.recv().await {
        let cell = match req {
            Request::Exterior { worldspace, cell } => {
                let cells = esp_loader
                    .children(RecordChildren::Cells, worldspace)
                    .await
                    .unwrap();

                cells
                    .iter()
                    .find_map(|record| match &record.data {
                        RecordData::Cell(Cell {
                            grid: Some(grid), ..
                        }) if cell.0.x == grid.x && cell.0.y == grid.y => Some(record.id),
                        _ => None,
                    })
                    .unwrap()
            }
            Request::Interior { cell } => cell,
        };

        let worldspace = match req {
            Request::Exterior {
                worldspace,
                cell: _,
            } => worldspace,
            Request::Interior { cell } => cell,
        };

        dbg!(worldspace);

        load_cell(
            loader,
            esp_loader,
            events,
            pool,
            file_loader,
            shared,
            cell,
            worldspace,
            false,
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
            if !cell.flags.contains(RecordHeaderFlags::PERSISTENT) {
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
            )
            .await;
        }
    }

    for cell in loader.lookup_all(Some(RecordKind::Cell)).await.unwrap() {
        if worldspace_cells.contains(&cell.id) {
            continue;
        }

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
    cell_form: FormId,
    worldspace: FormId,
    persistent_only: bool,
) {
    let cell = match loader.lookup(Some(RecordKind::Cell), cell_form).await {
        Ok(record) => match record.data {
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

    let persistent = loader
        .children(RecordChildren::PersistentEntities, cell_form)
        .await
        .unwrap();

    let temporary = if !persistent_only {
        loader
            .children(RecordChildren::TemporaryEntities, cell_form)
            .await
            .unwrap()
    } else {
        Vec::new()
    };

    for record in persistent.into_iter().chain(temporary) {
        let loader = loader.clone();
        let file_loader = file_loader.clone();
        let shared = shared.clone();
        let events = events.clone();
        let asset_loader = asset_loader.clone();

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
                entity.persistent = record.flags.contains(RecordHeaderFlags::PERSISTENT);
                entity.worldspace = Some(WorldSpace(worldspace));
                // entity.form = Some(record.id);
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
