use crate::material::StandardMaterial;

#[derive(Clone, Debug)]
pub struct TerrainMaterial {
    pub layers: Vec<TerrainMaterialLayer>,
}

#[derive(Clone, Debug)]
pub struct TerrainMaterialLayer {
    pub material: StandardMaterial,
    pub alpha: [[f32; 17]; 17],
}
