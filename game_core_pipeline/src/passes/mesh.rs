use bytemuck::{Pod, Zeroable};
use game_common::collections::slab::Slab;
use game_common::collections::vec_map::Index;
use game_common::components::Transform;
use game_common::define_slab_key;
use game_render::api::CommandQueue;
use game_render::backend::BufferUsage;
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{CompactSlabBuffer, SlabBuffer, SlabIndex};
use game_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use game_render::mesh::Mesh;
use game_tracing::trace_span;
use glam::{Mat3, Mat4, Vec4};
use hashbrown::HashMap;

use crate::material::AlphaMode;
use crate::passes::MaterialState;

/// Storage for meshes and mesh instances.
#[derive(Debug)]
pub struct MeshState {
    meshes: Slab<MeshKey, MeshData>,

    // Properties
    pub positions: SubAllocatedGrowableBuffer<f32>,
    pub normals: SubAllocatedGrowableBuffer<f32>,
    pub uvs: SubAllocatedGrowableBuffer<f32>,
    pub tangents: SubAllocatedGrowableBuffer<f32>,
    pub colors: SubAllocatedGrowableBuffer<f32>,
    pub indices: SubAllocatedGrowableBuffer<u32>,

    pub mesh_offsets: SlabBuffer<MeshOffsets>,
    // Skinning
    pub joint_indices: SubAllocatedGrowableBuffer<u8>,
    pub joint_weights: SubAllocatedGrowableBuffer<f32>,
    pub pre_skinned_positions: SubAllocatedGrowableBuffer<f32>,
    pub pre_skinned_normals: SubAllocatedGrowableBuffer<f32>,
    pub pre_skinned_tangents: SubAllocatedGrowableBuffer<f32>,
    pub skin_bones: SubAllocatedGrowableBuffer<Mat4>,
    pub skin_instances: CompactSlabBuffer<SkinInstance>,
    pub skin_instance_data: HashMap<InstanceKey, SkinInstanceData>,

    pub instances: InstanceBuffer,
}

impl MeshState {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        Self {
            meshes: Slab::new(),
            positions: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            normals: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            uvs: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            tangents: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            colors: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            indices: SubAllocatedGrowableBuffer::new(
                queue,
                BufferUsage::STORAGE | BufferUsage::INDEX,
            ),
            mesh_offsets: SlabBuffer::new(BufferUsage::STORAGE),
            joint_indices: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            joint_weights: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            pre_skinned_positions: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            pre_skinned_normals: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            pre_skinned_tangents: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            skin_bones: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            skin_instances: CompactSlabBuffer::new(BufferUsage::STORAGE),
            skin_instance_data: HashMap::new(),
            instances: InstanceBuffer::new(),
        }
    }

    pub fn create_mesh_unchecked(&mut self, queue: &CommandQueue<'_>, mesh: &Mesh) -> MeshKey {
        let _span = trace_span!("MeshState::create_mesh").entered();

        // FIXME: What if element count is not equal across all attributes?
        let vertex_count = mesh.positions().len();

        if cfg!(debug_assertions) {
            mesh.validate_indices().unwrap();
        }

        let indices = mesh.indicies().unwrap().clone().into_u32();
        let indices_offset = self.indices.insert(queue, &indices);

        // If no colors are set we set all colors to 1.0
        // which corresponds to no effect.
        // TODO: Instead of duplicating a bunch of 1.0s for each mesh
        // we should share them between meshes.
        let mut colors = mesh.colors().to_vec();
        colors.resize(mesh.positions().len(), Vec4::ONE);

        // If no colors are set we set all colors to 1.0
        // which corresponds to no effect.
        // TODO: Instead of duplicating a bunch of 1.0s for each mesh
        // we should share them between meshes.
        let mut colors = mesh.colors().to_vec();
        colors.resize(mesh.positions().len(), Vec4::ONE);

        let colors_offset = self
            .colors
            .insert(queue, bytemuck::must_cast_slice::<Vec4, f32>(&colors));
        let uvs_offset = self
            .uvs
            .insert(queue, bytemuck::must_cast_slice(mesh.uvs()));

        // Skinned meshes are special in that we can't reuse the same
        // position/normal/tangent buffer for all instances since they
        // may be transformed differently across instances.
        // This means we need to defer mesh creation until instance creation
        // and duplicate the position/normal/tangent buffers for each instance.
        let data = if mesh.is_skinned() {
            let pre_skinned_positions_offset = self
                .pre_skinned_positions
                .insert(queue, bytemuck::must_cast_slice(mesh.positions()));
            let pre_skinned_normals_offset = self
                .pre_skinned_normals
                .insert(queue, bytemuck::must_cast_slice(mesh.normals()));
            let pre_skinned_tangents_offset = self
                .pre_skinned_tangents
                .insert(queue, bytemuck::must_cast_slice(mesh.tangents()));

            let joint_indices_offset = self.joint_indices.insert(
                queue,
                bytemuck::must_cast_slice(mesh.skinning_joint_indices()),
            );
            let joint_weights_offset = self.joint_weights.insert(
                queue,
                bytemuck::must_cast_slice(mesh.skinning_joint_weights()),
            );

            MeshData::Skinned(SkinnedMeshData {
                pre_skinned_positions_offset,
                pre_skinned_normals_offset,
                pre_skinned_tangents_offset,
                uvs_offset,
                colors_offset,
                joint_indices_offset,
                joint_weights_offset,
                indices_offset: indices_offset,
                indices_count: indices.len() as u64,
                vertex_count,
                min_bone_count: compute_skinned_mesh_bone_count(mesh),
            })
        } else {
            let positions_offset = self
                .positions
                .insert(queue, bytemuck::must_cast_slice(mesh.positions()));
            let normals_offset = self
                .normals
                .insert(queue, bytemuck::must_cast_slice(mesh.normals()));
            let tangents_offset = self
                .tangents
                .insert(queue, bytemuck::must_cast_slice(mesh.tangents()));

            let offsets = self.mesh_offsets.insert(&MeshOffsets {
                positions_offset: positions_offset as u32,
                normals_offset: normals_offset as u32,
                uvs_offset: uvs_offset as u32,
                tangents_offset: tangents_offset as u32,
                colors_offset: colors_offset as u32,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            });

            MeshData::Unskinned(UnskinnedMeshData {
                positions_offset,
                normals_offset,
                uvs_offset,
                tangents_offset,
                colors_offset,
                indices_offset,
                indices_count: indices.len() as u64,
                offsets,
            })
        };

        self.meshes.insert(data)
    }

    pub fn remove_mesh(&mut self, key: MeshKey) {
        let Some(mesh) = self.meshes.remove(key) else {
            return;
        };

        match mesh {
            MeshData::Unskinned(mesh) => {
                self.positions.remove(mesh.positions_offset);
                self.normals.remove(mesh.normals_offset);
                self.uvs.remove(mesh.uvs_offset);
                self.tangents.remove(mesh.tangents_offset);
                self.colors.remove(mesh.colors_offset);
                self.indices.remove(mesh.indices_offset);
                self.mesh_offsets.remove(mesh.offsets);
            }
            MeshData::Skinned(mesh) => {
                self.pre_skinned_positions
                    .remove(mesh.pre_skinned_positions_offset);
                self.pre_skinned_normals
                    .remove(mesh.pre_skinned_normals_offset);
                self.pre_skinned_tangents
                    .remove(mesh.pre_skinned_tangents_offset);

                self.colors.remove(mesh.colors_offset);
                self.uvs.remove(mesh.uvs_offset);
                self.indices.remove(mesh.indices_offset);
            }
        }
    }

    pub fn create_instance(
        &mut self,
        queue: &CommandQueue<'_>,
        transform: Transform,
        mesh: MeshKey,
        material: &MaterialState,
    ) -> InstanceKey {
        let mesh = self.meshes.get(mesh).unwrap();

        let normal = Mat3::from_quat(transform.rotation);
        let normal_x = Vec4::new(normal.x_axis.x, normal.x_axis.y, normal.x_axis.z, 0.0);
        let normal_y = Vec4::new(normal.y_axis.x, normal.y_axis.y, normal.y_axis.z, 0.0);
        let normal_z = Vec4::new(normal.z_axis.x, normal.z_axis.y, normal.z_axis.z, 0.0);

        let mut skinned_instance = None;

        let instance = match mesh {
            MeshData::Unskinned(mesh) => Instance {
                transform: Mat4::from_scale_rotation_translation(
                    transform.scale,
                    transform.rotation,
                    transform.translation,
                )
                .to_cols_array_2d(),
                normal: [
                    normal_x.to_array(),
                    normal_y.to_array(),
                    normal_z.to_array(),
                ],
                material_index: material.index,
                offsets: mesh.offsets,
                index_offset: mesh.indices_offset as u32,
                index_count: mesh.indices_count as u32,
            },
            MeshData::Skinned(mesh) => {
                let positions_offset = self.positions.alloc(queue, mesh.vertex_count * 3);
                let normals_offset = self.normals.alloc(queue, mesh.vertex_count * 3);
                let tangents_offset = self.tangents.alloc(queue, mesh.vertex_count * 4);

                // Fill initial bone array with identity transforms.
                // This is required in case the caller never updates the transforms in
                // this frame.
                let bones_offset = self.skin_bones.insert(
                    queue,
                    &std::iter::repeat_n(Mat4::IDENTITY, mesh.min_bone_count.into())
                        .collect::<Vec<_>>(),
                );

                let offsets = self.mesh_offsets.insert(&MeshOffsets {
                    positions_offset: positions_offset as u32,
                    normals_offset: normals_offset as u32,
                    uvs_offset: mesh.uvs_offset as u32,
                    tangents_offset: tangents_offset as u32,
                    colors_offset: mesh.colors_offset as u32,
                    _pad0: 0,
                    _pad1: 0,
                    _pad2: 0,
                });

                let index = self.skin_instances.insert(&SkinInstance {
                    bone_offset: bones_offset as u32,
                    vertex_count: mesh.vertex_count as u32,
                    positions_in_offset: mesh.pre_skinned_positions_offset as u32,
                    normals_in_offset: mesh.pre_skinned_normals_offset as u32,
                    tangents_in_offset: mesh.pre_skinned_tangents_offset as u32,
                    positions_out_offset: positions_offset as u32,
                    normals_out_offset: normals_offset as u32,
                    tangents_out_offset: tangents_offset as u32,
                    indices_offset: mesh.joint_indices_offset as u32,
                    weights_offset: mesh.joint_weights_offset as u32,
                });

                skinned_instance = Some(SkinInstanceData {
                    positions_offset,
                    normals_offset,
                    tangents_offset,
                    min_bone_count: mesh.min_bone_count,
                    bones_offset,
                    index,
                    vertex_count: mesh.vertex_count as u32,
                    mesh_offsets: offsets,
                });

                Instance {
                    transform: Mat4::from_scale_rotation_translation(
                        transform.scale,
                        transform.rotation,
                        transform.translation,
                    )
                    .to_cols_array_2d(),
                    normal: [
                        normal_x.to_array(),
                        normal_y.to_array(),
                        normal_z.to_array(),
                    ],
                    material_index: material.index,
                    offsets,
                    index_offset: mesh.indices_offset as u32,
                    index_count: mesh.indices_count as u32,
                }
            }
        };

        let key = self.instances.insert(material.alpha_mode, instance);

        if let Some(data) = skinned_instance {
            self.skin_instance_data.insert(key, data);
        }

        key
    }

    pub fn remove_instance(&mut self, key: InstanceKey) {
        self.instances.remove(key);

        if let Some(data) = self.skin_instance_data.remove(&key) {
            self.positions.remove(data.positions_offset);
            self.normals.remove(data.normals_offset);
            self.tangents.remove(data.tangents_offset);
            self.skin_instances.remove(data.index);
            self.skin_bones.remove(data.bones_offset);
            self.mesh_offsets.remove(data.mesh_offsets);
        }
    }

    pub fn set_skinned_bones_transforms(
        &mut self,
        queue: &CommandQueue<'_>,
        instance: InstanceKey,
        transforms: &[Transform],
    ) {
        let Some(data) = self.skin_instance_data.get(&instance) else {
            return;
        };

        let matrices = transforms
            .iter()
            .map(|v| v.compute_matrix())
            .take(data.min_bone_count.into())
            .collect::<Vec<_>>();

        self.skin_bones.write(queue, data.bones_offset, &matrices);
    }

    pub fn update_transform(&mut self, key: InstanceKey, transform: Transform) {
        self.instances.update_transform(key, transform);
    }
}

define_slab_key! {
    pub struct MeshKey: usize;

}

impl Index for MeshKey {
    fn index(&self) -> usize {
        self.0.index()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MeshData {
    Unskinned(UnskinnedMeshData),
    Skinned(SkinnedMeshData),
}

#[derive(Copy, Clone, Debug)]
pub struct UnskinnedMeshData {
    positions_offset: u64,
    normals_offset: u64,
    uvs_offset: u64,
    tangents_offset: u64,
    colors_offset: u64,
    indices_offset: u64,
    indices_count: u64,
    offsets: SlabIndex,
}

#[derive(Copy, Clone, Debug)]
pub struct SkinnedMeshData {
    pre_skinned_positions_offset: u64,
    pre_skinned_normals_offset: u64,
    pre_skinned_tangents_offset: u64,
    uvs_offset: u64,
    colors_offset: u64,
    joint_indices_offset: u64,
    joint_weights_offset: u64,
    indices_offset: u64,
    indices_count: u64,

    vertex_count: usize,
    min_bone_count: u8,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MeshOffsets {
    positions_offset: u32,
    normals_offset: u32,
    uvs_offset: u32,
    tangents_offset: u32,
    colors_offset: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

impl GpuBuffer for MeshOffsets {
    const ALIGN: usize = 4;
    const SIZE: usize = size_of::<Self>();
}

/// Computes the minimum number of bones required to skin the mesh.
fn compute_skinned_mesh_bone_count(mesh: &Mesh) -> u8 {
    mesh.skinning_joint_indices()
        .iter()
        .flatten()
        .copied()
        .max()
        // Need to add one to go from max-index to count.
        .map(|v| v.checked_add(1).unwrap())
        .unwrap_or_default()
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct SkinInstance {
    bone_offset: u32,
    vertex_count: u32,
    positions_in_offset: u32,
    normals_in_offset: u32,
    tangents_in_offset: u32,
    positions_out_offset: u32,
    normals_out_offset: u32,
    tangents_out_offset: u32,
    indices_offset: u32,
    weights_offset: u32,
}

impl GpuBuffer for SkinInstance {
    const SIZE: usize = size_of::<SkinInstance>();
    const ALIGN: usize = align_of::<SkinInstance>();
}

#[derive(Clone, Debug)]
pub struct SkinInstanceData {
    positions_offset: u64,
    normals_offset: u64,
    tangents_offset: u64,
    index: SlabIndex,
    bones_offset: u64,
    min_bone_count: u8,
    pub vertex_count: u32,
    mesh_offsets: SlabIndex,
}

#[derive(Debug)]
pub struct InstanceBuffer {
    pub opaque: CompactSlabBuffer<Instance>,
    pub transparent: CompactSlabBuffer<Instance>,
    pub translucent: CompactSlabBuffer<Instance>,
    pub num_opaque: u32,
    pub num_transparent: u32,
    pub num_translucent: u32,
}

impl InstanceBuffer {
    pub fn new() -> Self {
        Self {
            opaque: CompactSlabBuffer::new(BufferUsage::STORAGE),
            transparent: CompactSlabBuffer::new(BufferUsage::STORAGE),
            translucent: CompactSlabBuffer::new(BufferUsage::STORAGE),
            num_opaque: 0,
            num_transparent: 0,
            num_translucent: 0,
        }
    }

    pub fn insert(&mut self, mode: AlphaMode, instance: Instance) -> InstanceKey {
        match mode {
            AlphaMode::Opaque => {
                self.num_opaque += 1;
                InstanceKey::Opaque(self.opaque.insert(&instance))
            }
            AlphaMode::Mask(_) => {
                self.num_transparent += 1;
                InstanceKey::Transparent(self.transparent.insert(&instance))
            }
            AlphaMode::Blend => {
                self.num_translucent += 1;
                InstanceKey::Translucent(self.translucent.insert(&instance))
            }
        }
    }

    pub fn update_transform(&mut self, key: InstanceKey, transform: Transform) {
        let normal = Mat3::from_quat(transform.rotation);
        let normal_x = Vec4::new(normal.x_axis.x, normal.x_axis.y, normal.x_axis.z, 0.0);
        let normal_y = Vec4::new(normal.y_axis.x, normal.y_axis.y, normal.y_axis.z, 0.0);
        let normal_z = Vec4::new(normal.z_axis.x, normal.z_axis.y, normal.z_axis.z, 0.0);

        let transform = InstanceTransform {
            transform: Mat4::from_scale_rotation_translation(
                transform.scale,
                transform.rotation,
                transform.translation,
            )
            .to_cols_array_2d(),
            normal: [
                normal_x.to_array(),
                normal_y.to_array(),
                normal_z.to_array(),
            ],
        };
        let bytes = bytemuck::bytes_of(&transform);

        match key {
            InstanceKey::Opaque(index) => {
                self.opaque.write_bytes(index, bytes);
            }
            InstanceKey::Transparent(index) => {
                self.transparent.write_bytes(index, bytes);
            }
            InstanceKey::Translucent(index) => {
                self.translucent.write_bytes(index, bytes);
            }
        }
    }

    pub fn remove(&mut self, key: InstanceKey) {
        match key {
            InstanceKey::Opaque(index) => {
                self.num_opaque -= 1;
                self.opaque.remove(index);
            }
            InstanceKey::Transparent(index) => {
                self.num_transparent -= 1;
                self.transparent.remove(index);
            }
            InstanceKey::Translucent(index) => {
                self.num_translucent -= 1;
                self.translucent.remove(index);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct InstanceTransform {
    transform: [[f32; 4]; 4],
    // rotation matrix for normals/tangents
    // Note that we can't use the transform matrix for non-uniform
    // scaling values.
    normal: [[f32; 4]; 3],
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Instance {
    transform: [[f32; 4]; 4],
    // rotation matrix for normals/tangents
    // Note that we can't use the transform matrix for non-uniform
    // scaling values.
    normal: [[f32; 4]; 3],
    material_index: SlabIndex,
    offsets: SlabIndex,
    index_offset: u32,
    index_count: u32,
}

impl GpuBuffer for Instance {
    const ALIGN: usize = 16;
    const SIZE: usize = size_of::<Self>();
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InstanceKey {
    Opaque(SlabIndex),
    Transparent(SlabIndex),
    Translucent(SlabIndex),
}
