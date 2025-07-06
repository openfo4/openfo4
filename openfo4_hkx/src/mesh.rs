use glam::{Mat4, Vec3, Vec4};
use openfo4_hkx_derive::{HkStruct, signature};

use crate::behavior::QsTransform;
use crate::physics::Shape;
use crate::{Array, OpaquePtr, Ptr, ReferencedObject, RelArray, Transform};

#[derive(Clone, Debug, HkStruct)]
pub struct CompressedMeshShape {
    _pad0: u64,
    _pad1: u64,
    _pad2: u64,
    _pad3: u64,
    _pad4: u64,
    _pad5: u64,
    _pad6: u64,
    _pad7: u64,
    _pad8: u64,
    _pad9: u64,
    _pad10: u64,
    _pad11: u64,
    // Offset: 96
    pub data: Ptr<CompressedMeshShapeData>,
}

#[derive(Clone, Debug, Default)]
pub struct TriMesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct CompressedMeshShapeData {
    pub object: ReferencedObject,
    // Offset 16
    pub mesh_tree: CompressedMeshShapeTree,
}

impl CompressedMeshShapeData {
    pub fn mesh(&self) -> TriMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let mut primitive_offset = 0;
        let mut shared_vertices_offset: u32 = 0;

        for section in &*self.mesh_tree.sections {
            // Seems like only the 7 LSB are used. The rest is seemingly
            // random.
            let num_primitives = section.primitives & ((1 << 7) - 1);

            for primitive in &self.mesh_tree.primitives
                [primitive_offset as usize..primitive_offset as usize + num_primitives as usize]
            {
                // Sometimes there are primitives that are degenerate (duplicte indices)
                // and point outside of the valid buffer.
                // Filtering them out seems to fix this?
                if primitive[0] == primitive[1]
                    || primitive[0] == primitive[2]
                    || primitive[0] == primitive[3]
                    || primitive[1] == primitive[2]
                    || primitive[1] == primitive[3]
                    || primitive[2] == primitive[3]
                {
                    continue;
                }

                let scale = Vec3::new(
                    section.codec_params4,
                    section.codec_params5,
                    section.codec_params6,
                );
                let offset = Vec3::new(
                    section.codec_params1,
                    section.codec_params2,
                    section.codec_params3,
                );

                let vs = primitive.map(|index| {
                    debug_assert!(
                        index <= section.num_packed_vertices + section.num_shared_indices
                    );

                    if index >= section.num_packed_vertices {
                        let offset = shared_vertices_offset + u32::from(index)
                            - u32::from(section.num_packed_vertices);
                        let vertex_index = self.mesh_tree.shared_vertices_index[offset as usize];
                        let vertex = self.mesh_tree.shared_vertices[usize::from(vertex_index)];
                        decompress_shared_vertex(
                            vertex,
                            self.mesh_tree.aabb.min,
                            self.mesh_tree.aabb.max,
                        )
                    } else {
                        let vertex = self.mesh_tree.packed_vertices
                            [usize::from(index) + section.first_packed_vertex as usize];
                        decompress_packed_vertex(vertex, scale, offset)
                    }
                });

                // A->B->C C->D->A
                indices.extend([
                    vertices.len() as u32 + 0,
                    vertices.len() as u32 + 1,
                    vertices.len() as u32 + 2,
                    vertices.len() as u32 + 2,
                    vertices.len() as u32 + 3,
                    vertices.len() as u32 + 0,
                ]);
                vertices.extend(vs);
            }

            primitive_offset += num_primitives;
            shared_vertices_offset += section.num_shared_indices as u32;
        }

        debug_assert_eq!(indices.len() % 3, 0);

        TriMesh { vertices, indices }
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct CompressedMeshShapeTree {
    // Offset: 0
    _pad0: u64,
    // Offset: 8
    _pad1: u64,
    // Offset: 16
    pub aabb: HkAabb,
    // Offset: 48
    pub num_primitive_keys: i32,
    // Offset: 52
    pub bits_per_key: i32,
    // Offset: 56
    pub max_key_value: u32,
    // Offset: 60
    _pad2: u32,
    // Offset: 64
    pub sections: Array<CompressedMeshShapeTreeSection>,
    // Offset: 80
    pub primitives: Array<[u8; 4]>,
    // Offset: 96
    pub shared_vertices_index: Array<u16>,
    // Offset: 112
    pub packed_vertices: Array<u32>,
    // Offset: 128
    pub shared_vertices: Array<u64>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct CompressedMeshShapeTreeSection {
    _pad0: u64,
    _pad1: u64,
    // Offset: 16
    pub aabb: HkAabb,
    pub codec_params1: f32,
    pub codec_params2: f32,
    pub codec_params3: f32,
    pub codec_params4: f32,
    pub codec_params5: f32,
    pub codec_params6: f32,
    // Offset: 72
    pub first_packed_vertex: u32,
    // Offset: 76
    pub shared_vertices: u32,
    // Offset: 80
    pub primitives: u32,
    // Offset: 84
    pub data_runs: u32,
    // Offset: 88
    pub num_packed_vertices: u8,
    // Offset: 89
    pub num_shared_indices: u8,
    // Offset: 90
    pub leaf_index: u16,
    // Offset: 92
    pub page: u8,
    // Offset: 93
    pub flags: u8,
    // Offset: 94
    pub layer_data: u8,
    // Offset: 95
    _unused_data: u8,
}

pub struct SimdTree {}

pub fn decompress_packed_vertex(vertex: u32, scale: Vec3, offset: Vec3) -> Vec3 {
    let x = ((vertex & 0x7FF) as f32) * scale.x + offset.x;
    let y = (((vertex >> 11) & 0x7FF) as f32) * scale.y + offset.y;
    let z = (((vertex >> 22) & 0x3FF) as f32) * scale.z + offset.z;
    Vec3 { x, y, z }
}

pub fn decompress_shared_vertex(vertex: u64, min: Vec4, max: Vec4) -> Vec3 {
    let scale_x = (max.x - min.x) / ((1 << 21) - 1) as f32;
    let scale_y = (max.y - min.y) / ((1 << 21) - 1) as f32;
    let scale_z = (max.z - min.z) / ((1 << 22) - 1) as f32;
    let x = ((vertex & 0x1FFFFF) as f32) * scale_x + min.x;
    let y = (((vertex >> 21) & 0x1FFFFF) as f32) * scale_y + min.y;
    let z = (((vertex >> 42) & 0x3FFFFF) as f32) * scale_z + min.z;
    Vec3 { x, y, z }
}

#[derive(Copy, Clone, Debug, HkStruct)]
pub struct HkAabb {
    pub min: Vec4,
    pub max: Vec4,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConvexPolytopeShape {
    // Offset: 0
    pub shape: Shape,
    // Offset: 40
    _pad0: u64,
    // Offset: 48
    pub vertices: RelArray<Vec4>,
    // Offset: 52
    _pad1: u32,
    // Offset: 56
    _pad2: u32,
    // Offset: 60
    _pad3: u32,
    // Offset: 64
    pub planes: RelArray<Vec4>,
    // Offset: 68
    pub faces: RelArray<ConvexPolytopeShapeFace>,
    // Offset: 72
    pub indices: RelArray<u8>,
}

impl ConvexPolytopeShape {
    pub fn to_tri_mesh(&self) -> TriMesh {
        let vertices = self
            .vertices
            .iter()
            .map(|vertex| vertex.truncate())
            .collect();

        let mut indices = Vec::new();

        for face in &*self.faces {
            // Faces with less than 3 vertices are always degenerate.
            if face.num_indices < 3 {
                continue;
            }

            let slice = &self.indices
                [face.first_index as usize..face.first_index as usize + face.num_indices as usize];

            // The polygon is convex so we can do simple fan triangulation.
            let a = slice[0];
            for s in slice.windows(2) {
                let b = s[0];
                let c = s[1];
                indices.extend([a as u32, b as u32, c as u32]);
            }
        }

        TriMesh { vertices, indices }
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConvexPolytopeShapeFace {
    // Offset: 0
    pub first_index: u16,
    // Offset: 2
    pub num_indices: u8,
    // Offset: 3
    pub min_half_anngle: u8,
}

#[derive(Clone, Debug, HkStruct)]
pub struct CapsuleShape {
    // Offset: 0
    pub shape: ConvexPolytopeShape,
    // Offset: 80
    pub a: Vec4,
    // Offset: 96
    pub b: Vec4,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ScaledConvexShape {
    // Offset: 0
    pub shape: Shape,
    // Offset: 40
    _pad0: u64,
    // Offset: 48
    pub core_shape: OpaquePtr,
    // Offset: 56
    pub core_shape_size: i32,
    // Offset: 60
    _pad1: u32,
    // Offset: 64
    pub scale: Vec4,
    // Offset: 80
    pub translation: Vec4,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x4620d11c)]
pub struct DynamicCompoundShape {
    // Offset: 0
    pub shape: CompositeShape,
    // Offset: 96
    pub instances: Array<ShapeInstance>,
    // Offset: 112
    _pad0: u64,
    _pad1: u64,
    // Offset: 128
    pub aabb: HkAabb,
    // Offset: 160
    pub is_mutable: bool,
    // pub mutation_signals:
    // ...
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x12bb3bef)]
pub struct CompositeShape {
    // Offset: 0
    pub shape: Shape,
    // Offset: 40
    _pad0: u64,
    // Offset: 48
    pub edge_welding_map: SparseCompactMap,
    // Offset: 88
    pub shape_tag_codec_info: u32,
    // Offset: 92
    _pad1: u32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x4558127c)]
pub struct SparseCompactMap {
    // Offset: 0
    pub secondary_key_mask: u32,
    // Offset: 4
    pub secondary_key_bits: u32,
    // Offset: 8
    pub primary_key_to_index: Array<u16>,
    // Offset: 24
    pub value_and_secondary_keys: Array<u16>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x5e3dae05)]
pub struct ShapeInstance {
    // Offset: 0
    pub transform: Mat4,
    // Offset: 64
    pub scale: Vec4,
    // Offset: 80
    pub shape: Option<OpaquePtr>,
    // Offset: 88
    pub shape_tag: u16,
    // Offset: 90
    pub destruction_tag: u16,
    // Offset: 92
    _pad0: [u8; 30],
}
