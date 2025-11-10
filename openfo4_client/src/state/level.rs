use std::collections::HashSet;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use crossbeam::queue::SegQueue;
use game_common::components::{Color, PointLight, Transform};
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
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_esp::records::{
    Cell, LandLandTexture, LandTextureAlphaLayerPoint, LightDataFlags, Quadrant,
};
use openfo4_esp::{FormId, RecordChildren, RecordData, RecordHeaderFlags, RecordKind};
use openfo4_hkx::behavior::EventBase;
use openfo4_nif::bgsm::Bgsm;
use openfo4_physics::components::{
    Collider, ColliderShape, CollisionLayer, RigidBody, RigidBodyKind, TriMesh,
};
use tracing::instrument;

use crate::assets::nif_v2::{Instantiate, NIF_TO_METERS, NifScene};
use crate::cell::{CELL_SIZE, CellId};
use crate::components::{NiTransform, Root};
use crate::events::{Event, UnloadCell};
use crate::loader::FileLoader;
use crate::render::RenderObject;
use crate::shared::SharedState;
use crate::state::npc::Actor;
use crate::systems::lights::{LightFlicker, LightFlickerMode, LightFlickerState};

use super::asset::load_image_buffer;
use super::esp::EspLoader;
use super::terrain::{TerrainLayer, combine_texture_layers};

const LAND_POINTS: UVec2 = UVec2::splat(33);

/// Translation offset applied to terrain mesh entities.
///
/// This is used to place the entity in the center of the cell. If we place it at the edge of cells
/// it causes some numerical instabilities that cause terrains to get incorrectly assigned to cells.
const LAND_OFFSET: Vec3 = Vec3::new(2048.0, 2048.0, 0.0);

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
        loader: Arc<EspLoader>,
        asset_loader: AssetLoader,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
    ) -> Self {
        let (tx, rx) = async_channel::unbounded::<CellId>();
        let events = Arc::new(SegQueue::new());

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

    pub fn update(&mut self, world: &mut World, events: &mut Vec<Event>, event_bus: &mut EventBus) {
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

                    let entity = if let Some(scene) = event.scene {
                        scene.instantiate(event.transform.into(), world)
                    } else if let Some(actor) = event.actor {
                        actor.instantiate(world, event_bus)
                    } else {
                        world.reserve_entity()
                    };

                    world
                        .insert_one(entity, Transform::from(event.transform))
                        .unwrap();
                    world.insert_one(entity, RigidBody::FIXED).unwrap();

                    if let Some(light) = event.light {
                        world.insert_one(entity, light).unwrap();
                    }

                    if let Some(light_flicker) = event.light_flicker {
                        world.insert_one(entity, light_flicker).unwrap();
                    }

                    world.insert_one(entity, Root).unwrap();
                }
                SpawnEvent::SpawnTerrain(transform, object, collider) => {
                    let cell = CellId::from(NiTransform::from(transform));
                    if !self.cells_loaded.contains(&cell) {
                        continue;
                    }

                    world.spawn((
                        transform,
                        object,
                        collider,
                        RigidBody {
                            kind: RigidBodyKind::Fixed,
                            linvel: Vec3::ZERO,
                            angvel: Vec3::ZERO,
                        },
                        Root,
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

#[derive(Debug)]
enum SpawnEvent {
    SpawnEntity(SpawnEntity),
    SpawnTerrain(Transform, RenderObject, Collider),
}

#[derive(Debug)]
pub struct SpawnEntity {
    pub transform: NiTransform,
    pub scene: Option<Handle<NifScene>>,
    pub light: Option<PointLight>,
    pub light_flicker: Option<LightFlicker>,
    pub actor: Option<Actor>,
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

            let mut light_mod = LightModification::default();

            match record.data {
                RecordData::PlacedObject(object) => {
                    position = object.position;
                    base_object = object.base_object;
                    scale = object.scale.unwrap_or(1.0);

                    if let Some(radius) = object.radius {
                        light_mod.radius = radius;
                    }

                    if let Some(light) = object.light_data {
                        light_mod.fov = light.fov_offset_from_base_object;
                        light_mod.fade = light.fade_offset_from_base_ojbect;
                    }
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
                            Vec3::new(x, y, z) - LAND_OFFSET
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
                        layer: CollisionLayer::STATIC,
                    };

                    events.push(SpawnEvent::SpawnTerrain(
                        Transform::from(NiTransform::from_translation(
                            offset.extend(0.0) + LAND_OFFSET,
                        )),
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

            let mut model = None;
            let mut light = None;
            let mut light_flicker = None;
            let mut actor = None;

            match base.data {
                RecordData::STAT(data) => {
                    model = data.modl.model;
                }
                RecordData::StaticCollection(data) => {
                    model = data.model.model;
                }
                RecordData::MSTT(data) => {
                    model = data.model.model;
                }
                RecordData::DOOR(data) => {
                    model = data.modl.model;
                }
                RecordData::Container(data) => {
                    model = data.modl.model;
                }
                RecordData::Npc(_) => {
                    actor = crate::state::npc::spawn_actor(&asset_loader, &loader, base.id)
                        .await
                        .ok();
                }
                RecordData::IdleMarker(_) => return,
                RecordData::Activator(data) => {
                    model = data.model.model;
                }
                RecordData::MiscObject(data) => {
                    model = data.model.model;
                }
                RecordData::Furniture(data) => {
                    model = data.model.model;
                }
                RecordData::Light(data) => {
                    model = data.model.model;

                    let intensity = (data.fade_value + light_mod.fade) * 10.0;
                    let radius = (data.data.radius as f32 + light_mod.radius) * NIF_TO_METERS;

                    light = Some(PointLight {
                        color: Color::from_rgb([
                            data.data.color.r as f32 / 255.0,
                            data.data.color.g as f32 / 255.0,
                            data.data.color.b as f32 / 255.0,
                        ]),
                        intensity,
                        radius,
                    });

                    let is_pulsing = data.data.flags.contains(LightDataFlags::PULSE);
                    let is_flicking = data.data.flags.contains(LightDataFlags::FLICKER);

                    if is_pulsing || is_flicking {
                        let mode = if is_pulsing {
                            LightFlickerMode::Pulse
                        } else {
                            debug_assert!(is_flicking);
                            LightFlickerMode::Flicker
                        };

                        let period = Duration::from_secs_f32(1.0 / data.data.flicker_period);

                        light_flicker = Some(LightFlicker {
                            mode,
                            period,
                            intensity,
                            intensity_amplitude: data.data.flicker_intensity_amplitude,
                            state: Default::default(),
                        });
                    }
                }
                RecordData::Ingestible(data) => {
                    model = data.model.model;
                }
                _ => return,
                x => todo!("{:?}", x),
            }

            let scene = match model {
                Some(model) => {
                    let path = format!("meshes/{}", model.as_str().unwrap());
                    match asset_loader.load::<NifScene, _>(&path).await {
                        Ok(scene) => Some(scene),
                        Err(err) => {
                            tracing::error!("{}", err);
                            None
                        }
                    }
                }
                None => None,
            };

            events.push(SpawnEvent::SpawnEntity(SpawnEntity {
                transform: NiTransform::from_ni_transform(transform),
                scene,
                light,
                light_flicker,
                actor,
            }));
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

#[derive(Copy, Clone, Debug, Default)]
struct LightModification {
    radius: f32,
    fov: f32,
    fade: f32,
}
