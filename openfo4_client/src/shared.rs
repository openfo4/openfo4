use std::collections::HashMap;
use std::sync::RwLock;

use game_common::collections::arena::Key;
use game_core_pipeline::entities::{self, Entities, ImageHandle, MaterialHandle, MeshHandle};
use game_render::entities::{ImageId, MaterialId, MeshId};
use game_render::mesh::Mesh;
use game_render::texture::Image;

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

#[derive(Clone, Debug)]
pub enum MeshAsset {
    Cpu(Mesh),
    Gpu(MeshId),
}

#[derive(Clone, Debug)]
pub enum MaterialAsset {
    Cpu(Material),
    Gpu(MaterialId),
}

#[derive(Clone, Debug)]
pub enum ImageAsset {
    Cpu(Image),
    Gpu(ImageId),
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Material {
    pub diffuse_texture: Option<Key>,
    pub normal_texture: Option<Key>,
}
