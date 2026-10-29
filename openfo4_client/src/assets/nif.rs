pub mod animation;
pub mod animation_controller;

mod mesh;
mod node;
mod physics;

use bitflags::bitflags;
use hashbrown::HashMap;
use openfo4_animation::skeleton::{Bone, Skeleton};
use openfo4_asset::{Asset, AssetLoader, Handle, HandleOrAsset};
use openfo4_common::collections::hierarchy::{Hierarchy, Key};
use openfo4_common::components::{Color, Transform};
use openfo4_core::ecs::{Entity, EntityBuilder, World};
use openfo4_core::hierarchy::Children;
use openfo4_core::path::{DataPath, DataPathBuf};
use openfo4_core_pipeline::entities::{Entities, MaterialHandle};
use openfo4_core_pipeline::material::{AlphaMode, StandardMaterial};
use openfo4_hkx::mesh::ExtractCompressedMeshShapeError;
use openfo4_nif::connect_points::BSConnectPointParents;
use openfo4_nif::material::{AlphaTestFunction, NiAlphaProperty, ShaderFlags1, ShaderFlags2};
use openfo4_nif::{BSLeafAnimNode, Block, BlockRef, BlockType, Nif};
use openfo4_physics::components::Collider;
use thiserror::Error;

use crate::assets::Error;
use crate::assets::data_file::DataFile;
use crate::assets::material::{BgsmMaterial, BgsmStandardMaterial, MaterialKey};
use crate::assets::nif::animation_controller::{
    AnimationController, AnimationControllerInstance, MaterialController,
};
use crate::assets::texture::{FileImageHandle, FileImageKey};
use crate::render::RenderObject;

pub const NIF_TO_METERS: f32 = 1.0 / 69.99125;

/// Z Offset applied to decals are MVP transformation to prevent Z fighting.
pub const DECAL_DEPTH_BIAS: f32 = 0.0001;

bitflags! {
    /// Defines which components of a nif file should be loaded.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct LoadNifFlags: u32 {
        /// [`RenderObject`]s.
        ///
        /// This includes loading of meshes, materials and textures.
        const RENDER_OBJECTS = 1 << 0;
        /// [`Collider`]s.
        const COLLIDERS = 1 << 1;
        /// [`AnimationController`]s.
        const ANIMATIONS = 1 << 2;

        /// All graphical flags.
        const GRAPHICS = Self::RENDER_OBJECTS.bits() | Self::ANIMATIONS.bits();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NifKey {
    pub path: DataPathBuf,
    pub flags: LoadNifFlags,
}

#[derive(Clone, Debug)]
pub struct NifScene {
    pub nodes: Hierarchy<Node>,
    /// The name of the parent [`AttachmentPoint`] where this scene attaches.
    pub attaches: Option<String>,
    /// The skeleton if the NIF is skinned.
    ///
    /// The bone order is arbitrary and the bones have the same name as the corresponding nodes in
    /// the nif file.
    pub skeleton: Option<Skeleton>,
    controller: AnimationController,
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
    type Node;

    fn instantiate_with_feedback<'a, F>(
        &'a self,
        callback: F,
        transform: Transform,
        world: &mut World,
        root: Entity,
        flags: LoadNifFlags,
    ) where
        F: FnMut(&'a Self::Node, Entity);
}

impl Instantiate for Handle<NifScene> {
    type Node = Node;

    fn instantiate_with_feedback<'a, F>(
        &'a self,
        mut callback: F,
        transform: Transform,
        world: &mut World,
        root: Entity,
        flags: LoadNifFlags,
    ) where
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

            if let Some(renderable) = &node.renderable
                && flags.contains(LoadNifFlags::RENDER_OBJECTS)
            {
                builder.add(renderable.clone());
            }

            if let Some(collider) = &node.collider
                && flags.contains(LoadNifFlags::COLLIDERS)
            {
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

        let root_children = self
            .nodes
            .root_keys()
            .map(|key| *entities.get(&key).unwrap())
            .collect();

        world.insert_one(root, Children(root_children)).unwrap();
        world.insert_one(root, transform).unwrap();

        // This will keep the scene cached until the root entity
        // is destroyed.
        let handle: Handle<NifScene> = self.clone();
        world.insert_one(root, handle.clone()).unwrap();

        if self.controller.is_visible() && flags.contains(LoadNifFlags::ANIMATIONS) {
            let controller = AnimationControllerInstance {
                scene: self.clone(),
                nodes: entities,
                sequences: HashMap::new(),
            };

            world.insert_one(root, controller).unwrap();
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub renderable: Option<RenderObject>,
    pub collider: Option<Collider>,
    pub skin: Option<Skin>,
    pub attachments: Vec<AttachmentPoint>,
    /// The name of the parent where this node attaches.
    pub attaches: String,
    /// The ID of the bone if this NIF contains a [`Skeleton`].
    pub bone: Option<usize>,
}

impl Node {
    pub fn build(&self, flags: LoadNifFlags) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        builder.add(self.transform);

        if let Some(renderable) = &self.renderable
            && flags.contains(LoadNifFlags::RENDER_OBJECTS)
        {
            builder.add(renderable.clone());
        }

        if let Some(collider) = &self.collider
            && flags.contains(LoadNifFlags::COLLIDERS)
        {
            builder.add(collider.clone());
        }

        builder
    }
}

#[derive(Clone, Debug)]
pub struct Skin {
    pub bones: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AttachmentPoint {
    pub name: String,
    pub offset: Transform,
}

#[derive(Debug)]
pub struct NifLoaderContext {
    pub entities: Entities,
    /// The material used if no other material is specified.
    pub default_material: MaterialHandle,
}

impl NifLoaderContext {
    pub fn new(entities: Entities) -> Self {
        let default_material = entities.create_material(StandardMaterial::default());

        Self {
            entities,
            default_material,
        }
    }
}

impl Asset for NifScene {
    type Key = NifKey;
    type Error = Error;
    type Context = NifLoaderContext;

    #[tracing::instrument(name = "NifScene::load", skip_all)]
    async fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let mut key = key.clone();

        // All nif files are located in the meshes directory.
        if !key.path.starts_with(DataPath::from_str("meshes")) {
            key.path.push_front("meshes");
        }

        // `meshes/markers` contains editor markers.
        // There does not seem any other indication that a
        // file/object is a marker.
        // Some markers are not inside this directory.
        if key.path.starts_with(DataPath::from_str("meshes/markers/")) {
            tracing::info!("skip {}", key.path);

            return Ok(Self {
                nodes: Hierarchy::new(),
                attaches: None,
                controller: AnimationController::default(),
                _materials: Vec::new(),
                _textures: Vec::new(),
                skeleton: None,
            });
        }

        let nif = loader.load::<DataFile<Nif>, _>(&key.path).await?;

        let Some(root) = nif.root() else {
            // Nif file is empty, return an empty scene.
            return Ok(Self {
                nodes: Hierarchy::new(),
                attaches: None,
                controller: AnimationController::default(),
                _materials: Vec::new(),
                _textures: Vec::new(),
                skeleton: None,
            });
        };

        let mut connect_points_children = None;
        let mut connect_points_parents = None;

        match nif.get(root) {
            Some(Block::NiNode(node) | Block::BSLeafAnimNode(BSLeafAnimNode { node })) => {
                for block in &node.extra_data {
                    match nif.get(*block) {
                        Some(Block::BSConnectPointChildren(node)) => {
                            connect_points_children = Some(node);
                        }
                        Some(Block::BSConnectPointParents(node)) => {
                            connect_points_parents = Some(node);
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }

        // If any node is skinned we need to produce a skeleton to bind the skin to.
        let is_skinned = nif.iter().any(|block| match block {
            Block::BSTriShape(node) => node.skin.is_some(),
            Block::BSSubIndexTriShape(node) => node.shape.skin.is_some(),
            Block::BSMeshLODTriShape(node) => node.shape.skin.is_some(),
            _ => false,
        });

        let mut nodes = Hierarchy::new();
        let mut block_to_node = HashMap::new();

        let mut visitor = Visitor::new(key, &nif, loader, ctx, LoadState::default(), root);
        visitor.connect_points_parents = connect_points_parents;
        visitor.queue.push((None, root));

        while let Some((parent, id)) = visitor.queue.pop() {
            if let Some(parent) = parent {
                visitor.parents.insert(id, parent);
            }

            let Some(block) = nif.get(id) else {
                continue;
            };

            visitor.id = id;
            if let Some(node) = block.visit(&mut visitor, id).await {
                let parent = parent.map(|parent| *block_to_node.get(&parent).unwrap());
                let node = nodes.append(parent, node);
                block_to_node.insert(id, node);
            }
        }

        let mut attaches = None;
        if let Some(children) = connect_points_children {
            if let Some(point) = children.points.first() {
                let name = extract_attachment_name(point);
                attaches = Some(name.to_owned());
            }
        }

        let mut controller = AnimationController::default();
        if visitor.key.flags.contains(LoadNifFlags::ANIMATIONS) {
            controller = build_animation_controller(
                &nif,
                loader,
                visitor.controllers_to_load,
                &visitor.state,
                &block_to_node,
                &visitor.key.path,
            )
            .await;
        };

        let skeleton = is_skinned.then(|| create_skeleton(&mut nodes));

        Ok(Self {
            nodes,
            attaches,
            controller,
            _materials: visitor.material_handles,
            _textures: visitor.texture_handles,
            skeleton,
        })
    }
}

#[derive(Clone, Debug, Default)]
struct LoadState<'a> {
    /// Maps the [`BlockRef`] of a material to the [`Node`] where the material is attached.
    material_to_node: HashMap<BlockRef, Key>,
    /// Maps the [`BlockRef`] of a material to the [`NiAlphaProperty`] used for the material.
    material_alpha_properties: HashMap<BlockRef, &'a NiAlphaProperty>,
}

async fn build_animation_controller(
    nif: &Nif,
    loader: &AssetLoader,
    controllers_to_load: Vec<BlockRef>,
    state: &LoadState<'_>,
    block_to_node: &HashMap<BlockRef, Key>,
    file_path: &DataPath,
) -> AnimationController {
    let mut material_controllers = HashMap::new();

    for block in controllers_to_load {
        let Ok(controller) = animation::load_controller(&*nif, block).inspect_err(|err| {
            tracing::error!(
                "failed to load animation controller from file {:?}: {}",
                file_path,
                err
            );
        }) else {
            continue;
        };

        for (target, values) in controller.material {
            match nif.get(target) {
                // Material
                Some(Block::BSLightingShaderProperty(_) | Block::BSEffectShaderProperty(_)) => {
                    let Some(node) = state.material_to_node.get(&target).copied() else {
                        continue;
                    };

                    let alpha_property = state.material_alpha_properties.get(&target).copied();
                    let material = load_material_plain(loader, nif, target, alpha_property)
                        .await
                        .unwrap();

                    material_controllers.insert(
                        node,
                        MaterialController {
                            material: material,
                            values,
                        },
                    );
                }
                _ => todo!(),
            }
        }
    }

    let sequences = animation::loader_controller_sequences(nif, block_to_node)
        .inspect_err(|err| {
            tracing::error!(
                "failed to load controller sequences from file {:?}: {}",
                file_path,
                err
            );
        })
        .unwrap_or_default();

    AnimationController {
        material: material_controllers,
        sequences,
    }
}

async fn load_material(
    loader: &AssetLoader,
    entities: &Entities,
    nif: &Nif,
    block: BlockRef,
    alpha_property: Option<&NiAlphaProperty>,
    controllers_to_load: &mut Vec<BlockRef>,
) -> Result<HandleOrAsset<BgsmMaterial>, Error> {
    let mut alpha_mode = None;
    if let Some(alpha_property) = alpha_property {
        if let Some(alpha_test) = alpha_property.alpha_test {
            let threshold = (alpha_test.threshold as f32) / 255.0;

            match alpha_test.function {
                AlphaTestFunction::Always => {
                    alpha_mode = Some(AlphaMode::Opaque);
                }
                AlphaTestFunction::Greater => {
                    alpha_mode = Some(AlphaMode::Mask(threshold));
                }
                AlphaTestFunction::GreaterEqual => {
                    alpha_mode = Some(AlphaMode::Mask(threshold));
                }
                _ => todo!(),
            }
        }

        if let Some(blend) = alpha_property.alpha_blend {
            alpha_mode = Some(AlphaMode::Blend);
        }
    }

    match nif.get(block) {
        Some(Block::BSLightingShaderProperty(shader_property)) => {
            if let Some(controller) = shader_property.controller {
                controllers_to_load.push(controller);
            }

            // If the name field is not empty we need to load from BGSM file.
            if !shader_property.name.is_empty() {
                let material = loader
                    .load::<BgsmMaterial, _>(&MaterialKey {
                        alpha_mode,
                        path: DataPathBuf::new(&*shader_property.name),
                    })
                    .await?;

                return Ok(HandleOrAsset::Handle(material));
            }

            let tree = shader_property
                .shader_flags2
                .contains(ShaderFlags2::TREE_ANIM);

            let mut material = StandardMaterial {
                alpha_mode: alpha_mode.unwrap_or(AlphaMode::Opaque),
                base_color: Color::from_rgba([1.0, 1.0, 1.0, shader_property.alpha]),
                flip_normal_y: true,
                two_component_normal_encoding: true,
                roughness: 1.0 - shader_property.smoothness,
                specular_color: Color::from_rgb(shader_property.specular_color.to_array()),
                specular_strength: shader_property.specular_strength,
                depth_bias: if shader_property.shader_flags1.contains(ShaderFlags1::DECAL) {
                    DECAL_DEPTH_BIAS
                } else {
                    0.0
                },
                grayscale_palette_scale: shader_property.grayscale_to_palette_scale,
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
                // 3. Grayscale Palette LUT
                // 4. Environment
                // 5. ?
                // 6. ?
                // 7. Specular Glossiness
                // 8. ?
                // 9. ?

                let has_grayscale_palette = texture_set
                    .textures
                    .get(3)
                    .is_some_and(|path| !path.is_empty());

                if let Some(path) = texture_set.textures.get(0) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let key = FileImageKey::new(path);
                        // The base color texture is SRGB by default, however if we
                        // have a grayscale palette texture the base color texture
                        // is no longer really color, but just a lookup table and needs
                        // to be UNORM.
                        let key = key.with_force_srgb(!has_grayscale_palette);

                        let handle = loader.load::<FileImageHandle, _>(&key).await?;

                        material.base_color_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(1) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let handle = loader
                            .load::<FileImageHandle, _>(&FileImageKey::new(path))
                            .await?;

                        material.normal_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(3) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let handle = loader
                            .load::<FileImageHandle, _>(
                                &FileImageKey::new(path).with_force_srgb(true),
                            )
                            .await?;

                        material.grayscale_palette = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(7) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let handle = loader
                            .load::<FileImageHandle, _>(&FileImageKey::new(path))
                            .await?;

                        material.specular_glossiness_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }
            }

            let material = entities.create_material(material);
            Ok(HandleOrAsset::Asset(BgsmMaterial {
                material,
                textures,
                tree,
            }))
        }
        Some(Block::BSEffectShaderProperty(property)) => {
            if let Some(controller) = property.controller {
                controllers_to_load.push(controller);
            }

            let has_grayscale_palette = !property.grayscale_texture.is_empty();

            let mut textures = Vec::new();
            let mut material = StandardMaterial {
                alpha_mode: alpha_mode.unwrap_or(AlphaMode::Opaque),
                base_color: Color::from_rgba([
                    property.base_color.r * property.base_color_scale,
                    property.base_color.g * property.base_color_scale,
                    property.base_color.b * property.base_color_scale,
                    // Alpha is unaffected by base color scale.
                    property.base_color.a,
                ]),
                ..Default::default()
            };

            if !property.source_texture.is_empty() {
                let path = DataPathBuf::new(&*property.source_texture);
                let key = FileImageKey::new(path);
                // The base color texture is SRGB by default, however if we
                // have a grayscale palette texture the base color texture
                // is no longer really color, but just a lookup table and needs
                // to be UNORM.
                let key = key.with_force_srgb(!has_grayscale_palette);

                let handle = loader.load::<FileImageHandle, _>(&key).await?;

                material.base_color_texture = Some(handle.0.clone());
                textures.push(handle);
            }

            if !property.grayscale_texture.is_empty() {
                let path = DataPathBuf::new(&*property.grayscale_texture);

                let handle = loader
                    .load::<FileImageHandle, _>(&FileImageKey::new(path).with_force_srgb(true))
                    .await?;

                material.grayscale_palette = Some(handle.0.clone());
                textures.push(handle);
            }

            let material = entities.create_material(material);
            Ok(HandleOrAsset::Asset(BgsmMaterial {
                material,
                textures,
                tree: false,
            }))
        }
        _ => Ok(HandleOrAsset::Asset(BgsmMaterial {
            material: entities.create_material(StandardMaterial::default()),
            textures: Vec::new(),
            tree: false,
        })),
    }
}

/// Loads the [`StandardMaterial`] for the given [`BlockRef`].
///
/// Unlike [`load_material`], this function does not upload the material to GPU memory and does not
/// deduplicate the same material across different nif scene instances.
// FIXME: Contains a lot of duplication with load_material, can we not do this?
async fn load_material_plain(
    loader: &AssetLoader,
    nif: &Nif,
    block: BlockRef,
    alpha_property: Option<&NiAlphaProperty>,
) -> Result<StandardMaterial, Error> {
    let mut alpha_mode = None;
    if let Some(alpha_property) = alpha_property {
        if let Some(alpha_test) = alpha_property.alpha_test {
            let threshold = (alpha_test.threshold as f32) / 255.0;

            match alpha_test.function {
                AlphaTestFunction::Always => {
                    alpha_mode = Some(AlphaMode::Opaque);
                }
                AlphaTestFunction::Greater => {
                    alpha_mode = Some(AlphaMode::Mask(threshold));
                }
                AlphaTestFunction::GreaterEqual => {
                    alpha_mode = Some(AlphaMode::Mask(threshold));
                }
                _ => todo!(),
            }
        }

        if let Some(blend) = alpha_property.alpha_blend {
            alpha_mode = Some(AlphaMode::Blend);
        }
    }

    match nif.get(block) {
        Some(Block::BSLightingShaderProperty(shader_property)) => {
            // If the name field is not empty we need to load from BGSM file.
            if !shader_property.name.is_empty() {
                let material = loader
                    .load::<BgsmStandardMaterial, _>(&MaterialKey {
                        alpha_mode,
                        path: DataPathBuf::new(&*shader_property.name),
                    })
                    .await?;

                return Ok(material.material.clone());
            }

            let mut material = StandardMaterial {
                alpha_mode: alpha_mode.unwrap_or(AlphaMode::Opaque),
                base_color: Color::from_rgba([1.0, 1.0, 1.0, shader_property.alpha]),
                flip_normal_y: true,
                two_component_normal_encoding: true,
                roughness: 1.0 - shader_property.smoothness,
                specular_color: Color::from_rgb(shader_property.specular_color.to_array()),
                specular_strength: shader_property.specular_strength,
                depth_bias: if shader_property.shader_flags1.contains(ShaderFlags1::DECAL) {
                    DECAL_DEPTH_BIAS
                } else {
                    0.0
                },
                grayscale_palette_scale: shader_property.grayscale_to_palette_scale,
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
                // 3. Grayscale Palette LUT
                // 4. Environment
                // 5. ?
                // 6. ?
                // 7. Specular Glossiness
                // 8. ?
                // 9. ?

                let has_grayscale_palette = texture_set
                    .textures
                    .get(3)
                    .is_some_and(|path| !path.is_empty());

                if let Some(path) = texture_set.textures.get(0) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let key = FileImageKey::new(path);
                        // The base color texture is SRGB by default, however if we
                        // have a grayscale palette texture the base color texture
                        // is no longer really color, but just a lookup table and needs
                        // to be UNORM.
                        let key = key.with_force_srgb(!has_grayscale_palette);

                        let handle = loader.load::<FileImageHandle, _>(&key).await?;

                        material.base_color_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(1) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let handle = loader
                            .load::<FileImageHandle, _>(&FileImageKey::new(path))
                            .await?;

                        material.normal_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(3) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let handle = loader
                            .load::<FileImageHandle, _>(
                                &FileImageKey::new(path).with_force_srgb(true),
                            )
                            .await?;

                        material.grayscale_palette = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }

                if let Some(path) = texture_set.textures.get(7) {
                    if !path.is_empty() {
                        let path = DataPathBuf::new(&**path);
                        let handle = loader
                            .load::<FileImageHandle, _>(&FileImageKey::new(path))
                            .await?;

                        material.specular_glossiness_texture = Some(handle.0.clone());
                        textures.push(handle);
                    }
                }
            }

            Ok(material)
        }
        Some(Block::BSEffectShaderProperty(property)) => {
            let mut textures = Vec::new();
            let mut material = StandardMaterial {
                alpha_mode: alpha_mode.unwrap_or(AlphaMode::Opaque),
                base_color: Color::from_rgba([
                    property.base_color.r * property.base_color_scale,
                    property.base_color.g * property.base_color_scale,
                    property.base_color.b * property.base_color_scale,
                    property.base_color.a * property.base_color_scale,
                ]),
                ..Default::default()
            };

            if !property.source_texture.is_empty() {
                let path = DataPathBuf::new(&*property.source_texture);

                let handle = loader
                    .load::<FileImageHandle, _>(&FileImageKey::new(path))
                    .await?;

                material.base_color_texture = Some(handle.0.clone());
                textures.push(handle);
            }

            Ok(material)
        }
        _ => Ok(StandardMaterial::default()),
    }
}

#[derive(Clone, Debug)]
struct ColliderNode {
    transform: Transform,
    collider: Collider,
}

#[derive(Clone, Debug, Error)]
enum LoadColliderError {
    #[error(transparent)]
    Hkx(openfo4_hkx::Error),
    #[error("invalid material id: {0}")]
    InvalidMaterialId(u16),
    #[error("cycle in collider pointers")]
    PointerCylce,
    #[error("unknown mesh shape")]
    UnknownMeshShape,
    #[error("invalid compressed mesh shape: {0}")]
    InvalidCompressedMeshShape(ExtractCompressedMeshShapeError),
}

fn extract_attachment_name(mut name: &str) -> &str {
    if name.starts_with("P-") {
        name = name.strip_prefix("P-").unwrap_or(name);
        name.strip_prefix("C-").unwrap_or(name)
    } else {
        name = name.strip_prefix("C-").unwrap_or(name);
        name.strip_prefix("P-").unwrap_or(name)
    }
}

#[derive(Clone, Debug, Error)]
pub enum NifError {
    #[error("unexpected block type {ty:?} for value {value} of {parent:?}")]
    UnexpectedBlockType {
        ty: BlockType,
        value: &'static str,
        parent: BlockType,
    },
}

struct Visitor<'a> {
    key: NifKey,
    nif: &'a Nif,
    parents: HashMap<BlockRef, BlockRef>,
    materials: HashMap<BlockRef, MaterialHandle>,
    loader: &'a AssetLoader,
    ctx: &'a NifLoaderContext,
    state: LoadState<'a>,
    texture_handles: Vec<Handle<FileImageHandle>>,
    material_handles: Vec<Handle<BgsmMaterial>>,
    controllers_to_load: Vec<BlockRef>,
    connect_points_parents: Option<&'a BSConnectPointParents>,
    queue: Vec<(Option<BlockRef>, BlockRef)>,
    id: BlockRef,
}

impl<'a> Visitor<'a> {
    fn new(
        key: NifKey,
        nif: &'a Nif,
        loader: &'a AssetLoader,
        ctx: &'a NifLoaderContext,
        state: LoadState<'a>,
        root: BlockRef,
    ) -> Self {
        let mut parents = HashMap::new();
        let mut queue = vec![(None, root)];

        while let Some((parent, id)) = queue.pop() {
            if let Some(parent) = parent {
                parents.insert(id, parent);
            }

            if let Some(Block::NiNode(node)) = nif.get(id) {
                for child in &node.children {
                    queue.push((Some(id), *child));
                }
            }
        }

        debug_assert!(queue.is_empty());

        Self {
            key,
            nif,
            parents,
            materials: HashMap::new(),
            loader,
            ctx,
            texture_handles: Vec::new(),
            material_handles: Vec::new(),
            controllers_to_load: Vec::new(),
            connect_points_parents: None,
            id: root,
            state,
            queue: Vec::new(),
        }
    }

    fn enqueue(&mut self, block: BlockRef) {
        self.queue.push((Some(self.id), block));
    }

    fn parent(&self, block: BlockRef) -> Option<BlockRef> {
        self.parents.get(&block).copied()
    }

    /// Returns the global transform relative to the root block.
    ///
    /// If any block without a [`Transform`] value is encountered on the path `None` is returned.
    fn global_transform(&self, mut block: BlockRef) -> Option<Transform> {
        let mut transform = self.nif.get(block).and_then(|block| block.transform())?;

        while let Some(parent) = self.parents.get(&block).copied() {
            // It looks like the transform of the root block is not applied.
            if Some(parent) == self.nif.root() {
                break;
            }

            let node = self.nif.get(parent)?;
            let parent_transform = node.transform()?;
            transform = parent_transform * transform;
            block = parent;
        }

        Some(transform)
    }
}

trait Visit {
    async fn visit(&self, visitor: &mut Visitor<'_>, id: BlockRef) -> Option<Node>;

    fn transform(&self) -> Option<Transform> {
        None
    }
}

fn is_marker(name: &str) -> bool {
    name.to_lowercase().contains("marker")
}

impl Visit for Block {
    async fn visit(&self, visitor: &mut Visitor<'_>, id: BlockRef) -> Option<Node> {
        match self {
            Self::NiNode(node) => node.visit(visitor, id).await,
            Self::BSLeafAnimNode(node) => node.node.visit(visitor, id).await,
            Self::BSTriShape(node) => node.visit(visitor, id).await,
            Self::BSSubIndexTriShape(node) => node.shape.visit(visitor, id).await,
            Self::BSMeshLODTriShape(node) => node.shape.visit(visitor, id).await,
            Self::BhkNPCollisionObject(node) => node.visit(visitor, id).await,
            _ => None,
        }
    }

    fn transform(&self) -> Option<Transform> {
        match self {
            Self::NiNode(node) => node.transform(),
            Self::BSLeafAnimNode(node) => node.node.transform(),
            Self::BSTriShape(node) => node.transform(),
            Self::BSSubIndexTriShape(node) => node.shape.transform(),
            Self::BSMeshLODTriShape(node) => node.shape.transform(),
            _ => None,
        }
    }
}

fn create_skeleton(nodes: &mut Hierarchy<Node>) -> Skeleton {
    let mut queue = Vec::<(Key, Option<Key>)>::new();

    for key in nodes.root_keys() {
        queue.push((key, None));
    }

    let mut bones = Vec::new();
    let mut node_to_bone_index = HashMap::new();

    while let Some((key, parent)) = queue.pop() {
        let index = bones.len();

        let node = nodes.get_mut(key).unwrap();
        node.bone = Some(index);

        node_to_bone_index.insert(key, index);
        bones.push(Bone {
            index,
            name: node.name.clone(),
            children: Vec::new(),
            transform: node.transform,
        });

        if let Some(parent) = parent {
            let parent = node_to_bone_index[&parent];
            bones[parent].children.push(index);
        }

        for (child, _) in nodes.children(key).into_iter().flatten() {
            queue.push((child, Some(key)));
        }
    }

    Skeleton { bones }
}
