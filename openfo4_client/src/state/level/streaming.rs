//! Handler for automatic cell streaming.

use std::sync::Arc;

use async_channel::{Receiver, Sender};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use hashbrown::{HashMap, HashSet};
use openfo4_common::components::Transform;
use openfo4_core::Frozen;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::hierarchy::Children;
use openfo4_core::record::RecordId;
use openfo4_esp::records::Cell;
use openfo4_esp::{RecordChildren, RecordData, RecordKind};
use openfo4_tracing::trace_span;
use parking_lot::Mutex;

use crate::cell::CellId;
use crate::components::{ActiveMarker, NiTransform, Root, WorldSpace};
use crate::config;
use crate::state::level::loader::{CustomRecordInfo, EntityLoader, EntityRef};
use crate::state::level::partial::{LodLevel, PartialEntityFlags};
use crate::state::level::{CellKey, CellState, INTERIOR_CELL, LoaderState, Streamer};

#[derive(Debug)]
pub struct StreamingManager {
    tx: Sender<Request>,
    config: config::Streaming,
    state: Arc<Mutex<StreamingState>>,
}

impl StreamingManager {
    pub fn new(
        state: Arc<LoaderState>,
        config: config::Streaming,
        loader: Arc<EntityLoader>,
    ) -> Self {
        let (tx, rx) = async_channel::unbounded();

        let streaming: Arc<Mutex<StreamingState>> = Arc::default();

        let pool = state.pool.clone();
        pool.spawn(load_cells_task(rx, state, loader, streaming.clone()))
            .deatch();

        Self {
            tx,
            config,
            state: streaming,
        }
    }

    pub fn update(&mut self, world: &mut World) {
        let _span = trace_span!("StreamingManager::update").entered();

        let radius = self.config.active.max(self.config.visible);

        let mut state = self.state.lock();

        let mut cells_to_unload = state.cells_loaded.keys().copied().collect::<HashSet<_>>();
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
            state.cells_loaded.remove(cell);
            state.active_cells.remove(cell);
        }

        for (cell, new_mode) in cells_to_load {
            if new_mode != CellActivityMode::Active {
                state.active_cells.remove(&cell);
            }

            if let Some(state) = state.cells_loaded.get(&cell) {
                if state.mode == new_mode {
                    continue;
                }

                // Reload the cell with a different mode.
                cells_to_unload.insert(cell);
            }

            tracing::debug!("loading cell {:?}", cell);

            state.cells_loaded.insert(
                cell,
                LoadedCell {
                    mode: new_mode,
                    entities: Vec::new(),
                },
            );
            self.tx
                .try_send(Request {
                    worldspace: cell.worldspace,
                    cell: cell.cell,
                    mode: new_mode,
                })
                .unwrap();
        }
    }

    pub fn update_frozen_entities(&self, world: &mut World) {
        let _span = trace_span!("StreamingManager::update_frozen_entities").entered();

        let state = self.state.lock();

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

            if state.active_cells.contains(&CellKey {
                worldspace: worldspace.record,
                cell,
            }) {
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
            world
                .insert(entity, (Frozen::default(), ActiveMarker))
                .unwrap();

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
            world.remove_one::<ActiveMarker>(entity).ok();

            if let Ok(children) = world.get::<&Children>(entity) {
                freeze.extend(&children.0);
            }
        }
    }
}

#[derive(Debug, Default)]
struct StreamingState {
    cells_loaded: HashMap<CellKey, LoadedCell>,
    /// Cells that are active and have all entities fully instantiated.
    active_cells: HashSet<CellKey>,
}

async fn load_cells_task(
    rx: Receiver<Request>,
    state: Arc<LoaderState>,
    loader: Arc<EntityLoader>,
    streaming: Arc<Mutex<StreamingState>>,
) {
    while let Ok(req) = rx.recv().await {
        let is_interior = state.persistent.interiors.contains(&req.worldspace);

        let cell = if !is_interior {
            resolve_cell_to_record(&state, req).await
        } else {
            req.worldspace
        };

        let pool = state.pool.clone();
        let state = state.clone();
        let loader = loader.clone();
        let streaming = streaming.clone();
        pool.spawn(async move {
            let entities =
                load_cell(&state, cell, req.worldspace, is_interior, req.mode, loader).await;

            let key = CellKey {
                worldspace: req.worldspace,
                cell: req.cell,
            };

            let mut streaming = streaming.lock();

            if let Some(state) = streaming.cells_loaded.get_mut(&key)
                && state.mode == req.mode
            {
                state.entities = entities;
                streaming.active_cells.insert(key);
            }
        });
    }
}

async fn load_cell(
    state: &Arc<LoaderState>,
    cell_form: RecordId,
    worldspace: RecordId,
    is_interior: bool,
    mode: CellActivityMode,
    loader: Arc<EntityLoader>,
) -> Vec<EntityRef> {
    let cell = match state
        .esp_loader
        .lookup(Some(RecordKind::Cell), cell_form)
        .await
    {
        Ok(record) => match record.local.data {
            RecordData::Cell(cell) => cell,
            _ => {
                tracing::error!("bad record type expected cell");
                return Vec::new();
            }
        },
        Err(err) => {
            tracing::error!("invalid cell {:?}: {}", cell_form, err);
            return Vec::new();
        }
    };

    let mut cell_id = CellId::new(0, 0);
    if let Some(grid) = cell.grid {
        cell_id = CellId::new(grid.x, grid.y);
    }

    let mut records = state
        .esp_loader
        .children(RecordChildren::TemporaryEntities, cell_form)
        .await
        .unwrap();

    for id in state
        .persistent
        .persistent
        .get(&CellKey {
            worldspace: worldspace,
            cell: cell_id,
        })
        .into_iter()
        .flatten()
    {
        let record = state.esp_loader.lookup(None, *id).await.unwrap();
        records.push(record);
    }

    let tasks = FuturesUnordered::new();
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
        let loader = Arc::clone(&loader);
        let task = pool.spawn(async move {
            let worldspace = WorldSpace {
                record: worldspace,
                is_interior,
            };

            loader
                .load_custom(record.id, mode.flags(), || {
                    std::future::ready(CustomRecordInfo {
                        record,
                        worldspace,
                        cell: cell_id,
                    })
                })
                .await
                .unwrap()
        });

        tasks.push(task);
    }

    tasks.collect().await
}

async fn resolve_cell_to_record(state: &LoaderState, req: Request) -> RecordId {
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

    let cells = state
        .esp_loader
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

#[derive(Clone, Debug)]
struct LoadedCell {
    mode: CellActivityMode,
    entities: Vec<EntityRef>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CellActivityMode {
    VisibleDistant,
    /// The cell is fully loaded.
    Active,
}

impl CellActivityMode {
    const fn flags(&self) -> PartialEntityFlags {
        match self {
            Self::Active => PartialEntityFlags {
                graphics: true,
                physics: true,
                actor: true,
                scene: true,
                script: true,
                lod: LodLevel::Normal,
            },
            Self::VisibleDistant => PartialEntityFlags {
                graphics: true,
                scene: true,
                physics: false,
                actor: false,
                script: false,
                lod: LodLevel::Distant0,
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Request {
    worldspace: RecordId,
    cell: CellId,
    mode: CellActivityMode,
}
