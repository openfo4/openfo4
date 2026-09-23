use bitflags::bitflags;
use bytes::Buf;
use glam::{FloatExt, Mat3, Vec3, Vec4};

use crate::{BlockRef, Decode, Error, HVec2, HVec3, HVec4, NiNodeFlags, Reader, Rgba, Sphere};

const MAX_PREALLOCATED_VERTEX_BUFFER_SIZE: u16 = 8192;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions: Option<Vec<Vec3>>,
    pub tex_coords: Option<Vec<HVec2>>,
    pub normals: Option<Vec<Vec3>>,
    pub tangents: Option<Vec<Vec4>>,
    pub colors: Option<Vec<Rgba<u8>>>,
    pub skinning: Option<MeshSkinning>,
    pub indices: Vec<[u16; 3]>,
}

#[derive(Clone, Debug)]
pub struct MeshSkinning {
    pub bone_weights: Vec<HVec4>,
    pub bone_indices: Vec<[u8; 4]>,
}

#[derive(Clone, Debug)]
pub struct BSTriShape {
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub flags: NiNodeFlags,
    pub translation: Vec3,
    pub rotation: Mat3,
    pub scale: f32,
    pub collision_object: Option<BlockRef>,
    pub bounding_sphere: Sphere,
    pub skin: Option<BlockRef>,
    pub shader_property: Option<BlockRef>,
    pub alpha_property: Option<BlockRef>,
    pub mesh: Mesh,
    pub segments: Option<MeshSegments>,
}

impl Decode for BSTriShape {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, crate::Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let extra_data_len = reader.get_u32()?;
        let extra_data = (0..extra_data_len)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        let controller = <Option<BlockRef>>::decode(reader)?;
        let flags = NiNodeFlags::from_bits_truncate(reader.get_u32()?);

        let translation = Vec3::decode(reader)?;
        let rotation = Mat3::decode(reader)?;
        let scale = reader.get_f32()?;

        let collision_object = <Option<BlockRef>>::decode(reader)?;
        let bounding_sphere = Sphere::decode(reader)?;

        let skin = <Option<BlockRef>>::decode(reader)?;
        let shader_property = <Option<BlockRef>>::decode(reader)?;
        let alpha_property = <Option<BlockRef>>::decode(reader)?;

        let vertex_desc = VertexDescriptor(reader.get_u64()?);
        let num_triangles = reader.get_u32()?;
        let num_vertices = reader.get_u16()?;
        let data_size = reader.get_u32()?;

        let init_buffer_size =
            usize::from(u16::min(num_vertices, MAX_PREALLOCATED_VERTEX_BUFFER_SIZE));
        let vertex_size = vertex_desc.vertex_size();

        let start = reader.buf.position();

        // The tangents are inteleaved after position, normal and bitangents
        // attributes.
        // FIXME: It would probably be better to just compute the tangents from
        // the bitangents and normals.
        let mut tangent_x = Vec::with_capacity(init_buffer_size);
        let mut tangent_y = Vec::with_capacity(init_buffer_size);
        let mut tangent_z = Vec::with_capacity(init_buffer_size);

        let positions = if vertex_desc.flags().contains(VertexFlags::VERTEX) {
            let mut positions = Vec::with_capacity(init_buffer_size);
            let offset = vertex_desc.attribute_offset(VertexAttribute::Position);

            for index in 0..num_vertices {
                let pos = start + u64::from(index) * vertex_size + offset;
                reader.buf.set_position(pos);

                if vertex_desc.flags().contains(VertexFlags::FULL_PRECISION) {
                    positions.push(Vec3::decode(reader)?);
                    tangent_x.push(reader.get_f32()?);
                } else {
                    positions.push(HVec3::decode(reader)?.as_vec3());
                    tangent_x.push(reader.get_f16()?.to_f32());
                }
            }

            Some(positions)
        } else {
            None
        };

        let normals = if vertex_desc.flags().contains(VertexFlags::NORMAL) {
            let mut normals = Vec::with_capacity(init_buffer_size);
            let offset = vertex_desc.attribute_offset(VertexAttribute::Normal);

            for index in 0..num_vertices {
                let pos = start + u64::from(index) * vertex_size + offset;
                reader.buf.set_position(pos);

                let x = f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0);
                let y = f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0);
                let z = f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0);
                normals.push(Vec3::new(x, y, z).normalize());

                tangent_y.push(f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0));
            }

            Some(normals)
        } else {
            None
        };

        let tangents = if vertex_desc.flags().contains(VertexFlags::TANGENT) {
            let mut bitangents = Vec::with_capacity(init_buffer_size);
            let offset = vertex_desc.attribute_offset(VertexAttribute::Tangent);

            // The tangent values in the shape are actually the bitangents.
            for index in 0..num_vertices {
                let pos = start + u64::from(index) * vertex_size + offset;
                reader.buf.set_position(pos);

                let x = f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0);
                let y = f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0);
                let z = f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0);
                let bitangent = Vec3::new(x, y, z);
                bitangents.push(bitangent);

                tangent_z.push(f32::lerp(-1.0, 1.0, reader.get_u8()? as f32 / 255.0));
            }

            debug_assert_eq!(tangent_x.len(), tangent_y.len());
            debug_assert_eq!(tangent_x.len(), tangent_z.len());
            debug_assert_eq!(tangent_x.len(), bitangents.len());

            let mut tangents = Vec::with_capacity(tangent_x.len());
            for (index, (((x, y), z), bitangent)) in tangent_x
                .into_iter()
                .zip(tangent_y)
                .zip(tangent_z)
                .zip(bitangents)
                .enumerate()
            {
                let tangent = Vec3::new(x, y, z);
                let normal = normals
                    .as_ref()
                    .map(|normals| normals[index])
                    .unwrap_or(Vec3::Z);

                let tangent_sign = -(1.0_f32.copysign(normal.cross(tangent).z * bitangent.z));
                tangents.push(tangent.extend(tangent_sign));
            }

            Some(tangents)
        } else {
            None
        };

        let tex_coords = if vertex_desc.flags().contains(VertexFlags::TEX_COORD_0) {
            let mut tex_coords = Vec::with_capacity(init_buffer_size);
            let offset = vertex_desc.attribute_offset(VertexAttribute::TexCoord0);

            for index in 0..num_vertices {
                let pos = start + u64::from(index) * vertex_size + offset;
                reader.buf.set_position(pos);
                tex_coords.push(HVec2::decode(reader)?);
            }

            Some(tex_coords)
        } else {
            None
        };

        let colors = if vertex_desc.flags().contains(VertexFlags::COLOR) {
            let mut colors = Vec::with_capacity(init_buffer_size);
            let offset = vertex_desc.attribute_offset(VertexAttribute::Color);

            for index in 0..num_vertices {
                let pos = start + u64::from(index) * vertex_size + offset;
                reader.buf.set_position(pos);
                colors.push(Rgba::<u8>::decode(reader)?);
            }

            Some(colors)
        } else {
            None
        };

        let skinning = if vertex_desc.flags().contains(VertexFlags::SKINNING) {
            let mut bone_weights = Vec::with_capacity(init_buffer_size);
            let mut bone_indices = Vec::with_capacity(init_buffer_size);
            let offset = vertex_desc.attribute_offset(VertexAttribute::Skinning);

            for index in 0..num_vertices {
                let pos = start + u64::from(index) * vertex_size + offset;
                reader.buf.set_position(pos);

                bone_weights.push(HVec4::decode(reader)?);
                bone_indices.push(<[u8; 4]>::decode(reader)?);
            }

            Some(MeshSkinning {
                bone_weights,
                bone_indices,
            })
        } else {
            None
        };

        reader
            .buf
            .set_position(start + u64::from(num_vertices) * vertex_size);

        let mut indices = Vec::new();
        // For some reason there are sometimes meshes that whose data_size
        // is zero, but they report a non-zero index count?
        if data_size != 0 {
            for _ in 0..num_triangles {
                let a = reader.get_u16()?;
                let b = reader.get_u16()?;
                let c = reader.get_u16()?;
                indices.push([a, b, c]);
            }
        }

        // FIXME: BSMeshLODTriShape has the MESH_LOD flag set,
        // need to check whether it is possible for both segments and to be present
        // and what order they have then.
        let mut segments = None;
        if reader.buf.has_remaining() && !flags.contains(NiNodeFlags::MESH_LOD) {
            segments = Some(MeshSegments::decode(reader)?);
        }

        Ok(Self {
            name,
            extra_data,
            controller,
            flags,
            translation,
            rotation,
            scale,
            collision_object,
            bounding_sphere,
            skin,
            shader_property,
            alpha_property,
            mesh: Mesh {
                positions,
                tex_coords,
                normals,
                tangents,
                indices,
                colors,
                skinning,
            },
            segments,
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct VertexDescriptor(u64);

impl VertexDescriptor {
    const fn flags(&self) -> VertexFlags {
        VertexFlags::from_bits_truncate(self.0 >> 44)
    }

    const fn vertex_size(&self) -> u64 {
        (self.0 & 0xf) * 4
    }

    const fn attribute_offset(&self, attr: VertexAttribute) -> u64 {
        (self.0 >> (4 * attr as u8 + 2)) & 0x3c
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum VertexAttribute {
    Position = 0,
    TexCoord0 = 1,
    TexCoord1 = 2,
    Normal = 3,
    Tangent = 4,
    Color = 5,
    Skinning = 6,
    Landdata = 7,
    EyeData = 8,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct VertexFlags: u64 {
        const VERTEX = 1 << 0;
        const TEX_COORD_0 = 1 << 1;
        const TEX_COORD_1 = 1 << 2;
        const NORMAL = 1 << 3;
        const TANGENT = 1 << 4;
        const COLOR = 1 << 5;
        const SKINNING = 1 << 6;
        const LANDDATA = 1 << 7;
        const EYEDATA = 1 << 8;
        /// If set position have 32-bit float values.
        const FULL_PRECISION = 0x400;
    }
}

#[derive(Clone, Debug)]
pub struct BSSubIndexTriShape {
    pub shape: BSTriShape,
}

impl Decode for BSSubIndexTriShape {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let shape = BSTriShape::decode(reader)?;

        Ok(Self { shape })
    }
}

#[derive(Clone, Debug)]
pub struct BSMeshLODTriShape {
    pub shape: BSTriShape,
}

impl Decode for BSMeshLODTriShape {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let shape = BSTriShape::decode(reader)?;

        Ok(Self { shape })
    }
}

#[derive(Clone, Debug)]
pub struct MeshSegments {
    pub num_primitives: u32,
    pub segments: Vec<Segment>,
    pub ssf_file: Option<String>,
}

impl Decode for MeshSegments {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let num_primitives = reader.get_u32()?;
        let num_segments = reader.get_u32()?;
        let total_segments = reader.get_u32()?;

        let mut segments: Vec<_> = (0..num_segments)
            .map(|_| Segment::decode(reader))
            .collect::<Result<_, _>>()?;

        let mut ssf_file = None;
        if reader.buf.has_remaining() {
            // If present has the same counst as the previous segments.
            let _num_segments = reader.get_u32()?;
            let _total_segments = reader.get_u32()?;

            for _ in 0..num_segments {
                reader.get_u32()?;
            }

            let data: Vec<_> = (0..total_segments)
                .map(|_| SegmentData::decode(reader))
                .collect::<Result<_, _>>()?;

            let mut data = data.into_iter();
            for seg in &mut segments {
                seg.data = data.next();
                for subseg in &mut seg.subsegments {
                    subseg.data = data.next();
                }
            }

            ssf_file = Some(reader.get_u16_str()?.to_owned());
        }

        Ok(Self {
            num_primitives,
            segments,
            ssf_file,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub start_index: u32,
    pub num_primitives: u32,
    pub subsegments: Vec<SubSegment>,
    pub data: Option<SegmentData>,
}

impl Decode for Segment {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let start_index = reader.get_u32()?;
        let num_primitives = reader.get_u32()?;

        // Always `u32::MAX` since we are a top-level element.
        let _parent_array_index = reader.get_u32()?;

        let num_subsegments = reader.get_u32()?;
        let subsegments = (0..num_subsegments)
            .map(|_| SubSegment::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            start_index,
            num_primitives,
            subsegments,
            data: None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SubSegment {
    pub start_index: u32,
    pub num_primitives: u32,
    pub data: Option<SegmentData>,
}

impl Decode for SubSegment {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let start_index = reader.get_u32()?;
        let num_primitives = reader.get_u32()?;

        let _parent_array_index = reader.get_u32()?;
        let _unused = reader.get_u32()?;

        Ok(Self {
            start_index,
            num_primitives,
            data: None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SegmentData {
    pub user_index: u32,
    pub bone_id: u32,
    pub cut_offsets: Vec<f32>,
}

impl Decode for SegmentData {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let user_index = reader.get_u32()?;
        let bone_id = reader.get_u32()?;

        let num_cut_offsets = reader.get_u32()?;
        let cut_offsets = (0..num_cut_offsets)
            .map(|_| reader.get_f32())
            .collect::<Result<_, _>>()?;

        Ok(Self {
            user_index,
            bone_id,
            cut_offsets,
        })
    }
}
