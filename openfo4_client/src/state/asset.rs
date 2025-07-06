use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::Arc;
use std::vec;

use bytes::Bytes;
use game_common::components::{Color, Transform};
use game_core::hierarchy::Hierarchy;
use game_core_pipeline::entities::{ImageHandle, MaterialHandle};
use game_core_pipeline::material::StandardMaterial;
use game_render::backend::TextureFormat;
use game_render::mesh::{Indices, Mesh};
use game_render::texture::Image;
use game_render::texture::image::MipImage;
use glam::{Mat3, Quat, Vec2, Vec3, Vec4};
use hecs::{Entity, EntityBuilder, World};
use image::{ImageBuffer, Rgba};
use openfo4_core::hierarchy::Children;
use openfo4_dds::Format;
use openfo4_hkx::{HkObject, HkxFile};
use openfo4_nif::bgsm::Bgsm;
use openfo4_nif::mesh::{BSMeshLODTriShape, BSSubIndexTriShape, BSTriShape};
use openfo4_nif::{BSXFlags, Block, BlockRef, Nif};
use openfo4_physics::components::{Collider, ColliderShape, RigidBody, RigidBodyKind, TriMesh};
use parking_lot::RwLock;

use crate::components::{NiTransform, Root};
use crate::loader::{FileLoader, fix_path};
use crate::render::RenderObject;
use crate::shared::SharedState;

pub const NIF_TO_METERS: f32 = 1.0 / 69.99125;
pub const NIF_TO_OPENGL: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => -Z
    [0.0, 0.0, -1.0],
    // Z => Y
    [0.0, 1.0, 0.0],
]);

#[derive(Debug)]
pub struct NifLoader {
    file_loader: Arc<FileLoader>,
    state: Arc<SharedState>,
    default_material: MaterialHandle,
    scenes: Arc<RwLock<HashMap<String, UntrackedNifScene>>>,
}

impl NifLoader {
    pub fn new(file_loader: Arc<FileLoader>, state: Arc<SharedState>) -> Self {
        let default_material = state.entities.create_material(StandardMaterial::default());

        Self {
            file_loader,
            state,
            default_material,
            scenes: Arc::default(),
        }
    }

    pub async fn load(&self, path: &str) -> Option<NifScene> {
        // `meshes/markers` contains editor markers.
        // There does not seem any other indication that a
        // file/object is a marker.
        // Some markers are not inside this directory.
        let path = fix_path(path);
        if path.starts_with("meshes/markers/") {
            tracing::info!("skip {}", path);
            return None;
        }

        {
            // TODO: We must remove elements from the scene cache if
            // all instances of that scene are destroyed.
            if let Some(scene) = self.scenes.read().get(&path) {
                return Some(NifScene {
                    handle: Some(NifHandle(Arc::new(NifHandleInner {
                        key: path,
                        scenes: self.scenes.clone(),
                    }))),
                    nodes: scene.nodes.clone(),
                });
            }
        }

        let mut reader = self.file_loader.open(&path).unwrap();

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        let nif = match Nif::decode(&buf[..]) {
            Ok(nif) => nif,
            Err(err) => {
                tracing::error!("failed to load nif: {:?}", err);
                return None;
            }
        };

        let Some(root) = nif.root() else {
            return None;
        };

        let mut queue = VecDeque::new();
        queue.push_back((None, root));

        let mut contains_editor_marker = true;
        let Some(root) = nif.root() else {
            return None;
        };

        match nif.get(root) {
            Some(Block::NiNode(node)) => {
                for block in &node.extra_data {
                    match nif.get(*block) {
                        Some(Block::BSXFlags(flags)) => {
                            contains_editor_marker = flags.contains(BSXFlags::EDITOR_MARKER);
                            break;
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }

        let is_marker = |name: &str| -> bool {
            let name = name.to_lowercase();
            // contains_editor_marker &&
            (name.starts_with("marker") || name.starts_with("editormarker"))
        };

        let mut meshes = HashMap::new();
        let mut materials = HashMap::new();

        let mut nodes = Hierarchy::new();
        let mut block_to_node = HashMap::new();

        while let Some((parent, block)) = queue.pop_front() {
            let parent = parent.map(|parent| *block_to_node.get(&parent).unwrap());

            // Always setup the node to be a root node
            // regardless of the position in the hierarchy.
            let mut set_root_node = false;

            let node = match nif.get(block) {
                Some(Block::NiNode(node)) => {
                    if is_marker(&node.name) {
                        continue;
                    }

                    if let Some(collision_object) = node.collision_object {
                        queue.push_back((Some(block), collision_object));
                    }

                    let node_transform = Transform {
                        translation: node.translation,
                        rotation: Quat::from_mat3(&node.rotation).normalize(),
                        scale: Vec3::splat(node.scale),
                    };

                    // One would really expected that the parent transform
                    // affects the children, but this is not the case?
                    for children in &node.children {
                        // queue.push_back((*children, transform * node_transform));
                        queue.push_back((Some(block), *children));
                    }

                    Node {
                        name: node.name.clone(),
                        transform: node_transform,
                        ..Default::default()
                    }
                }
                Some(Block::BSTriShape(node))
                | Some(Block::BSSubIndexTriShape(BSSubIndexTriShape { shape: node }))
                | Some(Block::BSMeshLODTriShape(BSMeshLODTriShape { shape: node })) => {
                    if is_marker(&node.name) {
                        continue;
                    }

                    if let Some(collision_object) = node.collision_object {
                        queue.push_back((Some(block), collision_object));
                    }

                    let transform = Transform {
                        translation: node.translation,
                        rotation: Quat::from_mat3(&node.rotation).normalize(),
                        scale: Vec3::splat(node.scale),
                    };

                    let mesh = meshes.entry(block).or_insert_with(|| {
                        let mesh = build_mesh(node);
                        self.state.entities.create_mesh(mesh)
                    });

                    let material = match node.shader_property {
                        Some(shader_property) => {
                            if materials.get(&shader_property).is_none() {
                                let material = load_material(
                                    &self.file_loader,
                                    &self.state,
                                    &nif,
                                    shader_property,
                                )
                                .await;
                                materials.insert(shader_property, material);
                            }

                            materials.get(&shader_property).unwrap()
                        }
                        None => &self.default_material,
                    };

                    let skin = match node.skin {
                        Some(skin) => match nif.get(skin) {
                            Some(Block::BSSkinInstance(skin)) => {
                                let bone_data = match nif.get(skin.data) {
                                    Some(Block::BSSkinBoneData(data)) => data
                                        .bones
                                        .iter()
                                        .map(|bone| Transform {
                                            translation: bone.translation,
                                            rotation: Quat::from_mat3(&bone.rotation).conjugate(),
                                            scale: Vec3::splat(bone.scale),
                                        })
                                        .collect::<Vec<_>>(),
                                    _ => todo!(),
                                };

                                let mut bones = Vec::new();
                                let mut bone_transforms = Vec::new();

                                for bone in &skin.bones {
                                    match nif.get(*bone) {
                                        Some(Block::NiNode(node)) => {
                                            bones.push(node.name.clone());
                                            bone_transforms.push(Transform {
                                                translation: node.translation,
                                                rotation: Quat::from_mat3(&node.rotation)
                                                    .conjugate(),
                                                scale: Vec3::splat(node.scale),
                                            });
                                        }
                                        _ => todo!(),
                                    }
                                }

                                let Some(skinning) = &node.mesh.skinning else {
                                    todo!();
                                };

                                let weights: Vec<_> =
                                    skinning.bone_weights.iter().map(|v| v.as_vec4()).collect();
                                let indices: Vec<_> =
                                    skinning.bone_indices.iter().map(|v| *v).collect();

                                let mut mesh = build_mesh(node);

                                let mut positions = vec![Vec3::ZERO; mesh.positions().len()];

                                for i in 0..weights.len() {
                                    let weights = weights[i];
                                    let indices = indices[i];
                                    let pos = mesh.positions()[i];

                                    let i0 = indices[0] as usize;
                                    let i1 = indices[1] as usize;
                                    let i2 = indices[2] as usize;
                                    let i3 = indices[3] as usize;

                                    let x =
                                        (bone_transforms[i0] * bone_data[i0]).transform_point(pos);
                                    let y =
                                        (bone_transforms[i1] * bone_data[i1]).transform_point(pos);
                                    let z =
                                        (bone_transforms[i2] * bone_data[i2]).transform_point(pos);
                                    let w =
                                        (bone_transforms[i3] * bone_data[i3]).transform_point(pos);

                                    positions[i] = x * weights.x
                                        + y * weights.y
                                        + z * weights.z
                                        + w * weights.w;
                                }

                                mesh.set_positions(positions);

                                Some(Skin {
                                    bones,
                                    mesh,
                                    weights,
                                    indices,
                                })
                            }
                            _ => todo!(),
                        },
                        None => None,
                    };

                    set_root_node = true;
                    Node {
                        name: node.name.clone(),
                        transform,
                        renderable: Some(RenderObject {
                            mesh: mesh.clone(),
                            material: material.clone(),
                        }),
                        skin,
                        ..Default::default()
                    }
                }
                Some(Block::BhkNPCollisionObject(_)) => {
                    if let Some(collider) = load_collider(&nif, block) {
                        // instances.push(Instance {
                        //     transform: instance.transform
                        //         * NiTransform::from_ni_transform(
                        //             Transform::from_scale(Vec3::splat(1.0 / NIF_TO_METERS))
                        //                 * collider.transform,
                        //         ),
                        //     collider: Some(collider.collider),
                        //     skin: None,
                        //     renderable: None,
                        // });

                        Node {
                            transform: Transform::from_scale(Vec3::splat(1.0 / NIF_TO_METERS))
                                * collider.transform,
                            collider: Some(collider.collider),
                            ..Default::default()
                        }
                    } else {
                        Node::default()
                    }
                }
                _ => Node::default(),
            };

            // If `set_root_node` is `true` we must not set
            // a parent.
            let parent = (!set_root_node).then_some(parent).flatten();

            let key = nodes.append(parent, node);
            block_to_node.insert(block, key);
        }

        self.scenes.write().insert(
            path.to_owned(),
            UntrackedNifScene {
                nodes: nodes.clone(),
            },
        );

        let scene = NifScene {
            handle: Some(NifHandle(Arc::new(NifHandleInner {
                key: path,
                scenes: self.scenes.clone(),
            }))),
            nodes,
        };

        Some(scene)
    }
}

#[derive(Clone, Debug)]
struct UntrackedNifScene {
    nodes: Hierarchy<Node>,
}

#[derive(Clone, Debug)]
pub struct NifScene {
    pub handle: Option<NifHandle>,
    pub nodes: Hierarchy<Node>,
}

impl NifScene {
    pub fn instantiate(&self, transform: NiTransform, world: &mut World) -> Entity {
        self.instantiate_with_feedback(|_, _| {}, transform, world)
    }

    pub fn instantiate_with_feedback<F>(
        &self,
        mut callback: F,
        transform: NiTransform,
        world: &mut World,
    ) -> Entity
    where
        F: FnMut(&Node, Entity),
    {
        let mut entities = HashMap::new();
        let mut queue = Vec::new();

        for (key, _) in self.nodes.iter() {
            let entity = world.reserve_entity();
            entities.insert(key, entity);
            queue.push(key);
        }

        while let Some(key) = queue.pop() {
            let node = self.nodes.get(key).unwrap();

            let mut builder = EntityBuilder::new();

            builder.add(node.transform);
            builder.add(RigidBody {
                kind: RigidBodyKind::Fixed,
                linvel: Vec3::ZERO,
                angvel: Vec3::ZERO,
            });

            if let Some(renderable) = &node.renderable {
                builder.add(renderable.clone());
            }

            if let Some(collider) = &node.collider {
                builder.add(collider.clone());
            }

            if let Some(children) = self.nodes.children(key) {
                let children = children.map(|(k, _)| *entities.get(&k).unwrap()).collect();
                builder.add(Children(children));
            }

            let entity = *entities.get(&key).unwrap();
            world.insert(entity, builder.build()).unwrap();

            callback(node, entity);
        }

        let root = self
            .nodes
            .root_keys()
            .map(|key| *entities.get(&key).unwrap())
            .collect();

        let root = world.spawn((transform, Children(root), Root));

        if let Some(handle) = &self.handle {
            // This will keep the scene cached until the root entity
            // is destroyed.
            world.insert_one(root, handle.clone()).unwrap();
        }

        root
    }
}

#[derive(Debug)]
struct NifHandleInner {
    key: String,
    scenes: Arc<RwLock<HashMap<String, UntrackedNifScene>>>,
}

impl Drop for NifHandleInner {
    fn drop(&mut self) {
        let mut scenes = self.scenes.write();
        scenes.remove(&self.key);
    }
}

/// A handle to the Nif scene that will keep it alive.
#[derive(Clone, Debug)]
pub struct NifHandle(Arc<NifHandleInner>);

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub renderable: Option<RenderObject>,
    pub collider: Option<Collider>,
    pub skin: Option<Skin>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum NodeTransform {
    Transform(Transform),
    NiTransform(NiTransform),
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub transform: NiTransform,
    pub renderable: Option<RenderObject>,
    pub collider: Option<Collider>,
    pub skin: Option<Skin>,
}

#[derive(Clone, Debug)]
pub struct Skin {
    pub bones: Vec<String>,
    pub mesh: Mesh,
    pub weights: Vec<Vec4>,
    pub indices: Vec<[u8; 4]>,
}

async fn load_material(
    loader: &FileLoader,
    shared: &SharedState,
    nif: &Nif,
    block: BlockRef,
) -> MaterialHandle {
    match nif.get(block) {
        // If the name field is not empty we need to load from BGSM file.
        Some(Block::BSLightingShaderProperty(shader_property))
            if !shader_property.name.is_empty() =>
        {
            if let Some(handle) = shared
                .materials_map
                .read()
                .unwrap()
                .get(&shader_property.name)
            {
                return handle.clone();
            }

            let mut buf = Vec::new();
            loader
                .open(&shader_property.name)
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            let bgsm = Bgsm::decode(&buf[..]).unwrap();

            let mut material = StandardMaterial {
                base_color: Color::from_rgba([1.0, 1.0, 1.0, bgsm.alpha]),
                flip_normal_y: true,
                two_component_normal_encoding: true,
                roughness: 1.0 - bgsm.smoothness,
                specular_color: Color::from_rgb(bgsm.specular_color.to_array()),
                specular_strength: bgsm.specular_strength,
                ..Default::default()
            };

            if !bgsm.diffuse_texture.is_empty() {
                material.base_color_texture =
                    load_texture(loader, shared, bgsm.diffuse_texture, false).await;
            }

            if !bgsm.normal_texture.is_empty() {
                material.normal_texture =
                    load_texture(loader, shared, bgsm.normal_texture, false).await;
            }

            if !bgsm.smooth_spec_texture.is_empty() {
                material.specular_glossiness_texture =
                    load_texture(loader, shared, bgsm.smooth_spec_texture, false).await;
            }

            if let Some(grayscale_palette) = bgsm.grayscale_palette {
                let texture =
                    load_texture(loader, shared, grayscale_palette.grayscale_texture, false).await;
                material.grayscale = texture;
            }

            let handle = shared.entities.create_material(material);
            shared
                .materials_map
                .write()
                .unwrap()
                .insert(shader_property.name.clone(), handle.clone());
            handle
        }
        _ => shared.entities.create_material(StandardMaterial::default()),
    }
}

pub async fn load_texture(
    loader: &FileLoader,
    shared: &SharedState,
    mut path: String,
    srgb: bool,
) -> Option<ImageHandle> {
    if !path.starts_with("textures") {
        path = format!("textures/{}", path);
    }

    if let Some(handle) = shared.images_map.read().unwrap().get(&path) {
        return Some(handle.clone());
    }

    let mut reader = match loader.open(&path) {
        Ok(reader) => reader,
        Err(err) => {
            tracing::error!("failed to load texture from path {}: {}", path, err);
            return None;
        }
    };

    let mut buf = Vec::new();
    if let Err(err) = reader.read_to_end(&mut buf) {
        tracing::error!("failed to read from buffer: {}", err);
        return None;
    }

    let buf = Bytes::from(buf);
    if path.to_lowercase().ends_with(".dds") {
        let image = openfo4_dds::Image::parse(&buf).unwrap();

        let format = match (image.format, srgb) {
            (Format::Dxt1, false) => TextureFormat::Bc1RgbaUnorm,
            (Format::Dxt1, true) => TextureFormat::Bc1RgbaUnormSrgb,
            (Format::Dxt2, false) => TextureFormat::Bc2RgbaUnorm,
            (Format::Dxt2, true) => TextureFormat::Bc2RgbaUnormSrgb,
            (Format::Dxt3, false) => TextureFormat::Bc2RgbaUnorm,
            (Format::Dxt3, true) => TextureFormat::Bc2RgbaUnormSrgb,
            (Format::Dxt4, false) => TextureFormat::Bc3RgbaUnorm,
            (Format::Dxt4, true) => TextureFormat::Bc3RgbaUnormSrgb,
            (Format::Dxt5, false) => TextureFormat::Bc3RgbaUnorm,
            (Format::Dxt5, true) => TextureFormat::Bc3RgbaUnormSrgb,
            (Format::Bc5U, _) => TextureFormat::Bc5RgUnorm,
            (Format::Rgb8, true) => TextureFormat::Rgb8UnormSrgb,
            (Format::Rgb8, false) => TextureFormat::Rgb8Unorm,
            (Format::Rgba8, true) => TextureFormat::Rgba8UnormSrgb,
            (Format::Rgba8, false) => TextureFormat::Rgba8Unorm,
        };

        let mips = image
            .mips
            .into_iter()
            .map(|mip| Image::new(mip.size, format, mip.bytes))
            .collect();

        let handle = shared.entities.create_image(MipImage::new(mips));
        shared
            .images_map
            .write()
            .unwrap()
            .insert(path, handle.clone());

        Some(handle)
    } else {
        tracing::error!("unknown file extension: {}", path);
        None
    }
}

pub async fn load_image_buffer(
    loader: &FileLoader,
    mut path: String,
) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    if !path.starts_with("textures") {
        path = format!("textures/{}", path);
    }

    let mut reader = match loader.open(&path) {
        Ok(reader) => reader,
        Err(err) => {
            tracing::error!("failed to load texture: {}", err);
            return None;
        }
    };

    let mut buf = Vec::new();
    if let Err(err) = reader.read_to_end(&mut buf) {
        tracing::error!("failed to read from buffer: {}", err);
        return None;
    }

    Some(image::load_from_memory(&buf).unwrap().to_rgba8())
}

fn build_mesh(shape: &BSTriShape) -> Mesh {
    let mut mesh = Mesh::new();

    if let Some(positions) = &shape.mesh.positions {
        mesh.set_positions(positions.clone());
    }

    if let Some(normals) = &shape.mesh.normals {
        mesh.set_normals(normals.clone());
    }

    if let Some(tex_coords) = &shape.mesh.tex_coords {
        let tex_coords = tex_coords.iter().map(|v| v.as_vec2()).collect();
        mesh.set_uvs(tex_coords);
    } else {
        // We need tex coords to be able to render.
        let tex_coords = std::iter::repeat_n(Vec2::ZERO, mesh.positions().len()).collect();
        mesh.set_uvs(tex_coords);
    }

    if let Some(colors) = &shape.mesh.colors {
        let colors = colors
            .iter()
            .map(|v| Vec4::new(v.r as f32, v.g as f32, v.b as f32, v.a as f32) / 255.0)
            .collect();
        mesh.set_colors(colors);
    }

    let indices = shape
        .mesh
        .indices
        .iter()
        .flatten()
        .map(|v| *v as u32)
        .collect();
    mesh.set_indices(Indices::U32(indices));

    if let Some(tangents) = &shape.mesh.tangents {
        let tangents = tangents.iter().map(|v| v.extend(1.0)).collect();
        mesh.set_tangents(tangents);
    } else {
        mesh.compute_tangents();
    }

    mesh
}

#[derive(Clone, Debug)]
struct ColliderNode {
    transform: Transform,
    collider: Collider,
}

fn load_collider(nif: &Nif, collision_object: BlockRef) -> Option<ColliderNode> {
    // The collision object is a `bhkNPCollisionObject` node, which
    // contains a reference to a `bhkPhysicsSystem` node that contains
    // a HKX file.
    // The `BodyID` of the `bhkNPCollisionObject` node contains the index
    // of the body that should be selected from the HKX file.
    let collision_object = match nif.get(collision_object) {
        Some(Block::BhkNPCollisionObject(object)) => object,
        _ => todo!(),
    };

    let data = match nif.get(collision_object.data) {
        Some(Block::BhkPhysicsSystem(data)) => &data.data,
        Some(Block::BhkRagdollSystem(data)) => &data.data,
        _ => todo!(),
    };

    let hkx = match HkxFile::decode(data) {
        Ok(f) => f,
        Err(err) => {
            tracing::error!("invalid hkx data: {}", err);
            return None;
        }
    };

    let system = hkx
        .iter()
        .find_map(|obj| match obj {
            HkObject::PhysicsSystemData(data) => Some(data),
            HkObject::RagdollData(data) => Some(&data.physics_system),
            _ => None,
        })
        .unwrap();

    let body = system
        .body_cinfos
        .get(collision_object.body_id as usize)
        .unwrap();

    // Note that unlike everything else in a NIF file,
    // the hkx transforms/collider meshes use meters.
    let mut transform = Transform {
        // There are position and orientation attributes on the body,
        // but they seem to not be applied?
        ..Default::default()
    };

    let mesh = match hkx.get(body.shape) {
        Some(HkObject::ConvexPolytopeShape(shape)) => shape.to_tri_mesh(),
        Some(HkObject::CompressedMeshShape(shape)) => shape.data.mesh(),
        Some(HkObject::ScaledConvexShape(shape)) => {
            // FIXME: Which of these two is applied first,
            // or rather the question is whether translation
            // should already be scaled by the new scale value.
            transform.translation += shape.translation.truncate();
            transform.scale *= shape.scale.truncate();

            match hkx.get(shape.core_shape) {
                Some(HkObject::ConvexPolytopeShape(shape)) => shape.to_tri_mesh(),
                _ => todo!(),
            }
        }
        _ => {
            tracing::error!("unknown mesh shape");
            return None;
        }
    };

    if mesh.indices.is_empty() {
        return None;
    }

    Some(ColliderNode {
        transform,
        collider: Collider {
            // TODO: Load values from material property.
            friction: 1.0,
            restitution: 1.0,
            shape: ColliderShape::TriMesh(TriMesh::new(mesh.vertices, mesh.indices)),
        },
    })
}
