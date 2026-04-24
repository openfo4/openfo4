use std::sync::Arc;

use game_common::components::Transform;
use game_core_pipeline::entities::{Entities, MaterialHandle};
use game_core_pipeline::material::StandardMaterial;
use game_core_pipeline::terrain::{TerrainMaterial, TerrainMaterialLayer};
use game_render::mesh::{Indices, Mesh};
use game_tracing::trace_span;
use glam::{UVec2, Vec2, Vec3};
use hashbrown::HashMap;
use openfo4_asset::Handle;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::transform::ConvertCoordinateSystem;

use crate::assets::texture::FileImageHandle;
use crate::cell::{CELL_SIZE, CellId};
use crate::components::{Player, WorldSpace};
use crate::render::RenderObject;

const QUAD_POINTS: UVec2 = UVec2::splat(17);

/// Translation offset applied to terrain mesh entities.
///
/// This is used to place the entity in the center of the cell. If we place it at the edge of cells
/// it causes some numerical instabilities that cause terrains to get incorrectly assigned to cells.
pub const LAND_OFFSET: Vec3 = Vec3::new(2048.0, 2048.0, 0.0);

/// The size in worldspace units of a single quadrant of a cell mesh.
// FIXME: Replace when traits are const
// QUAD_SIZE = CELL_SIZE / 2 + UNIT_SIZE
const QUAD_SIZE: UVec2 = CELL_SIZE.checked_div(UVec2::splat(2)).unwrap();

pub struct TerrainManager {
    regions: HashMap<CellId, TerrainRegion>,
    entities: Entities,
}

impl TerrainManager {
    pub fn new(entities: Entities) -> Self {
        Self {
            entities,
            regions: HashMap::new(),
        }
    }

    pub fn update(&mut self, world: &mut World) {
        let _span = trace_span!("TerrainManager::update").entered();

        let Some((_, worldspace, _)) = world
            .query_mut::<(&Transform, &WorldSpace, &Player)>()
            .into_iter()
            .next()
        else {
            return;
        };

        let worldspace = *worldspace;

        for (cell, region) in &mut self.regions {
            if !region.entities.is_empty() {
                continue;
            }

            let q0 = build_quadrant(&region, &self.entities, TerrainQuadrant::LxLy);
            let q1 = build_quadrant(&region, &self.entities, TerrainQuadrant::LxHy);
            let q2 = build_quadrant(&region, &self.entities, TerrainQuadrant::HxLy);
            let q3 = build_quadrant(&region, &self.entities, TerrainQuadrant::HxHy);

            let t0 = Transform::from_translation(cell.min().extend(0.0) + LAND_OFFSET).to_opengl();
            let t1 = Transform::from_translation(
                cell.min().extend(0.0) + LAND_OFFSET + Vec3::new(0.0, QUAD_SIZE.y as f32, 0.0),
            )
            .to_opengl();
            let t2 = Transform::from_translation(
                cell.min().extend(0.0) + LAND_OFFSET + Vec3::new(QUAD_SIZE.x as f32, 0.0, 0.0),
            )
            .to_opengl();
            let t3 = Transform::from_translation(
                cell.min().extend(0.0)
                    + LAND_OFFSET
                    + Vec3::new(QUAD_SIZE.x as f32, QUAD_SIZE.y as f32, 0.0),
            )
            .to_opengl();

            let e0 = world.spawn((t0, q0, worldspace));
            let e1 = world.spawn((t1, q1, worldspace));
            let e2 = world.spawn((t2, q2, worldspace));
            let e3 = world.spawn((t3, q3, worldspace));
            region.entities.extend([e0, e1, e2, e3]);
        }
    }

    pub fn insert(&mut self, region: TerrainRegion) {
        self.regions.insert(region.cell, region);
    }
}

#[derive(Clone, Debug)]
pub struct TerrainRegion {
    pub cell: CellId,
    pub heights: Box<[[f32; 33]; 33]>,
    pub normals: Box<[[Vec3; 33]; 33]>,
    pub layers: Arc<Vec<TerrainLayer>>,
    pub entities: Vec<Entity>,
}

#[derive(Clone, Debug)]
pub struct TerrainLayer {
    pub position: TerrainQuadrant,
    pub layer: i16,
    pub alpha: [[f32; 17]; 17],
    pub diffuse_texture: Option<Handle<FileImageHandle>>,
    pub normal_texture: Option<Handle<FileImageHandle>>,
    pub smooth_spec_texture: Option<Handle<FileImageHandle>>,
    pub smoothness: f32,
    pub specular_strength: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TerrainQuadrant {
    /// Low X Low Y
    LxLy,
    /// Low X High Y
    LxHy,
    /// High X Low Y
    HxLy,
    // High X High Y
    HxHy,
}

fn build_quadrant(
    region: &TerrainRegion,
    entities: &Entities,
    quad: TerrainQuadrant,
) -> RenderObject {
    let mut layers: Vec<_> = region
        .layers
        .iter()
        .filter(|layer| layer.position == quad)
        .collect();
    layers.sort_by(|a, b| a.layer.cmp(&b.layer));

    let mesh = build_quadrant_mesh(&region.heights, &region.normals, quad);
    let mesh = entities.create_mesh(mesh);

    let material = build_quadrant_material(entities, &layers);

    RenderObject { mesh, material }
}

fn build_quadrant_mesh(
    heights: &[[f32; 33]; 33],
    normals: &[[Vec3; 33]; 33],
    quad: TerrainQuadrant,
) -> Mesh {
    let (start_x, start_y, end_x, end_y) = match quad {
        TerrainQuadrant::LxLy => (0, 0, 17, 17),
        TerrainQuadrant::LxHy => (0, 16, 17, 33),
        TerrainQuadrant::HxLy => (16, 0, 33, 17),
        TerrainQuadrant::HxHy => (16, 16, 33, 33),
    };

    let mut positions = Vec::new();
    let mut norms = Vec::new();
    let mut uvs = Vec::new();

    for x in start_x..end_x {
        for y in start_y..end_y {
            let height = heights[x][y];
            let normal = normals[x][y];

            let l_x = (x - start_x) as f32 / (end_x - 1 - start_x) as f32;
            let l_y = (y - start_y) as f32 / (end_y - 1 - start_y) as f32;

            let x = l_x * QUAD_SIZE.x as f32;
            let y = l_y * QUAD_SIZE.y as f32;

            positions.push(Vec3::new(x, y, height) - LAND_OFFSET);
            norms.push(normal);
            uvs.push(Vec2::new(l_x, l_y));
        }
    }

    let mut indices = Vec::new();
    for index in 0..QUAD_POINTS.x * QUAD_POINTS.y {
        let x = index % QUAD_POINTS.x;
        let y = index / QUAD_POINTS.x;

        if x != QUAD_POINTS.x - 1 && y != QUAD_POINTS.y - 1 {
            indices.extend([
                index,
                index + QUAD_POINTS.x,
                index + QUAD_POINTS.x + 1,
                index + QUAD_POINTS.x + 1,
                index + 1,
                index,
            ]);
        }
    }

    let mut mesh = Mesh::new();
    mesh.set_positions(positions);
    mesh.set_normals(norms);
    mesh.set_uvs(uvs);
    mesh.set_indices(Indices::U32(indices));
    mesh.compute_tangents();

    mesh
}

fn build_quadrant_material(entities: &Entities, regions: &[&TerrainLayer]) -> MaterialHandle {
    let mut layers = Vec::new();

    for region in regions {
        layers.push(TerrainMaterialLayer {
            material: StandardMaterial {
                base_color_texture: region.diffuse_texture.as_ref().map(|v| v.0.clone()),
                normal_texture: region.normal_texture.as_ref().map(|v| v.0.clone()),
                specular_glossiness_texture: region
                    .smooth_spec_texture
                    .as_ref()
                    .map(|v| v.0.clone()),
                roughness: 1.0 - region.smoothness,
                specular_strength: region.specular_strength,
                flip_normal_y: true,
                two_component_normal_encoding: true,
                ..Default::default()
            },
            alpha: region.alpha,
        });
    }

    entities.create_terrain_material(TerrainMaterial { layers })
}
