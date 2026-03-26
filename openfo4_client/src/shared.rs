use std::collections::HashMap;
use std::sync::RwLock;

use game_core_pipeline::entities::{Entities, ImageHandle, MaterialHandle, MeshHandle};

#[derive(Debug)]
pub struct SharedState {
    pub entities: Entities,
    pub images_map: RwLock<HashMap<String, ImageHandle>>,
    pub materials_map: RwLock<HashMap<String, MaterialHandle>>,
    pub meshes_map: RwLock<HashMap<String, MeshHandle>>,
}

impl SharedState {
    pub fn new(entities: Entities) -> Self {
        Self {
            entities,
            images_map: RwLock::new(HashMap::new()),
            materials_map: RwLock::new(HashMap::new()),
            meshes_map: RwLock::new(HashMap::new()),
        }
    }
}
