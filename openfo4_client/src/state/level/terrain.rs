use std::io::Read;

use game_core_pipeline::material::StandardMaterial;
use game_render::backend::TextureFormat;
use game_render::mesh::{Indices, Mesh};
use game_render::texture::Image;
use glam::{UVec2, Vec2, Vec3};
use openfo4_esp::records::{Land, LandLandTexture, LandTextureAlphaLayerPoint, Quadrant};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_nif::bgsm::Bgsm;
use openfo4_physics::components::{Collider, ColliderShape, CollisionLayer, TriMesh};

use crate::cell::CELL_SIZE;
use crate::components::NiTransform;
use crate::loader::FileLoader;
use crate::render::RenderObject;
use crate::state::asset::load_image_buffer;
use crate::state::esp::EspLoader;
use crate::state::level::LoadContext;
use crate::state::level::record::SpawnEntity;
use crate::state::terrain::{TerrainLayer, combine_texture_layers};

const LAND_POINTS: UVec2 = UVec2::splat(33);

/// Translation offset applied to terrain mesh entities.
///
/// This is used to place the entity in the center of the cell. If we place it at the edge of cells
/// it causes some numerical instabilities that cause terrains to get incorrectly assigned to cells.
const LAND_OFFSET: Vec3 = Vec3::new(2048.0, 2048.0, 0.0);

pub async fn load_land(ctx: &LoadContext<'_>, land: &Land) -> SpawnEntity {
    let positions: Vec<Vec3> = land
        .vertex_height
        .as_ref()
        .unwrap()
        .heightmap
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, point)| {
            // Grid lines up if we stretch by 128, i.e.
            // CELL_SIZE / (LAND_POINTS-1) = 128.
            let cell_size = CELL_SIZE + CELL_SIZE / (LAND_POINTS - 1);

            let x = (index / 33) as f32 / LAND_POINTS.x as f32 * cell_size.x as f32;
            let y = (index % 33) as f32 / LAND_POINTS.y as f32 * cell_size.y as f32;
            let z = *point;
            Vec3::new(x, y, z) - LAND_OFFSET
        })
        .collect();
    let normals = land
        .vertex_normals
        .as_ref()
        .unwrap()
        .0
        .iter()
        .map(|[x, y, z]| {
            (Vec3::new(*x as f32, *y as f32, *z as f32) / i8::MAX as f32).normalize_or_zero()
        })
        .collect();

    let mut uvs = Vec::new();
    for x in 0..33 {
        for y in 0..33 {
            uvs.push(Vec2::new(x as f32, y as f32) / 33.0);
        }
    }

    let mut indices = Vec::new();
    for index in 0..LAND_POINTS.x * LAND_POINTS.y {
        let x = index % LAND_POINTS.x;
        let y = index / LAND_POINTS.y;

        if x != LAND_POINTS.x - 1 && y != LAND_POINTS.y - 1 {
            indices.extend([
                index,
                index + LAND_POINTS.x,
                index + LAND_POINTS.x + 1,
                index + LAND_POINTS.x + 1,
                index + 1,
                index,
            ]);
        }
    }

    let mut mesh = Mesh::new();
    mesh.set_positions(positions.clone());
    mesh.set_normals(normals);
    mesh.set_uvs(uvs);
    mesh.set_indices(Indices::U32(indices.clone()));
    mesh.compute_tangents();

    let mut layers = Vec::new();
    for base_layer in &land.base_layer {
        if base_layer.land_texture == FormId(0) {
            continue;
        }

        if let Some(layer) =
            load_terrain_layer(base_layer, None, ctx.esp_loader, ctx.file_loader).await
        {
            layers.push(layer);
        }
    }

    for alpha_layer in &land.alpha_layer.0 {
        // continue;

        if alpha_layer.texture.land_texture == FormId(0) {
            continue;
        }

        if let Some(layer) = load_terrain_layer(
            &alpha_layer.texture,
            Some(&alpha_layer.alpha_layer_data.0),
            ctx.esp_loader,
            ctx.file_loader,
        )
        .await
        {
            layers.push(layer);
        }
    }

    let size = UVec2::splat(4096);
    layers.sort_by(|a, b| a.layer.cmp(&b.layer));

    let terrain = combine_texture_layers(&layers, size);

    let material = StandardMaterial {
        base_color_texture: Some(ctx.shared.entities.create_image(Image::new(
            size,
            TextureFormat::Rgba8UnormSrgb,
            terrain.diffuse_texture.into_vec(),
        ))),
        normal_texture: Some(ctx.shared.entities.create_image(Image::new(
            size,
            TextureFormat::Rgba8Unorm,
            terrain.normal_texture.into_vec(),
        ))),
        // Normal maps are still in DirectX format.
        flip_normal_y: true,
        two_component_normal_encoding: true,
        specular_glossiness_texture: Some(ctx.shared.entities.create_image(Image::new(
            size,
            TextureFormat::Rgba8UnormSrgb,
            terrain.smooth_spec_texture.into_vec(),
        ))),
        // The glossiness is reconstructed from 1.0 - roughness.
        // We want a glossiness factor of 1.0 since we have premultiplied
        // all glossiness factors into the glossiness texture.
        roughness: 0.0,
        specular_strength: 1.0,
        ..Default::default()
    };

    let mesh = ctx.shared.entities.create_mesh(mesh);
    let material = ctx.shared.entities.create_material(material);

    let collider = Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::TriMesh(TriMesh::new(positions, indices)),
        layer: CollisionLayer::STATIC,
    };

    SpawnEntity {
        transform: NiTransform::from_translation(ctx.grid_offset().extend(0.0) + LAND_OFFSET),
        collider: Some(collider),
        object: Some(RenderObject { mesh, material }),
        ..Default::default()
    }
}

async fn load_terrain_layer(
    texture: &LandLandTexture,
    alpha_layer: Option<&[LandTextureAlphaLayerPoint]>,
    esp_loader: &EspLoader,
    file_loader: &FileLoader,
) -> Option<TerrainLayer> {
    let land_texture = esp_loader
        .lookup(Some(RecordKind::LandTexture), texture.land_texture)
        .await
        .unwrap();

    let land_texture = match land_texture.data {
        RecordData::LandTexture(data) => data,
        _ => todo!(),
    };

    let texture_set = esp_loader
        .lookup(Some(RecordKind::TextureSet), land_texture.texture_set)
        .await
        .unwrap();

    let texture_set = match texture_set.data {
        RecordData::TextureSet(set) => set,
        _ => todo!(),
    };

    let position = match texture.quadrant {
        Quadrant::BottomLeft => UVec2::ZERO,
        Quadrant::BottomRight => UVec2::new(CELL_SIZE.x / 2, 0),
        Quadrant::TopLeft => UVec2::new(0, CELL_SIZE.y / 2),
        Quadrant::TopRight => CELL_SIZE / 2,
    };

    let diffuse_texture = match &texture_set.material {
        Some(material) => {
            let mut reader = file_loader
                .open(&format!("materials/{}", material.as_str().unwrap()))
                .await
                .unwrap();

            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();

            let bgsm = Bgsm::decode(&buf).unwrap();

            Some(
                load_image_buffer(file_loader, bgsm.diffuse_texture)
                    .await
                    .unwrap(),
            )
        }
        None => match &texture_set.diffuse_texture {
            Some(path) => Some(
                load_image_buffer(file_loader, path.as_str().unwrap().to_owned())
                    .await
                    .unwrap(),
            ),
            None => None,
        },
    };

    let normal_texture = match &texture_set.material {
        Some(material) => {
            let mut reader = file_loader
                .open(&format!("materials/{}", material.as_str().unwrap()))
                .await
                .unwrap();

            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();

            let bgsm = Bgsm::decode(&buf).unwrap();

            Some(
                load_image_buffer(file_loader, bgsm.normal_texture)
                    .await
                    .unwrap(),
            )
        }
        None => match &texture_set.normal_texture {
            Some(path) => Some(
                load_image_buffer(file_loader, path.as_str().unwrap().to_owned())
                    .await
                    .unwrap(),
            ),
            None => None,
        },
    };

    let smooth_spec_texture = match &texture_set.material {
        Some(material) => {
            let mut reader = file_loader
                .open(&format!("materials/{}", material.as_str().unwrap()))
                .await
                .unwrap();

            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();

            let bgsm = Bgsm::decode(&buf).unwrap();

            Some(
                load_image_buffer(file_loader, bgsm.smooth_spec_texture)
                    .await
                    .unwrap(),
            )
        }
        None => match &texture_set.smooth_spec_texture {
            Some(path) => Some(
                load_image_buffer(file_loader, path.as_str().unwrap().to_owned())
                    .await
                    .unwrap(),
            ),
            None => None,
        },
    };

    let smoothness = match &texture_set.material {
        Some(material) => {
            let mut reader = file_loader
                .open(&format!("materials/{}", material.as_str().unwrap()))
                .await
                .unwrap();

            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();

            let bgsm = Bgsm::decode(&buf).unwrap();

            bgsm.smoothness
        }
        None => 1.0,
    };

    let specular_strength = match &texture_set.material {
        Some(material) => {
            let mut reader = file_loader
                .open(&format!("materials/{}", material.as_str().unwrap()))
                .await
                .unwrap();

            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();

            let bgsm = Bgsm::decode(&buf).unwrap();

            bgsm.specular_strength
        }
        None => 1.0,
    };

    let alpha = if let Some(alpha_layer) = alpha_layer {
        let mut alpha = [[0.0; 17]; 17];

        for point in alpha_layer {
            let x = point.position % 17;
            let y = point.position / 17;
            alpha[x as usize][y as usize] = point.alpha;
        }

        alpha
    } else {
        // If the alpha layer is missing it is considered fully opaque (1.0).
        [[1.0; 17]; 17]
    };

    Some(TerrainLayer {
        position,
        layer: texture.texture_layer,
        alpha,
        diffuse_texture,
        normal_texture,
        size: CELL_SIZE / 2,
        smooth_spec_texture,
        smoothness,
        specular_strength,
    })
}
