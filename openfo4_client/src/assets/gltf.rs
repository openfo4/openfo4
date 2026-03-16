use std::path::PathBuf;

use game_common::components::Transform;
use game_core::hierarchy::Hierarchy;
use game_core_pipeline::entities::{ImageHandle, MaterialHandle, MeshHandle};
use game_core_pipeline::material::{AlphaMode, StandardMaterial};
use game_gltf::types::{GltfMaterial, GltfMesh, TextureIndex};
use game_gltf::{GltfData, GltfDecoder};
use game_render::mesh::{Indices, Mesh};
use hashbrown::HashMap;
use hecs::{Entity, EntityBuilder};
use openfo4_asset::{Asset, AssetLoader, Handle};
use openfo4_core::hierarchy::Children;
use tracing::instrument;

use crate::assets::Error;
use crate::assets::nif::{Instantiate, NifLoaderContext};
use crate::render::RenderObject;

#[derive(Clone, Debug)]
pub struct GltfScene {
    nodes: Hierarchy<Node>,
}

impl Instantiate for Handle<GltfScene> {
    type Node = Node;

    fn instantiate_with_feedback<'a, F>(
        &'a self,
        mut callback: F,
        transform: Transform,
        world: &mut hecs::World,
        _flags: super::nif::LoadNifFlags,
    ) -> Entity
    where
        F: FnMut(&'a Node, Entity),
    {
        let mut entities = HashMap::new();

        for (key, _) in self.nodes.iter() {
            let entity = world.reserve_entity();
            entities.insert(key, entity);
        }

        for (key, node) in self.nodes.iter() {
            let entity = *entities.get(&key).unwrap();

            let mut builder = EntityBuilder::new();
            builder.add(node.transform);

            if let Some(object) = &node.render_object {
                builder.add(object.clone());
            }

            if let Some(children) = self.nodes.children(key) {
                let children = children.map(|(k, _)| *entities.get(&k).unwrap()).collect();
                builder.add(Children(children));
            }

            world.insert(entity, builder.build()).unwrap();
            callback(node, entity);
        }

        let root = self
            .nodes
            .root_keys()
            .map(|key| *entities.get(&key).unwrap())
            .collect();

        // The Handle<Self> component will keep the scene cached until
        // the entity is despawned.
        let scene: Handle<GltfScene> = self.clone();
        let root = world.spawn((transform, Children(root), scene));

        root
    }
}

impl Asset for GltfScene {
    type Key = String;
    type Context = NifLoaderContext;
    type Error = Error;

    #[instrument(name = "GltfScene::load", skip_all)]
    async fn load(
        _loader: &AssetLoader,
        path: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let mut dir = PathBuf::from(path);
        dir.pop();

        let data = std::fs::read(&path)?;

        let mut decoder = GltfDecoder::new(&data)?;

        while let Some(path) = decoder.pop_source() {
            let file_path = dir.join(&path);
            let data = std::fs::read(&file_path)?;
            decoder.push_source(path, data);
        }

        let data = decoder.finish().unwrap();

        let scene = data.default_scene().unwrap();

        let mut meshes = HashMap::new();
        let mut materials = HashMap::new();
        let mut images = HashMap::new();

        let mut nodes = Hierarchy::new();
        let mut node_map = HashMap::new();

        for (gltf_key, node) in scene.nodes.iter_topo_sorted() {
            let parent = scene
                .nodes
                .parent_key(gltf_key)
                .map(|v| *node_map.get(&v).unwrap());

            let mesh = node.mesh.map(|index| {
                meshes
                    .entry(index)
                    .or_insert_with(|| {
                        let mesh = data.meshes.get(&index).unwrap();
                        create_mesh(ctx, mesh.clone())
                    })
                    .clone()
            });

            let material = node.material.map(|index| {
                materials
                    .entry(index)
                    .or_insert_with(|| {
                        let material = data.materials.get(&index).unwrap();
                        create_material(ctx, &data, material, &mut images)
                    })
                    .clone()
            });

            let key = nodes.append(
                parent,
                Node {
                    transform: node.transform,
                    render_object: mesh.map(|mesh| RenderObject {
                        mesh,
                        material: material.unwrap_or(ctx.default_material.clone()),
                    }),
                },
            );
            node_map.insert(gltf_key, key);
        }

        Ok(Self { nodes })
    }
}
#[derive(Clone, Debug)]
pub struct Node {
    transform: Transform,
    render_object: Option<RenderObject>,
}

fn create_mesh(ctx: &NifLoaderContext, gltf_mesh: GltfMesh) -> MeshHandle {
    let mut mesh = Mesh::new();
    mesh.set_positions(gltf_mesh.positions);
    mesh.set_normals(gltf_mesh.normals);
    mesh.set_tangents(gltf_mesh.tangents);
    mesh.set_uvs(gltf_mesh.uvs);
    mesh.set_indices(Indices::U32(gltf_mesh.indices));

    ctx.entities.create_mesh(mesh)
}

fn create_material(
    ctx: &NifLoaderContext,
    data: &GltfData,
    material: &GltfMaterial,
    images: &mut HashMap<TextureIndex, ImageHandle>,
) -> MaterialHandle {
    let base_color_texture = material.base_color_texture.map(|index| {
        images
            .entry(index)
            .or_insert_with(|| {
                let image = data.images.get(&index).unwrap();
                ctx.entities.create_image(image.clone())
            })
            .clone()
    });

    let normal_texture = material.normal_texture.map(|index| {
        images
            .entry(index)
            .or_insert_with(|| {
                let image = data.images.get(&index).unwrap();
                ctx.entities.create_image(image.clone())
            })
            .clone()
    });

    let metallic_roughness_texture = material.metallic_roughness_texture.map(|index| {
        images
            .entry(index)
            .or_insert_with(|| {
                let image = data.images.get(&index).unwrap();
                ctx.entities.create_image(image.clone())
            })
            .clone()
    });

    ctx.entities.create_material(StandardMaterial {
        alpha_mode: AlphaMode::Opaque,
        base_color: material.base_color,
        base_color_texture,
        normal_texture,
        roughness: material.roughness,
        metallic: material.metallic,
        metallic_roughness_texture,
        ..Default::default()
    })
}
