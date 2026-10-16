use bevy_mikktspace::{Geometry, TangentSpace};
use game_common::collections::hierarchy::Hierarchy;
use game_common::components::{Color, Transform};
use glam::{Vec2, Vec3, Vec4};
use gltf::material::AlphaMode;

#[derive(Clone, Debug)]
pub struct GltfScene {
    pub nodes: Hierarchy<GltfNode>,
}

#[derive(Clone, Debug)]
pub struct GltfNode {
    pub transform: Transform,
    pub mesh: Option<MeshIndex>,
    pub material: Option<MaterialIndex>,
    pub name: Option<String>,
}

#[derive(Copy, Clone, Debug)]
pub struct GltfMeshMaterial {
    pub mesh: MeshIndex,
    pub material: MaterialIndex,
}

#[derive(Clone, Debug, Default)]
pub struct GltfMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub tangents: Vec<Vec4>,
    pub indices: Vec<u32>,
    pub colors: Vec<Vec4>,
}

impl GltfMesh {
    pub fn compute_tangents(&mut self) {
        // TODO: Checks and precomputes should move to gltf crate.
        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.uvs.len());

        let len = self.positions.len();

        struct Mikktpsace<'a> {
            indices: &'a [u32],
            positions: &'a [Vec3],
            normals: &'a [Vec3],
            uvs: &'a [Vec2],
            tangents: Vec<Vec4>,
        }

        impl<'a> Mikktpsace<'a> {
            fn index(&self, face: usize, vert: usize) -> usize {
                self.indices[face * 3 + vert] as usize
            }
        }

        impl<'a> Geometry for Mikktpsace<'a> {
            fn num_faces(&self) -> usize {
                self.indices.len() / 3
            }

            fn num_vertices_of_face(&self, _face: usize) -> usize {
                3
            }

            fn position(&self, face: usize, vert: usize) -> [f32; 3] {
                self.positions[self.index(face, vert)].to_array()
            }

            fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
                self.normals[self.index(face, vert)].to_array()
            }

            fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
                self.uvs[self.index(face, vert)].to_array()
            }

            fn set_tangent(
                &mut self,
                tangent_space: Option<TangentSpace>,
                face: usize,
                vert: usize,
            ) {
                let tangent = tangent_space.unwrap_or_default().tangent_encoded();
                let tangent = Vec4::from_array(tangent);
                let index = self.index(face, vert);
                self.tangents[index] = tangent;
            }
        }

        let mut mesh = Mikktpsace {
            indices: &self.indices,
            positions: &self.positions,
            normals: &self.normals,
            uvs: &self.uvs,
            tangents: vec![Vec4::ZERO; len],
        };
        bevy_mikktspace::generate_tangents(&mut mesh).unwrap();

        self.tangents = mesh.tangents;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GltfMaterial {
    pub alpha_mode: AlphaMode,
    pub base_color: Color,
    pub base_color_texture: Option<TextureIndex>,
    pub normal_texture: Option<TextureIndex>,
    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: Option<TextureIndex>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshIndex {
    // A `MeshIndex` is composed of the gltf mesh and primitive index.
    // The primitive is what our scene "mesh" actually is.
    // Neither index is enough to uniquely identify the mesh.
    pub(crate) mesh: usize,
    pub(crate) primitive: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialIndex(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureIndex(pub(crate) usize);
