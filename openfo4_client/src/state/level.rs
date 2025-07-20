use std::collections::HashSet;
use std::io::Read;
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use game_common::components::Transform;
use game_core::hierarchy::Hierarchy;
use game_core_pipeline::material::StandardMaterial;
use game_render::backend::TextureFormat;
use game_render::mesh::{Indices, Mesh};
use game_render::texture::Image;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::{EulerRot, Quat, UVec2, Vec2, Vec3};
use hecs::World;
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::hierarchy::Children;
use openfo4_esp::records::{Cell, LandLandTexture, LandTextureAlphaLayerPoint, Quadrant};
use openfo4_esp::{FormId, RecordChildren, RecordData, RecordHeaderFlags, RecordKind};
use openfo4_hkx::behavior::EventBase;
use openfo4_nif::bgsm::Bgsm;
use openfo4_physics::components::{Collider, ColliderShape, RigidBody, RigidBodyKind, TriMesh};
use tracing::instrument;

use crate::assets::nif_v2::{Instantiate, NifScene};
use crate::cell::{CELL_SIZE, CellId};
use crate::components::{NiTransform, Root};
use crate::events::{Event, UnloadCell};
use crate::loader::FileLoader;
use crate::render::RenderObject;
use crate::shared::SharedState;

use super::asset::load_image_buffer;
use super::esp::EspLoader;
use super::terrain::{TerrainLayer, combine_texture_layers};

const LAND_POINTS: UVec2 = UVec2::splat(33);

/// A streamer is an entity that causes a region around it to be loaded (i.e. streamed in).
#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub struct Level {
    pub loader: Arc<EspLoader>,
    events: Arc<SegQueue<SpawnEvent>>,
    tx: async_channel::Sender<CellId>,
    cells_loaded: HashSet<CellId>,
    cells_to_unload_next_tick: HashSet<CellId>,
}

impl Level {
    pub fn new(
        loader: EspLoader,
        asset_loader: AssetLoader,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
    ) -> Self {
        let (tx, rx) = async_channel::unbounded::<CellId>();
        let events = Arc::new(SegQueue::new());
        let loader = Arc::new(loader);

        pool.spawn({
            let loader = loader.clone();
            let events = events.clone();
            let pool = pool.clone();

            async move {
                let worldspace = FormId(0x3C);

                while let Ok(cell_id) = rx.recv().await {
                    let mut form_id = None;
                    for record in loader
                        .children(RecordChildren::Cells, worldspace)
                        .await
                        .unwrap()
                    {
                        match record.data {
                            RecordData::Cell(Cell {
                                grid: Some(grid), ..
                            }) => {
                                if grid.x == cell_id.0.x && grid.y == cell_id.0.y {
                                    form_id = Some(record.id);
                                    break;
                                }
                            }
                            _ => (),
                        }
                    }

                    let Some(form_id) = form_id else {
                        continue;
                    };

                    load_cell(
                        &asset_loader,
                        &loader,
                        &events,
                        &pool,
                        &file_loader,
                        &shared,
                        form_id,
                    )
                    .await;
                }
            }
        })
        .deatch();

        Self {
            loader,
            events,
            tx,
            cells_loaded: HashSet::new(),
            cells_to_unload_next_tick: HashSet::new(),
        }
    }

    pub fn update(&mut self, world: &mut World, events: &mut Vec<Event>) {
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
                    self.tx.try_send(cell).unwrap();
                }

                self.cells_loaded.insert(cell);
                prev_cells_loaded.remove(&cell);
            }
        }

        while let Some(event) = self.events.pop() {
            match event {
                SpawnEvent::SpawnEntity(event) => {
                    // It is possible for the event to arrive after the cell
                    // was already unloaded again.
                    // In this case we must not spawn the new entity to prevent
                    // duplicating entities when the cell gets loaded again.
                    let cell = CellId::from(event.transform);
                    if !self.cells_loaded.contains(&cell) {
                        continue;
                    }

                    event.scene.instantiate(event.transform.into(), world);
                }
                SpawnEvent::SpawnTerrain(transform, object, collider) => {
                    dbg!("TERRAIN");
                    world.spawn((
                        transform,
                        object,
                        collider,
                        RigidBody {
                            kind: RigidBodyKind::Fixed,
                            linvel: Vec3::ZERO,
                            angvel: Vec3::ZERO,
                        },
                    ));
                }
            }
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

        for (entity, (cell, children, _)) in
            world.query_mut::<(&CellId, Option<&Children>, &Root)>()
        {
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

#[derive(Clone, Debug)]
enum SpawnEvent {
    SpawnEntity(SpawnEntity),
    SpawnTerrain(Transform, RenderObject, Collider),
}

#[derive(Clone, Debug)]
pub struct SpawnEntity {
    pub transform: NiTransform,
    pub scene: Handle<NifScene>,
}

async fn load_cell(
    asset_loader: &AssetLoader,
    loader: &Arc<EspLoader>,
    events: &Arc<SegQueue<SpawnEvent>>,
    pool: &TaskPool,
    file_loader: &Arc<FileLoader>,
    shared: &Arc<SharedState>,
    cell_id: FormId,
) {
    let cell = match loader.lookup(Some(RecordKind::Cell), cell_id).await {
        Ok(record) => match record.data {
            RecordData::Cell(cell) => cell,
            _ => {
                tracing::error!("bad record type expected cell");
                return;
            }
        },
        Err(err) => {
            tracing::error!("invalid cell {:?}: {}", cell_id, err);
            return;
        }
    };

    let mut offset = Vec2::ZERO;
    if let Some(grid) = cell.grid {
        offset = CellId::new(grid.x, grid.y).min();
    }

    let persistent = loader
        .children(RecordChildren::PersistentEntities, cell_id)
        .await
        .unwrap();
    let temporary = loader
        .children(RecordChildren::TemporaryEntities, cell_id)
        .await
        .unwrap();

    for record in persistent.into_iter().chain(temporary) {
        let loader = loader.clone();
        let file_loader = file_loader.clone();
        let shared = shared.clone();
        let events = events.clone();
        let asset_loader = asset_loader.clone();

        pool.spawn(async move {
            let mut position;
            let scale;
            let base_object;

            match record.data {
                RecordData::PlacedObject(object) => {
                    position = object.position;
                    base_object = object.base_object;
                    scale = object.scale.unwrap_or(1.0);
                }
                RecordData::PlacedNpc(npc) => {
                    position = npc.position;
                    base_object = npc.base_npc;
                    scale = 1.0;
                }
                RecordData::Land(land) => {
                    let positions: Vec<Vec3> = land
                        .vertex_height
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
                            Vec3::new(x, y, z)
                        })
                        .collect();
                    let normals = land
                        .vertex_normals
                        .unwrap()
                        .0
                        .iter()
                        .map(|[x, y, z]| {
                            (Vec3::new(*x as f32, *y as f32, *z as f32) / i8::MAX as f32)
                                .normalize_or_zero()
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
                            load_terrain_layer(base_layer, None, &loader, &file_loader).await
                        {
                            layers.push(layer);
                        }
                    }

                    for alpha_layer in &land.alpha_layer.0 {
                        continue;

                        if alpha_layer.texture.land_texture == FormId(0) {
                            continue;
                        }

                        if let Some(layer) = load_terrain_layer(
                            &alpha_layer.texture,
                            Some(&alpha_layer.alpha_layer_data.0),
                            &loader,
                            &file_loader,
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
                        base_color_texture: Some(shared.entities.create_image(Image::new(
                            size,
                            TextureFormat::Rgba8UnormSrgb,
                            terrain.diffuse_texture.into_vec(),
                        ))),
                        normal_texture: Some(shared.entities.create_image(Image::new(
                            size,
                            TextureFormat::Rgba8Unorm,
                            terrain.normal_texture.into_vec(),
                        ))),
                        // Normal maps are still in DirectX format.
                        flip_normal_y: true,
                        two_component_normal_encoding: true,
                        specular_glossiness_texture: Some(shared.entities.create_image(
                            Image::new(
                                size,
                                TextureFormat::Rgba8UnormSrgb,
                                terrain.smooth_spec_texture.into_vec(),
                            ),
                        )),
                        // The glossiness is reconstructed from 1.0 - roughness.
                        // We want a glossiness factor of 1.0 since we have premultiplied
                        // all glossiness factors into the glossiness texture.
                        roughness: 0.0,
                        specular_strength: 1.0,
                        ..Default::default()
                    };

                    let mesh = shared.entities.create_mesh(mesh);
                    let material = shared.entities.create_material(material);

                    let collider = Collider {
                        friction: 1.0,
                        restitution: 1.0,
                        shape: ColliderShape::TriMesh(TriMesh::new(positions, indices)),
                    };

                    events.push(SpawnEvent::SpawnTerrain(
                        Transform::from(NiTransform::from_translation(offset.extend(0.0))),
                        RenderObject { mesh, material },
                        collider,
                    ));

                    // events.push(SpawnEvent::SpawnEntity(SpawnEntity {
                    //     transform: NiTransform::from_translation(offset.extend(0.0)),
                    //     scene: NifScene {
                    //         handle: None,
                    //         nodes: vec![Instance {
                    //             transform: NiTransform::default(),
                    //             collider: Some(collider),
                    //             skin: None,
                    //             renderable: Some(RenderObject {
                    //                 mesh: mesh,
                    //                 material: material,
                    //             }),
                    //         }],
                    //     },
                    // }));

                    return;
                }
                _ => return,
            }

            let transform = Transform {
                translation: Vec3::new(
                    position.position_x,
                    position.position_y,
                    position.position_z,
                ),
                // Rotation is negative for some reason?
                rotation: Quat::from_euler(
                    EulerRot::XYZ,
                    -position.rotation_x,
                    -position.rotation_y,
                    -position.rotation_z,
                ),
                scale: Vec3::splat(scale),
            };

            let base = match loader.lookup(None, base_object).await {
                Ok(base) => base,
                Err(err) => {
                    // TODO
                    return;
                }
            };

            if base.flags.contains(RecordHeaderFlags::IS_MARKER) {
                return;
            }

            let model = match base.data {
                RecordData::STAT(data) => data.modl.model,
                RecordData::StaticCollection(data) => data.model.model,
                RecordData::MSTT(data) => data.model.model,
                RecordData::DOOR(data) => data.modl.model,
                RecordData::Container(data) => data.modl.model,
                RecordData::Npc(_) => return,
                RecordData::IdleMarker(_) => return,
                RecordData::Activator(data) => data.model.model,
                RecordData::MiscObject(data) => data.model.model,
                RecordData::Furniture(data) => data.model.model,
                RecordData::Light(data) => data.model.model,
                RecordData::Ingestible(data) => data.model.model,
                _ => return,
                x => todo!("{:?}", x),
            };

            let Some(model) = model else {
                return;
            };

            let path = format!("meshes/{}", model.as_str().unwrap());
            match asset_loader.load::<NifScene, _>(&path).await {
                Ok(scene) => {
                    events.push(SpawnEvent::SpawnEntity(SpawnEntity {
                        transform: NiTransform::from_ni_transform(transform),
                        scene: scene.clone(),
                    }));
                }
                Err(err) => {
                    tracing::error!("{}", err);
                }
            }
        });
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
        RecordData::TXST(set) => set,
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
