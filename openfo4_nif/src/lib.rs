pub mod bgsm;
pub mod connect_points;
pub mod material;
pub mod mesh;

use std::io::Cursor;
use std::str::FromStr;

use bitflags::bitflags;
use bstr::{BStr, BString};
use bytes::Buf;
use glam::{Mat3, Quat, Vec2, Vec3, Vec4};
use half::f16;
use material::{BSLightingShaderProperty, BSShaderTextureSet};
use mesh::{BSMeshLODTriShape, BSSubIndexTriShape, BSTriShape};
use thiserror::Error;

use crate::connect_points::{BSConnectPointChildren, BSConnectPointParents};
use crate::material::{BSEffectShaderProperty, BSSkyShaderProperty, NiAlphaProperty};

#[derive(Clone, Debug)]
pub struct Nif {
    header: Header,
    blocks: Vec<Block>,
}

impl Nif {
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        let mut buf = Cursor::new(buf);

        let header = Header::decode(&mut buf)?;

        let mut reader = Reader {
            buf,
            endian: header.endian,
            strings: Vec::new(),
        };

        let user_version = reader.get_u32()?;
        let num_blocks = reader.get_u32()?;

        let bs_header = BsHeader::decode(&mut reader)?;

        let num_block_types = reader.get_u16()?;

        let mut block_types_info = Vec::new();
        for _ in 0..num_block_types {
            let name = String::decode(&mut reader)?;
            let block_type = name
                .parse()
                .map_err(|BadBlockType| Error::BadBlockType(name.clone()))?;
            block_types_info.push(block_type);
        }

        let mut block_types = Vec::new();
        for _ in 0..num_blocks {
            let block_type_index = reader.get_u16().unwrap();
            block_types.push(block_types_info[block_type_index as usize]);
        }

        let mut block_sizes = Vec::new();
        for _ in 0..num_blocks {
            let block_size = reader.get_u32().unwrap();
            block_sizes.push(block_size);
        }

        let num_strings = reader.get_u32().unwrap();
        let max_string_length = reader.get_u32().unwrap();

        let mut strings = Vec::new();
        for _ in 0..num_strings {
            let s = String::decode(&mut reader)?;
            strings.push(s);
        }
        reader.strings = strings;

        let num_groups = reader.get_u32().unwrap();
        //GROUPS

        let mut blocks = Vec::new();
        for index in 0..num_blocks {
            let start_offset = reader.buf.position();

            let block_type = block_types[index as usize];
            let block_size = block_sizes[index as usize];

            let block = match block_type {
                BlockType::NiNode => Block::NiNode(NiNode::decode(&mut reader).unwrap()),
                BlockType::BSBehaviorGraphExtraData => {
                    let behaviour_graph_file = reader.get_u32().unwrap();

                    let controls_base_skeleton = reader.get_bool().unwrap();
                    Block::Skipped
                }
                BlockType::BSXFlags => {
                    let name = reader.get_u32().unwrap();

                    let flags = reader.get_u32().unwrap();
                    Block::BSXFlags(BSXFlags::from_bits_truncate(flags))
                }
                BlockType::BhkNPCollisionObject => {
                    let target = BlockRef::decode(&mut reader).unwrap();
                    let flags = reader.get_u16().unwrap();
                    let data = BlockRef::decode(&mut reader).unwrap();
                    let body_id = reader.get_u32().unwrap();
                    Block::BhkNPCollisionObject(BhkNPCollisionObject {
                        target,
                        flags,
                        data,
                        body_id,
                    })
                }
                BlockType::BhkPhysicsSystem => {
                    let data = reader.get_byte_array()?;
                    Block::BhkPhysicsSystem(BhkPhysicsSystem { data })
                }
                BlockType::BSTriShape => Block::BSTriShape(BSTriShape::decode(&mut reader)?),
                BlockType::BSSubIndexTriShape => {
                    Block::BSSubIndexTriShape(BSSubIndexTriShape::decode(&mut reader)?)
                }
                BlockType::BSMeshLODTriShape => {
                    Block::BSMeshLODTriShape(BSMeshLODTriShape::decode(&mut reader)?)
                }
                BlockType::BSLightingShaderProperty => {
                    Block::BSLightingShaderProperty(BSLightingShaderProperty::decode(&mut reader)?)
                }
                BlockType::BSEffectShaderProperty => {
                    Block::BSEffectShaderProperty(BSEffectShaderProperty::decode(&mut reader)?)
                }
                BlockType::BSSkyShaderProperty => {
                    Block::BSSkyShaderProperty(BSSkyShaderProperty::decode(&mut reader)?)
                }
                BlockType::BSShaderTextureSet => {
                    Block::BSShaderTextureSet(BSShaderTextureSet::decode(&mut reader)?)
                }
                BlockType::NiAlphaProperty => {
                    Block::NiAlphaProperty(NiAlphaProperty::decode(&mut reader)?)
                }
                BlockType::BSSkinInstance => {
                    let skeleton_root = BlockRef::decode(&mut reader).unwrap();
                    let data = BlockRef::decode(&mut reader).unwrap();
                    let num_bones = reader.get_u32().unwrap();
                    let bones = (0..num_bones)
                        .map(|_| BlockRef::decode(&mut reader))
                        .collect::<Result<_, _>>()
                        .unwrap();
                    let num_scales = reader.get_u32().unwrap();
                    assert_eq!(num_scales, 0, "todo");

                    Block::BSSkinInstance(BSSkinInstance {
                        skeleton_root,
                        data,
                        bones,
                    })
                }
                BlockType::BSSkinBoneData => {
                    let num_bones = reader.get_u32().unwrap();
                    let bones = (0..num_bones)
                        .map(|_| Bone::decode(&mut reader))
                        .collect::<Result<_, _>>()
                        .unwrap();

                    Block::BSSkinBoneData(BSSkinBoneData { bones })
                }
                BlockType::BSBound => {
                    let block = BSBound::decode(&mut reader)?;
                    Block::BSBound(block)
                }
                BlockType::BhkRagdollSystem => {
                    let data = reader.get_byte_array()?;
                    Block::BhkRagdollSystem(BhkRagdollSystem { data })
                }
                BlockType::BSConnectPointChildren => {
                    Block::BSConnectPointChildren(BSConnectPointChildren::decode(&mut reader)?)
                }
                BlockType::BSConnectPointParents => {
                    Block::BSConnectPointParents(BSConnectPointParents::decode(&mut reader)?)
                }
                _ => Block::Skipped,
            };

            blocks.push(block);

            reader.buf.set_position(start_offset + block_size as u64);
        }

        Ok(Self { header, blocks })
    }

    pub fn root(&self) -> Option<BlockRef> {
        if self.blocks.len() != 0 {
            Some(BlockRef(0))
        } else {
            None
        }
    }

    pub fn get(&self, block: BlockRef) -> Option<&Block> {
        self.blocks.get(block.0 as usize)
    }
}

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("unexpected eof")]
    Eof,
    #[error("bad magic")]
    BadMagic,
    #[error("invalid string value: {0}")]
    InvalidString(std::string::FromUtf8Error),
    #[error("invalid string pointer: {0}")]
    InvalidStringPointer(u32),
    #[error("bad block type: {0}")]
    BadBlockType(String),
    #[error("invalid blend mode: {0}")]
    InvalidBlendMode(u8),
    #[error("invalid alpha test function: {0}")]
    InvalidAlphaTestFunction(u8),
}

pub trait Decode: Sized {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error>;
}

#[derive(Clone, Debug)]
pub struct Header {
    version: [u8; 4],
    endian: Endianess,
}

impl Header {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let mut header_string = Vec::new();
        loop {
            if !buf.has_remaining() {
                return Err(Error::Eof);
            }

            let byte = buf.get_u8();

            if byte == 0x0a {
                break;
            }
            header_string.push(byte);
        }

        if !header_string.starts_with(b"Gamebryo File Format") {
            return Err(Error::BadMagic);
        }

        if buf.remaining() < 4 {
            return Err(Error::Eof);
        }

        // Version is reversed
        let mut version = [0; 4];
        buf.copy_to_slice(&mut version);
        version.reverse();

        if !buf.has_remaining() {
            return Err(Error::Eof);
        }

        let endian = match buf.get_u8() {
            0 => Endianess::Big,
            1 => Endianess::Little,
            _ => todo!(),
        };

        Ok(Self { version, endian })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Endianess {
    Little,
    Big,
}

#[derive(Clone, Debug)]
pub struct Reader<'a> {
    buf: Cursor<&'a [u8]>,
    endian: Endianess,
    strings: Vec<String>,
}

impl Reader<'_> {
    fn string(&self, index: u32) -> Result<&str, Error> {
        if index == u32::MAX {
            Ok("")
        } else {
            match self.strings.get(index as usize) {
                Some(s) => Ok(s),
                None => Err(Error::InvalidStringPointer(index)),
            }
        }
    }

    fn copy_to_bytes(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        if self.buf.remaining() < dst.len() {
            Err(Error::Eof)
        } else {
            self.buf.copy_to_slice(dst);
            Ok(())
        }
    }

    fn get_u8(&mut self) -> Result<u8, Error> {
        let mut arr = [0; 1];
        self.copy_to_bytes(&mut arr)?;
        Ok(arr[0])
    }

    fn get_u16(&mut self) -> Result<u16, Error> {
        let mut bytes = [0; size_of::<u16>()];
        self.copy_to_bytes(&mut bytes)?;

        match self.endian {
            Endianess::Big => Ok(u16::from_be_bytes(bytes)),
            Endianess::Little => Ok(u16::from_le_bytes(bytes)),
        }
    }

    fn get_u32(&mut self) -> Result<u32, Error> {
        let mut bytes = [0; size_of::<u32>()];
        self.copy_to_bytes(&mut bytes)?;

        match self.endian {
            Endianess::Big => Ok(u32::from_be_bytes(bytes)),
            Endianess::Little => Ok(u32::from_le_bytes(bytes)),
        }
    }

    fn get_u64(&mut self) -> Result<u64, Error> {
        let mut bytes = [0; size_of::<u64>()];
        self.copy_to_bytes(&mut bytes)?;

        match self.endian {
            Endianess::Big => Ok(u64::from_be_bytes(bytes)),
            Endianess::Little => Ok(u64::from_le_bytes(bytes)),
        }
    }

    fn get_f16(&mut self) -> Result<f16, Error> {
        let mut bytes = [0; size_of::<f16>()];
        self.copy_to_bytes(&mut bytes)?;

        match self.endian {
            Endianess::Big => Ok(f16::from_be_bytes(bytes)),
            Endianess::Little => Ok(f16::from_le_bytes(bytes)),
        }
    }

    fn get_f32(&mut self) -> Result<f32, Error> {
        let mut bytes = [0; size_of::<f32>()];
        self.copy_to_bytes(&mut bytes)?;

        match self.endian {
            Endianess::Big => Ok(f32::from_be_bytes(bytes)),
            Endianess::Little => Ok(f32::from_le_bytes(bytes)),
        }
    }

    fn get_bool(&mut self) -> Result<bool, Error> {
        self.get_u8().map(|v| v != 0)
    }

    fn get_byte_array(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.get_u32()?;
        let mut buf = vec![0; len as usize];
        self.copy_to_bytes(&mut buf)?;
        Ok(buf)
    }
}

#[derive(Clone, Debug)]
pub struct BsHeader {
    version: u32,
    // These fields should represent strings, but
    // sometimes contain garbage, so we handle them
    // as bytes instead.
    author: BString,
    process_script: BString,
    export_script: BString,
    max_filepath: BString,
}

impl Decode for BsHeader {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let version = reader.get_u32()?;

        let author_len = u8::decode(reader)?;
        let mut author = vec![0; author_len.into()];
        reader.copy_to_bytes(&mut author)?;

        let process_script_len = u8::decode(reader)?;
        let mut process_script = vec![0; process_script_len.into()];
        reader.copy_to_bytes(&mut process_script)?;

        let export_script_len = u8::decode(reader)?;
        let mut export_script = vec![0; export_script_len.into()];
        reader.copy_to_bytes(&mut export_script)?;

        let max_filepath_len = u8::decode(reader)?;
        let mut max_filepath = vec![0; max_filepath_len.into()];
        reader.copy_to_bytes(&mut max_filepath)?;

        Ok(Self {
            version,
            author: BString::new(author),
            process_script: BString::new(process_script),
            export_script: BString::new(export_script),
            max_filepath: BString::new(max_filepath),
        })
    }
}

impl Decode for String {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let len = reader.get_u32()?;
        let mut bytes = vec![0; len as usize];
        reader.copy_to_bytes(&mut bytes)?;

        Self::from_utf8(bytes).map_err(Error::InvalidString)
    }
}

#[derive(Clone, Debug)]
pub enum Block {
    Skipped,
    NiNode(NiNode),
    BSXFlags(BSXFlags),
    BSTriShape(BSTriShape),
    BSSubIndexTriShape(BSSubIndexTriShape),
    BSMeshLODTriShape(BSMeshLODTriShape),
    BSLightingShaderProperty(BSLightingShaderProperty),
    BSEffectShaderProperty(BSEffectShaderProperty),
    BSShaderTextureSet(BSShaderTextureSet),
    BhkNPCollisionObject(BhkNPCollisionObject),
    BhkPhysicsSystem(BhkPhysicsSystem),
    BSSkinInstance(BSSkinInstance),
    BSSkinBoneData(BSSkinBoneData),
    BSBound(BSBound),
    BhkRagdollSystem(BhkRagdollSystem),
    BSSkyShaderProperty(BSSkyShaderProperty),
    NiAlphaProperty(NiAlphaProperty),
    BSConnectPointChildren(BSConnectPointChildren),
    BSConnectPointParents(BSConnectPointParents),
}

#[derive(Clone, Debug)]
pub struct NiNode {
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub flags: NiNodeFlags,
    pub translation: Vec3,
    pub rotation: Mat3,
    pub scale: f32,
    pub collision_object: Option<BlockRef>,
    pub children: Vec<BlockRef>,
}

impl Decode for NiNode {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
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

        let children_len = reader.get_u32()?;
        let children = (0..children_len)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            name,
            extra_data,
            controller,
            flags,
            translation,
            rotation,
            scale,
            collision_object,
            children,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Decode for Sphere {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let center = Vec3::decode(reader)?;
        let radius = reader.get_f32()?;
        Ok(Self { center, radius })
    }
}

#[derive(Clone, Debug)]
pub struct BhkNPCollisionObject {
    pub target: BlockRef,
    pub flags: u16,
    pub data: BlockRef,
    pub body_id: u32,
}

#[derive(Clone, Debug)]
pub struct BSSkinInstance {
    pub skeleton_root: BlockRef,
    pub data: BlockRef,
    pub bones: Vec<BlockRef>,
}

#[derive(Clone, Debug)]
pub struct BSSkinBoneData {
    pub bones: Vec<Bone>,
}

#[derive(Copy, Clone, Debug)]
pub struct Bone {
    pub bounding_sphere: Sphere,
    pub rotation: Mat3,
    pub translation: Vec3,
    pub scale: f32,
}

impl Decode for Bone {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let bounding_sphere = Sphere::decode(reader)?;
        let rotation = Mat3::decode(reader)?;
        let translation = Vec3::decode(reader)?;
        let scale = reader.get_f32()?;
        Ok(Self {
            bounding_sphere,
            rotation,
            translation,
            scale,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BhkPhysicsSystem {
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct BhkRagdollSystem {
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockRef(pub u32);

impl Decode for BlockRef {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.get_u32().map(Self)
    }
}

impl Decode for Option<BlockRef> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        BlockRef::decode(reader).map(|v| (v.0 != u32::MAX).then_some(v))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    NiNode,
    NiControllerManager,
    NiMultiTargetTransformController,
    NiControllerSequence,
    NiPoint3Interpolator,
    NiFloatInterpolator,
    NiPosData,
    NiBlendPoint3Interpolator,
    NiFloatData,
    NiBlendFloatInterpolator,
    NiTextKeyExtraData,
    NiDefaultAVObjectPalette,
    NiAlphaProperty,
    NiTransformInterpolator,
    NiTransformData,
    NiBoolTimelineInterpolator,
    NiPSysModifierActiveCtlr,
    NiBlendBoolInterpolator,
    NiBoolInterpolator,
    NiParticleSystem,
    NiPSysEmitterCtlr,
    NiPSysUpdateCtlr,
    NiBoolData,
    NiPSysData,
    NiPSysAgeDeathModifier,
    NiPSysMeshEmitter,
    NiPSysSpawnModifier,
    NiPSysRotationModifier,
    NiPSysGravityModifier,
    NiPSysDragModifier,
    NiPSysColliderManager,
    NiPSysPlanarCollider,
    NiPSysPositionModifier,
    NiPSysBoundUpdateModifier,
    NiPSysBoxEmitter,
    NiBillboardNode,
    NiPSysEmitterSpeedCtlr,
    NiVisController,
    NiPSysEmitterLifeSpanCtlr,
    NiPathInterpolator,
    NiPSysCylinderEmitter,
    NiPSysBombModifier,
    NiTransformController,
    BSBehaviorGraphExtraData,
    BSXFlags,
    BhkNPCollisionObject,
    BhkPhysicsSystem,
    BSTriShape,
    BSLightingShaderProperty,
    BSShaderTextureSet,
    BSEffectShaderPropertyColorController,
    BSEffectShaderPropertyFloatController,
    BSEffectShaderProperty,
    BSPositionData,
    BSPSysLODModifier,
    BSPSysSimpleColorModifier,
    BSPSysSubTexModifier,
    BSPSysScaleModifier,
    BSNiAlphaPropertyTestRefController,
    BSMeshLODTriShape,
    BSOrderedNode,
    BSLightingShaderPropertyFloatController,
    BSSkinInstance,
    BSSkinBoneData,
    BSBlastNode,
    BSValueNode,
    BSSubIndexTriShape,
    BSLeafAnimNode,
    BSConnectPointParents,
    BSConnectPointChildren,
    BSBound,
    BSBoneLodExtraData,
    BhkRagdollSystem,
    BSSkyShaderProperty,
    BSDamageStage,
}

impl FromStr for BlockType {
    type Err = BadBlockType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NiNode" => Ok(Self::NiNode),
            "NiControllerManager" => Ok(Self::NiControllerManager),
            "NiMultiTargetTransformController" => Ok(Self::NiMultiTargetTransformController),
            "NiControllerSequence" => Ok(Self::NiControllerSequence),
            "NiPoint3Interpolator" => Ok(Self::NiPoint3Interpolator),
            "NiFloatInterpolator" => Ok(Self::NiFloatInterpolator),
            "NiPosData" => Ok(Self::NiPosData),
            "NiBlendPoint3Interpolator" => Ok(Self::NiBlendPoint3Interpolator),
            "NiFloatData" => Ok(Self::NiFloatData),
            "NiBlendFloatInterpolator" => Ok(Self::NiBlendFloatInterpolator),
            "NiTextKeyExtraData" => Ok(Self::NiTextKeyExtraData),
            "NiDefaultAVObjectPalette" => Ok(Self::NiDefaultAVObjectPalette),
            "NiAlphaProperty" => Ok(Self::NiAlphaProperty),
            "NiTransformInterpolator" => Ok(Self::NiTransformInterpolator),
            "NiTransformData" => Ok(Self::NiTransformData),
            "NiPSysModifierActiveCtlr" => Ok(Self::NiPSysModifierActiveCtlr),
            "NiBlendBoolInterpolator" => Ok(Self::NiBlendBoolInterpolator),
            "NiBoolTimelineInterpolator" => Ok(Self::NiBoolTimelineInterpolator),
            "NiBoolInterpolator" => Ok(Self::NiBoolInterpolator),
            "NiParticleSystem" => Ok(Self::NiParticleSystem),
            "NiPSysEmitterCtlr" => Ok(Self::NiPSysEmitterCtlr),
            "NiPSysUpdateCtlr" => Ok(Self::NiPSysUpdateCtlr),
            "NiBoolData" => Ok(Self::NiBoolData),
            "NiPSysData" => Ok(Self::NiPSysData),
            "NiPSysAgeDeathModifier" => Ok(Self::NiPSysAgeDeathModifier),
            "NiPSysMeshEmitter" => Ok(Self::NiPSysMeshEmitter),
            "NiPSysSpawnModifier" => Ok(Self::NiPSysSpawnModifier),
            "NiPSysRotationModifier" => Ok(Self::NiPSysRotationModifier),
            "NiPSysGravityModifier" => Ok(Self::NiPSysGravityModifier),
            "NiPSysDragModifier" => Ok(Self::NiPSysDragModifier),
            "NiPSysColliderManager" => Ok(Self::NiPSysColliderManager),
            "NiPSysPlanarCollider" => Ok(Self::NiPSysPlanarCollider),
            "NiPSysPositionModifier" => Ok(Self::NiPSysPositionModifier),
            "NiPSysBoundUpdateModifier" => Ok(Self::NiPSysBoundUpdateModifier),
            "NiPSysBoxEmitter" => Ok(Self::NiPSysBoxEmitter),
            "NiBillboardNode" => Ok(Self::NiBillboardNode),
            "NiPSysEmitterSpeedCtlr" => Ok(Self::NiPSysEmitterSpeedCtlr),
            "NiVisController" => Ok(Self::NiVisController),
            "NiPathInterpolator" => Ok(Self::NiPathInterpolator),
            "NiPSysEmitterLifeSpanCtlr" => Ok(Self::NiPSysEmitterLifeSpanCtlr),
            "NiPSysCylinderEmitter" => Ok(Self::NiPSysCylinderEmitter),
            "NiPSysBombModifier" => Ok(Self::NiPSysBombModifier),
            "NiTransformController" => Ok(Self::NiTransformController),
            "BSBehaviorGraphExtraData" => Ok(Self::BSBehaviorGraphExtraData),
            "BSXFlags" => Ok(Self::BSXFlags),
            "bhkNPCollisionObject" => Ok(Self::BhkNPCollisionObject),
            "bhkPhysicsSystem" => Ok(Self::BhkPhysicsSystem),
            "BSTriShape" => Ok(Self::BSTriShape),
            "BSLightingShaderProperty" => Ok(Self::BSLightingShaderProperty),
            "BSShaderTextureSet" => Ok(Self::BSShaderTextureSet),
            "BSEffectShaderPropertyColorController" => {
                Ok(Self::BSEffectShaderPropertyColorController)
            }
            "BSEffectShaderPropertyFloatController" => {
                Ok(Self::BSEffectShaderPropertyFloatController)
            }
            "BSEffectShaderProperty" => Ok(Self::BSEffectShaderProperty),
            "BSPositionData" => Ok(Self::BSPositionData),
            "BSPSysLODModifier" => Ok(Self::BSPSysLODModifier),
            "BSPSysSimpleColorModifier" => Ok(Self::BSPSysSimpleColorModifier),
            "BSPSysSubTexModifier" => Ok(Self::BSPSysSubTexModifier),
            "BSPSysScaleModifier" => Ok(Self::BSPSysScaleModifier),
            "BSNiAlphaPropertyTestRefController" => Ok(Self::BSNiAlphaPropertyTestRefController),
            "BSMeshLODTriShape" => Ok(Self::BSMeshLODTriShape),
            "BSOrderedNode" => Ok(Self::BSOrderedNode),
            "BSLightingShaderPropertyFloatController" => {
                Ok(Self::BSLightingShaderPropertyFloatController)
            }
            "BSSkin::Instance" => Ok(Self::BSSkinInstance),
            "BSSkin::BoneData" => Ok(Self::BSSkinBoneData),
            "BSBlastNode" => Ok(Self::BSBlastNode),
            "BSValueNode" => Ok(Self::BSValueNode),
            "BSSubIndexTriShape" => Ok(Self::BSSubIndexTriShape),
            "BSLeafAnimNode" => Ok(Self::BSLeafAnimNode),
            "BSConnectPoint::Parents" => Ok(Self::BSConnectPointParents),
            "BSConnectPoint::Children" => Ok(Self::BSConnectPointChildren),
            "BSBound" => Ok(Self::BSBound),
            "BSBoneLODExtraData" => Ok(Self::BSBoneLodExtraData),
            "bhkRagdollSystem" => Ok(Self::BhkRagdollSystem),
            "BSSkyShaderProperty" => Ok(Self::BSSkyShaderProperty),
            "BSDamageStage" => Ok(Self::BSDamageStage),
            _ => Err(BadBlockType),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BadBlockType;

impl Decode for Vec3 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = reader.get_f32()?;
        let y = reader.get_f32()?;
        let z = reader.get_f32()?;
        Ok(Self::from_array([x, y, z]))
    }
}

impl Decode for Mat3 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = Vec3::decode(reader)?;
        let y = Vec3::decode(reader)?;
        let z = Vec3::decode(reader)?;
        Ok(Self::from_cols(x, y, z))
    }
}

impl Decode for Quat {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = reader.get_f32()?;
        let y = reader.get_f32()?;
        let z = reader.get_f32()?;
        let w = reader.get_f32()?;
        Ok(Self::from_xyzw(x, y, z, w))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VertexDesc {
    pub flags: VertexFlags,
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
        const FULL_PRECISION = 0x400;
    }
}

const DESC_MASK_OFFSET: u64 = 0xFFFFFF0000000000;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HVec3 {
    pub x: f16,
    pub y: f16,
    pub z: f16,
}

impl HVec3 {
    pub fn as_vec3(&self) -> Vec3 {
        Vec3 {
            x: self.x.to_f32(),
            y: self.y.to_f32(),
            z: self.z.to_f32(),
        }
    }
}

impl Decode for HVec3 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = reader.get_f16()?;
        let y = reader.get_f16()?;
        let z = reader.get_f16()?;
        Ok(Self { x, y, z })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HVec2 {
    pub x: f16,
    pub y: f16,
}

impl HVec2 {
    pub fn as_vec2(&self) -> Vec2 {
        Vec2 {
            x: self.x.to_f32(),
            y: self.y.to_f32(),
        }
    }
}

impl Decode for HVec2 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = reader.get_f16()?;
        let y = reader.get_f16()?;
        Ok(Self { x, y })
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

#[derive(Copy, Clone, Debug)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const fn to_array(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

impl Decode for Rgb {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let r = reader.get_f32()?;
        let g = reader.get_f32()?;
        let b = reader.get_f32()?;
        Ok(Self { r, g, b })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rgba<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl<T> Decode for Rgba<T>
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let r = T::decode(reader)?;
        let g = T::decode(reader)?;
        let b = T::decode(reader)?;
        let a = T::decode(reader)?;
        Ok(Self { r, g, b, a })
    }
}

impl Decode for Vec2 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = reader.get_f32().unwrap();
        let y = reader.get_f32().unwrap();
        Ok(Self { x, y })
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TextureClampMode: u32 {
        const WRAP_T = 0x1;
        const WRAP_S = 0x2;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct BSXFlags: u32 {
        const ANIMATED = 1 << 0;
        const HAVOK = 1 << 1;
        const RAGDOLL = 1 << 2;
        const COMPLEX = 1 << 3;
        const ADDON = 1 << 4;
        const EDITOR_MARKER = 1 << 5;
        const DYNAMIC = 1 << 6;
        const ARTICULATED = 1 << 7;
        const NEEDS_TRANSFORM_UPDATES = 1 << 8;
        const EXTERNAL_EMIT = 1 << 9;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NiNodeFlags: u32 {
        const HIDDEN = 1 << 0;
        const SELECTIVE_UPDATE = 1 << 1;
        const SELECTIVE_UPDATE_TRANSFORMS = 1 << 2;
        const SELECTIVE_UPDATE_CONTROLLER = 1 << 3;
        const SELECTIVE_UPDATE_RIGID = 1 << 4;
        const DISPLAY_UI_OBJECT = 1 << 5;
        const DISABLE_SORTING = 1 << 6;
        const SELECTIVE_UPDATE_TRANSFORMS_OVERRIDE = 1 << 7;
        const SAVE_EXTERNAL_GEOM_DATA = 1 << 9;
        const NO_DECALS = 1 << 10;
        const ALWAYS_DRAW = 1 << 11;
        const MESH_LOD = 1 << 12;
        const FIXED_BOUND = 1 << 13;
        const TOP_FADE_NODE = 1 << 14;
        const IGNORE_FADE = 1 << 15;
        const NO_ANIM_SYNC_X = 1 << 16;
        const NO_ANIM_SYNC_Y = 1 << 17;
        const NO_ANIM_SYNC_Z = 1 << 18;
        const NO_ANIM_SYNC_S = 1 << 19;
        const NO_DISMEMBER = 1 << 20;
        const NO_DISMEMBER_VALIDITY = 1 << 21;
        const RENDER_USE = 1 << 22;
        const MATERIALS_APPLIED = 1 << 23;
        const HIGH_DETAIL = 1 << 24;
        const FORCE_UPDATE = 1 << 25;
        const PRE_PROCESSED_NODE = 1 << 26;
    }
}

impl Decode for u8 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.get_u8()
    }
}

impl Decode for u16 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.get_u16()
    }
}

impl Decode for u32 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.get_u32()
    }
}

impl Decode for f32 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.get_f32()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HVec4 {
    pub x: f16,
    pub y: f16,
    pub z: f16,
    pub w: f16,
}

impl HVec4 {
    pub fn as_vec4(&self) -> Vec4 {
        Vec4::new(
            self.x.to_f32(),
            self.y.to_f32(),
            self.z.to_f32(),
            self.w.to_f32(),
        )
    }
}

impl Decode for HVec4 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x = reader.get_f16()?;
        let y = reader.get_f16()?;
        let z = reader.get_f16()?;
        let w = reader.get_f16()?;
        Ok(Self { x, y, z, w })
    }
}

impl<const N: usize> Decode for [u8; N] {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let mut arr = [0; N];
        reader.copy_to_bytes(&mut arr)?;
        Ok(arr)
    }
}

#[derive(Clone, Debug)]
pub struct BSBound {
    pub name: String,
    pub center: Vec3,
    pub bounds: Vec3,
}

impl Decode for BSBound {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let center = Vec3::decode(reader)?;
        let bounds = Vec3::decode(reader)?;

        Ok(Self {
            name,
            center,
            bounds,
        })
    }
}
