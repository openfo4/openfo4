use std::sync::Arc;

use glam::{UVec2, Vec3};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{Land, LandLandTexture, LandTextureAlphaLayerPoint, Quadrant};
use openfo4_esp::{RecordData, RecordKind};
use openfo4_nif::bgsm::Bgsm;
use openfo4_physics::components::{Collider, ColliderShape, CollisionLayer, RigidBody, TriMesh};

use crate::assets::data_file::DataFile;
use crate::assets::texture::{FileImageHandle, FileImageKey};
use crate::cell::{CELL_SIZE, CellId};
use crate::components::NiTransform;
use crate::state::esp::EspLoader;
use crate::state::hardcoded::{
    PATH_LAND_TEXTURE_DEFAULT_DIFFUSE, PATH_LAND_TEXTURE_DEFAULT_NORMAL,
    PATH_LAND_TEXTURE_DEFAULT_SPEC_GLOSS,
};
use crate::state::level::LoadContext;
use crate::state::level::record::SpawnEntity;
use crate::state::terrain::{TerrainLayer, TerrainQuadrant, TerrainRegion};

pub const LAND_POINTS: UVec2 = UVec2::splat(33);

/// Translation offset applied to terrain mesh entities.
///
/// This is used to place the entity in the center of the cell. If we place it at the edge of cells
/// it causes some numerical instabilities that cause terrains to get incorrectly assigned to cells.
pub const LAND_OFFSET: Vec3 = Vec3::new(2048.0, 2048.0, 0.0);

pub async fn load_land(ctx: &LoadContext<'_>, land_id: RecordId, land: &Land) -> SpawnEntity {
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

    let mut heights = Box::new([[0.0; 33]; 33]);
    for a in 0..33 {
        for b in 0..33 {
            let height = land.vertex_height.as_ref().unwrap().heightmap[a][b];
            heights[a][b] = height;
        }
    }

    let mut normals = Box::new([[Vec3::ZERO; 33]; 33]);
    for a in 0..33 {
        for b in 0..33 {
            let [x, y, z] = land.vertex_normals.as_ref().unwrap()[b * 33 + a];
            normals[a][b] =
                (Vec3::new(x as f32, y as f32, z as f32) / i8::MAX as f32).normalize_or_zero();
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

    let mut layers = Vec::new();

    let mut base_layer_loaded_bottom_left = false;
    let mut base_layer_loaded_bottom_right = false;
    let mut base_layer_loaded_top_left = false;
    let mut base_layer_loaded_top_right = false;

    for base_layer in &land.base_layer {
        if let Some(layer) =
            load_terrain_layer(land_id, base_layer, None, ctx.esp_loader, ctx.loader).await
        {
            layers.push(layer);

            match base_layer.quadrant {
                Quadrant::BottomLeft => base_layer_loaded_bottom_left = true,
                Quadrant::BottomRight => base_layer_loaded_bottom_right = true,
                Quadrant::TopLeft => base_layer_loaded_top_left = true,
                Quadrant::TopRight => base_layer_loaded_top_right = true,
            }
        }
    }

    for (base_layer_loaded, position) in [
        (base_layer_loaded_bottom_left, TerrainQuadrant::LxLy),
        (base_layer_loaded_bottom_right, TerrainQuadrant::HxLy),
        (base_layer_loaded_top_left, TerrainQuadrant::LxHy),
        (base_layer_loaded_top_right, TerrainQuadrant::HxHy),
    ] {
        if base_layer_loaded {
            continue;
        }

        let material = load_default_terrain_material(ctx.loader).await;

        layers.push(TerrainLayer {
            position,
            layer: -1,
            alpha: [[1.0; 17]; 17],
            diffuse_texture: material.diffuse_texture,
            normal_texture: material.normal_texture,
            smooth_spec_texture: material.smooth_spec_texture,
            smoothness: material.smoothness,
            specular_strength: material.specular_strength,
        });
    }

    for alpha_layer in &land.alpha_layer.0 {
        if let Some(layer) = load_terrain_layer(
            land_id,
            &alpha_layer.texture,
            Some(&alpha_layer.alpha_layer_data.0),
            ctx.esp_loader,
            ctx.loader,
        )
        .await
        {
            layers.push(layer);
        }
    }

    layers.sort_by(|a, b| a.layer.cmp(&b.layer));

    let collider = Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::TriMesh(TriMesh::new(positions, indices)),
        layer: CollisionLayer::STATIC,
    };

    SpawnEntity {
        transform: NiTransform::from_translation(ctx.grid_offset().extend(0.0) + LAND_OFFSET),
        collider: Some(collider),
        rigid_body: Some(RigidBody::FIXED),
        terrain: Some(TerrainRegion {
            cell: CellId::from(ctx.grid_offset()),
            heights,
            normals,
            layers: Arc::new(layers),
            entities: Vec::new(),
        }),
        ..Default::default()
    }
}

async fn load_terrain_layer(
    land_id: RecordId,
    texture: &LandLandTexture,
    alpha_layer: Option<&[LandTextureAlphaLayerPoint]>,
    esp_loader: &EspLoader,
    loader: &AssetLoader,
) -> Option<TerrainLayer> {
    let material = if !texture.land_texture.is_null() {
        let id = esp_loader
            .resolve_form(texture.land_texture, land_id)
            .unwrap();
        load_terrain_material_from_form(esp_loader, loader, id).await
    } else {
        load_default_terrain_material(loader).await
    };

    let position = match texture.quadrant {
        Quadrant::BottomLeft => TerrainQuadrant::LxLy,
        Quadrant::BottomRight => TerrainQuadrant::HxLy,
        Quadrant::TopLeft => TerrainQuadrant::LxHy,
        Quadrant::TopRight => TerrainQuadrant::HxHy,
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
        diffuse_texture: material.diffuse_texture,
        normal_texture: material.normal_texture,
        smooth_spec_texture: material.smooth_spec_texture,
        smoothness: material.smoothness,
        specular_strength: material.specular_strength,
    })
}

#[derive(Clone, Debug)]
struct TerrainLayerMaterial {
    smoothness: f32,
    specular_strength: f32,
    diffuse_texture: Option<Handle<FileImageHandle>>,
    normal_texture: Option<Handle<FileImageHandle>>,
    smooth_spec_texture: Option<Handle<FileImageHandle>>,
}

async fn load_terrain_material_from_form(
    esp_loader: &EspLoader,
    loader: &AssetLoader,
    id: RecordId,
) -> TerrainLayerMaterial {
    let land_texture_record = esp_loader
        .lookup(Some(RecordKind::LandTexture), id)
        .await
        .unwrap();

    let land_texture = match land_texture_record.local.data {
        RecordData::LandTexture(data) => data,
        _ => todo!(),
    };

    let texture_set = esp_loader
        .resolve_form(land_texture.texture_set, land_texture_record.id)
        .unwrap();

    let texture_set = esp_loader
        .lookup(Some(RecordKind::TextureSet), texture_set)
        .await
        .unwrap();

    let texture_set = match texture_set.local.data {
        RecordData::TextureSet(set) => set,
        _ => todo!(),
    };

    let bgsm = match &texture_set.material {
        Some(path) => {
            let mut path = DataPathBuf::new(path.as_bytes());
            path.push_front("materials");

            let bgsm = loader.load::<DataFile<Bgsm>, _>(&path).await.unwrap();
            Some(bgsm)
        }
        None => None,
    };

    let diffuse_path = match &bgsm {
        Some(bgsm) => Some(bgsm.diffuse_texture.as_bytes()),
        None => match &texture_set.diffuse_texture {
            Some(path) => Some(path.as_bytes()),
            None => None,
        },
    }
    .map(|path| {
        let mut path = DataPathBuf::new(path);
        path.push_front("textures");
        path
    });

    let normal_path = match &bgsm {
        Some(bgsm) => Some(bgsm.normal_texture.as_bytes()),
        None => match &texture_set.normal_texture {
            Some(path) => Some(path.as_bytes()),
            None => None,
        },
    }
    .map(|path| {
        let mut path = DataPathBuf::new(path);
        path.push_front("textures");
        path
    });

    let smooth_spec_path = match &bgsm {
        Some(bgsm) => Some(bgsm.smooth_spec_texture.as_bytes()),
        None => match &texture_set.smooth_spec_texture {
            Some(path) => Some(path.as_bytes()),
            None => None,
        },
    }
    .map(|path| {
        let mut path = DataPathBuf::new(path);
        path.push_front("textures");
        path
    });

    let diffuse_texture = match &diffuse_path {
        Some(path) => Some(
            loader
                .load::<FileImageHandle, _>(&FileImageKey {
                    path: path.clone(),
                    force_srgb: Some(true),
                })
                .await
                .unwrap(),
        ),
        None => None,
    };
    let normal_texture = match &normal_path {
        Some(path) => Some(
            loader
                .load::<FileImageHandle, _>(&FileImageKey {
                    path: path.clone(),
                    force_srgb: None,
                })
                .await
                .unwrap(),
        ),
        None => None,
    };
    let smooth_spec_texture = match &smooth_spec_path {
        Some(path) => Some(
            loader
                .load::<FileImageHandle, _>(&FileImageKey {
                    path: path.clone(),
                    force_srgb: None,
                })
                .await
                .unwrap(),
        ),
        None => None,
    };

    let smoothness = bgsm.as_ref().map(|v| v.smoothness).unwrap_or(1.0);
    let specular_strength = bgsm.as_ref().map(|v| v.specular_strength).unwrap_or(1.0);

    TerrainLayerMaterial {
        smoothness,
        specular_strength,
        diffuse_texture,
        normal_texture,
        smooth_spec_texture,
    }
}

async fn load_default_terrain_material(loader: &AssetLoader) -> TerrainLayerMaterial {
    let diffuse_texture = loader
        .load::<FileImageHandle, _>(&FileImageKey {
            path: PATH_LAND_TEXTURE_DEFAULT_DIFFUSE.to_owned(),
            force_srgb: Some(true),
        })
        .await
        .unwrap();
    let normal_texture = loader
        .load::<FileImageHandle, _>(&FileImageKey {
            path: PATH_LAND_TEXTURE_DEFAULT_NORMAL.to_owned(),
            force_srgb: None,
        })
        .await
        .unwrap();
    let smooth_spec_texture = loader
        .load::<FileImageHandle, _>(&FileImageKey {
            path: PATH_LAND_TEXTURE_DEFAULT_SPEC_GLOSS.to_owned(),
            force_srgb: None,
        })
        .await
        .unwrap();

    TerrainLayerMaterial {
        smoothness: 1.0,
        specular_strength: 1.0,
        diffuse_texture: Some(diffuse_texture),
        normal_texture: Some(normal_texture),
        smooth_spec_texture: Some(smooth_spec_texture),
    }
}
