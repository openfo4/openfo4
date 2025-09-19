use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;

use game_common::components::{Color, Transform};
use game_core::hierarchy::Hierarchy;
use game_core_pipeline::entities::{Entities, MaterialHandle};
use game_core_pipeline::material::{AlphaMode, StandardMaterial};
use game_render::mesh::{Indices, Mesh};
use glam::{Quat, Vec2, Vec3, Vec4};
use hashbrown::HashSet;
use hecs::{Entity, EntityBuilder, World};
use openfo4_asset::{Asset, AssetLoader, Handle, HandleOrAsset};
use openfo4_core::hierarchy::Children;
use openfo4_hkx::{HkObject, HkxFile};
use openfo4_nif::mesh::{BSMeshLODTriShape, BSSubIndexTriShape, BSTriShape};
use openfo4_nif::{BSXFlags, Block, BlockRef, Nif};
use openfo4_physics::components::{Collider, ColliderShape, RigidBody, RigidBodyKind, TriMesh};

use crate::assets::material::BgsmMaterial;
use crate::assets::texture::FileImageHandle;
use crate::assets::{AssetServer, Error};
use crate::components::Root;
use crate::loader::fix_path;
use crate::render::RenderObject;

pub const NIF_TO_METERS: f32 = 1.0 / 69.99125;

#[derive(Clone, Debug)]
pub struct NifScene {
    pub nodes: Hierarchy<Node>,
    /// List of materials that are referenced by the scene.
    ///
    /// This exists to keep all the handles alive while this `NifScene` is alive.
    _materials: Vec<Handle<BgsmMaterial>>,
    /// List of textures that are referenced by the scene.
    ///
    /// This exists to keep all the handles alive while this `NifScene` is alive to prevent loading
    /// the same textures into memory twice.
    _textures: Vec<Handle<FileImageHandle>>,
}

// Note that this is a trait because we wan't to implement
// this on `Handle<NifScene>`, but we cannot do this directly
// because of the orphan rule. If custom pointer receivers (akin to `Arc`)
// every become stable we can use `fn instantiate(self: Handle<Self>, ..)`
// instead.
pub trait Instantiate {
    fn instantiate(&self, transform: Transform, world: &mut World) -> Entity;

    fn instantiate_with_feedback<'a, F>(
        &'a self,
        callback: F,
        transform: Transform,
        world: &mut World,
    ) -> Entity
    where
        F: FnMut(&'a Node, Entity);
}

impl Instantiate for Handle<NifScene> {
    fn instantiate(&self, transform: Transform, world: &mut World) -> Entity {
        self.instantiate_with_feedback(|_, _| {}, transform, world)
    }

    fn instantiate_with_feedback<'a, F>(
        &'a self,
        mut callback: F,
        transform: Transform,
        world: &mut World,
    ) -> Entity
    where
        F: FnMut(&'a Node, Entity),
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

        // This will keep the scene cached until the root entity
        // is destroyed.
        let handle: Handle<NifScene> = self.clone();
        world.insert_one(root, handle.clone()).unwrap();

        root
    }
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub renderable: Option<RenderObject>,
    pub collider: Option<Collider>,
    pub skin: Option<Skin>,
}

impl Node {
    pub fn build(&self) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        builder.add(self.transform);

        if let Some(renderable) = &self.renderable {
            builder.add(renderable.clone());
        }

        if let Some(collider) = &self.collider {
            builder.add(collider.clone());
        }

        builder
    }
}

#[derive(Clone, Debug)]
pub struct Skin {
    pub bones: Vec<String>,
    pub mesh: Mesh,
    pub weights: Vec<Vec4>,
    pub indices: Vec<[u8; 4]>,
}

#[derive(Debug)]
pub struct NifLoaderContext {
    pub server: AssetServer,
    pub entities: Entities,
    /// The material used if no other material is specified.
    pub default_material: MaterialHandle,
}

impl NifLoaderContext {
    pub fn new(server: AssetServer, entities: Entities) -> Self {
        let default_material = entities.create_material(StandardMaterial::default());

        Self {
            server,
            entities,
            default_material,
        }
    }
}

impl Asset for NifScene {
    type Key = String;
    type Error = Error;
    type Context = NifLoaderContext;

    #[tracing::instrument(name = "NifScene::load", skip_all)]
    async fn load(
        loader: &AssetLoader,
        path: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        // `meshes/markers` contains editor markers.
        // There does not seem any other indication that a
        // file/object is a marker.
        // Some markers are not inside this directory.
        let path = fix_path(path);
        if path.starts_with("meshes/markers/") {
            tracing::info!("skip {}", path);

            return Ok(Self {
                nodes: Hierarchy::new(),
                _materials: Vec::new(),
                _textures: Vec::new(),
            });
        }

        let nif = ctx
            .server
            .load::<Nif>(&path)
            .await
            .map_err(|err| Error::Other(format!("failed to read file {}: {}", path, err)))?;

        let Some(root) = nif.root() else {
            // Nif file is empty, return an empty scene.
            return Ok(Self {
                nodes: Hierarchy::new(),
                _materials: Vec::new(),
                _textures: Vec::new(),
            });
        };

        let mut queue = VecDeque::new();
        queue.push_back((None, root));

        let mut contains_editor_marker = true;
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

        let mut material_handles = Vec::new();
        let mut texture_handles = Vec::new();

        while let Some((parent, block)) = queue.pop_front() {
            let parent = parent.map(|parent| *block_to_node.get(&parent).unwrap());

            let node = match nif.get(block) {
                Some(Block::NiNode(node)) => {
                    if is_marker(&node.name) {
                        continue;
                    }

                    if let Some(collision_object) = node.collision_object {
                        queue.push_back((Some(block), collision_object));
                    }

                    let mut node_transform = Transform {
                        translation: node.translation,
                        rotation: Quat::from_mat3(&node.rotation).normalize(),
                        scale: Vec3::splat(node.scale),
                    };

                    // For some scenes the transform of the root node breaks the
                    // transform exactly by the root transform, so maybe the transform
                    // of the root node is not applied?
                    if parent.is_none() {
                        node_transform = Transform::default();
                    }

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

                    let mut transform = Transform {
                        translation: node.translation,
                        rotation: Quat::from_mat3(&node.rotation).normalize(),
                        scale: Vec3::splat(node.scale),
                    };
                    // let transform = Transform::default();

                    // let mut parent = parent;
                    // let mut parent_transform = Transform::default();
                    // while let Some(p) = parent {
                    //     let parent_node: &Node = nodes.get(p).unwrap();
                    //     parent_transform = parent_node.transform * parent_transform;
                    //     parent = nodes.parent_key(p);
                    // }

                    // dbg!(&parent_transform);

                    // transform = Transform::from_matrix(
                    //     parent_transform.compute_matrix().inverse() * transform.compute_matrix(),
                    // );

                    let mesh = meshes.entry(block).or_insert_with(|| {
                        let mesh = build_mesh(node);
                        ctx.entities.create_mesh(mesh)
                    });

                    let material = match node.shader_property {
                        Some(shader_property) => {
                            if materials.get(&shader_property).is_none() {
                                let material =
                                    load_material(loader, &ctx.entities, &nif, shader_property)
                                        .await
                                        .map_err(|err| {
                                            Error::Other(format!(
                                                "invalid material in nif file {}: {}",
                                                path, err
                                            ))
                                        })?;

                                match &material {
                                    HandleOrAsset::Asset(material) => {
                                        texture_handles.extend(material.textures.clone());
                                    }
                                    HandleOrAsset::Handle(handle) => {
                                        material_handles.push(handle.clone());
                                    }
                                }

                                materials.insert(shader_property, material.material.clone());
                            }

                            materials.get(&shader_property).unwrap()
                        }
                        None => &ctx.default_material,
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

            let key = nodes.append(parent, node);
            block_to_node.insert(block, key);
        }

        Ok(Self {
            nodes,
            _materials: material_handles,
            _textures: texture_handles,
        })
    }
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

async fn load_material(
    loader: &AssetLoader,
    entities: &Entities,
    nif: &Nif,
    block: BlockRef,
) -> Result<HandleOrAsset<BgsmMaterial>, Error> {
    match nif.get(block) {
        // If the name field is not empty we need to load from BGSM file.
        Some(Block::BSLightingShaderProperty(shader_property))
            if !shader_property.name.is_empty() =>
        {
            let material = loader
                .load::<BgsmMaterial, _>(&shader_property.name)
                .await?;

            Ok(HandleOrAsset::Handle(material))
        }
        Some(Block::BSLightingShaderProperty(shader_property)) => {
            let mut material = StandardMaterial {
                base_color: Color::from_rgba([1.0, 1.0, 1.0, shader_property.alpha]),
                flip_normal_y: true,
                two_component_normal_encoding: true,
                roughness: 1.0 - shader_property.smoothness,
                specular_color: Color::from_rgb(shader_property.specular_color.to_array()),
                specular_strength: shader_property.specular_strength,
                ..Default::default()
            };

            let mut textures = Vec::new();

            if let Some(texture_set) =
                shader_property
                    .texture_set
                    .and_then(|set| match nif.get(set) {
                        Some(Block::BSShaderTextureSet(set)) => Some(set),
                        _ => None,
                    })
            {
                // Texture order is as follows:
                // 0. Diffuse
                // 1. Normal
                // 2. ?
                // 3. ?
                // 4. Environment
                // 5. ?
                // 6. ?
                // 7. Specular Glossiness
                // 8. ?
                // 9. ?

                if let Some(path) = texture_set.textures.get(0) {
                    if !path.is_empty() {
                        let handle = loader.load::<FileImageHandle, _>(path).await?;

                        material.base_color_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(1) {
                    if !path.is_empty() {
                        let handle = loader.load::<FileImageHandle, _>(path).await?;

                        material.normal_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(7) {
                    if !path.is_empty() {
                        let handle = loader.load::<FileImageHandle, _>(path).await?;

                        material.specular_glossiness_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }
            }

            let material = entities.create_material(material);
            Ok(HandleOrAsset::Asset(BgsmMaterial { material, textures }))
        }
        Some(Block::BSEffectShaderProperty(property)) => {
            let mut textures = Vec::new();
            let mut material = StandardMaterial {
                alpha_mode: AlphaMode::Mask,
                base_color: Color([
                    property.base_color.r * property.base_color_scale,
                    property.base_color.g * property.base_color_scale,
                    property.base_color.b * property.base_color_scale,
                    property.base_color.a * property.base_color_scale,
                ]),
                ..Default::default()
            };

            if !property.source_texture.is_empty() {
                let handle = loader
                    .load::<FileImageHandle, _>(&property.source_texture)
                    .await?;

                material.base_color_texture = Some(handle.0.clone());
                textures.push(handle);
            }

            let material = entities.create_material(material);
            Ok(HandleOrAsset::Asset(BgsmMaterial { material, textures }))
        }
        _ => Ok(HandleOrAsset::Asset(BgsmMaterial {
            material: entities.create_material(StandardMaterial::default()),
            textures: Vec::new(),
        })),
    }
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

    let Some(material) = system.materials.get(usize::from(body.material_id)) else {
        tracing::error!("invalid hkx data: invalid material id {}", body.material_id);
        return None;
    };

    let friction = material.static_friction.to_f32();
    let restitution = material.restitution.to_f32();

    // Note that unlike everything else in a NIF file,
    // the hkx transforms/collider meshes use meters.
    let transform = Transform {
        // There are position and orientation attributes on the body,
        // but they seem to not be applied?
        ..Default::default()
    };

    let mut meshes = Vec::new();
    let mut all_meshes_are_convex = true;

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((body.shape, Transform::default()));

    while let Some((ptr, mut transform)) = queue.pop_front() {
        // Since composite shapes can point to any other shape
        // we need to catch cycles we may get stuck here forever.
        if !visited.insert(ptr) {
            tracing::error!("cycle in collider pointers");
            return None;
        }

        match hkx.get(ptr) {
            Some(HkObject::CompressedMeshShape(shape)) => {
                let mesh = shape.data.mesh();
                meshes.push((mesh, transform));
                all_meshes_are_convex = false;
            }
            Some(HkObject::ConvexPolytopeShape(shape)) => {
                let mesh = shape.to_tri_mesh();
                meshes.push((mesh, transform));
            }
            Some(HkObject::CapsuleShape(shape)) => {
                let mesh = shape.shape.to_tri_mesh();
                meshes.push((mesh, transform));
            }
            Some(HkObject::ConvexShape(shape)) => {
                let mesh = shape.mesh();
                meshes.push((mesh, transform));
            }
            Some(HkObject::ScaledConvexShape(shape)) => {
                // FIXME: Which of these two is applied first,
                // or rather the question is whether translation
                // should already be scaled by the new scale value.
                transform.translation += shape.translation.truncate();
                transform.scale *= shape.scale.truncate();

                queue.push_back((shape.core_shape, transform));
            }
            Some(HkObject::DynamicCompoundShape(shape)) => {
                for instance in &*shape.instances {
                    let Some(shape) = instance.shape else {
                        continue;
                    };

                    let (_, rotation, translation) =
                        instance.transform.to_scale_rotation_translation();
                    let scale = instance.scale.truncate();

                    let transform = transform
                        * Transform {
                            translation,
                            rotation,
                            scale,
                        };

                    queue.push_back((shape, transform));
                }
            }
            _ => {
                tracing::error!("unknown mesh shape");
                return None;
            }
        }
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (mesh, transform) in meshes {
        let base_index = vertices.len() as u32;

        for vertex in mesh.vertices {
            let vertex = transform.transform_point(vertex);
            vertices.push(vertex);
        }

        for index in mesh.indices {
            let index = base_index + index;
            debug_assert!(index < vertices.len() as u32);
            indices.push(index);
        }
    }

    if indices.is_empty() {
        return None;
    }

    let mesh = if all_meshes_are_convex {
        TriMesh::new_convex(vertices, indices)
    } else {
        TriMesh::new(vertices, indices)
    };

    Some(ColliderNode {
        transform,
        collider: Collider {
            // TODO: Load values from material property.
            friction: 0.5,
            restitution: 0.5,
            shape: ColliderShape::TriMesh(mesh),
        },
    })
}
