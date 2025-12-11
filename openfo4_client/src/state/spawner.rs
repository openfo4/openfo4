use std::collections::HashMap;
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use game_render::mesh::Mesh;
use game_tasks::TaskPool;
use hecs::CommandBuffer;
use openfo4_esp::{FormId, RecordData, ZString};

use crate::cell::CellId;
use crate::loader::FileLoader;

use super::esp::EspLoader;

#[derive(Clone, Debug)]
pub enum SpawnEvent {
    Land { cell: CellId, mesh: Mesh },
    Record { id: FormId },
}

#[derive(Clone, Debug)]
pub struct Spawner {
    loader: Arc<EspLoader>,
    pool: Arc<TaskPool>,
    events: Arc<SegQueue<SpawnEvent>>,
}

impl Spawner {
    pub fn new() -> Self {
        todo!()
    }

    pub async fn update(&mut self) {
        while let Some(event) = self.events.pop() {
            match event {
                SpawnEvent::Land { cell, mesh } => {}
                SpawnEvent::Record { id } => {
                    let record = self.loader.lookup(None, id).await.unwrap();
                }
            }
        }
    }
}

async fn load_record(loader: &EspLoader, file_loader: &FileLoader, id: FormId) -> CommandBuffer {
    let mut buffer = CommandBuffer::new();

    let record = loader.lookup(None, id).await.unwrap();

    let mut model = None;

    match record.data {
        RecordData::STAT(data) => {
            model = data.modl.model;
        }
        RecordData::MSTT(data) => {
            model = data.model.model;
        }
        RecordData::Door(data) => {
            model = data.model.model;
        }
        _ => (),
    }

    if let Some(model) = model {}

    buffer
}
