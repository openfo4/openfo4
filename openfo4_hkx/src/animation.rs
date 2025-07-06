mod spline_compressed;

use glam::Vec4;
use openfo4_hkx_derive::HkStruct;

use crate::{Array, Decode, Error, OpaquePtr, Ptr, Reader, ReferencedObject, Transform};

#[derive(Clone, Debug, HkStruct)]
pub struct Animation {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub ty: AnimationType,
    // Offset: 20
    pub duration: f32,
    // Offset: 24
    pub num_of_transform_tracks: u32,
    // Offset: 28
    pub num_of_float_tracks: u32,
    // Offset: 32
    pub extracted_motion: Option<Ptr<ReferencedObject>>,
    // Offset: 40
    pub annotation_tracks: Array<AnnotationTrack>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AnimationType {
    Unknown,
    Interleaved,
    Mirrored,
    SplineCompressed,
    QuantizedCompressed,
    PredictiveCompressed,
    ReferencePose,
}

impl Decode for AnimationType {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match u32::decode(reader)? {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Interleaved),
            2 => Ok(Self::Mirrored),
            3 => Ok(Self::SplineCompressed),
            4 => Ok(Self::QuantizedCompressed),
            5 => Ok(Self::PredictiveCompressed),
            6 => Ok(Self::ReferencePose),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct AnnotationTrack {
    // Offset: 0
    pub name: String,
    // Offset: 8
    pub annotations: Array<AnnotationTrackAnnotation>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct AnnotationTrackAnnotation {
    // Offset: 0
    pub time: f32,
    _pad0: u32,
    // Offset: 8
    pub text: String,
}

#[derive(Clone, Debug, HkStruct)]
pub struct AnimationBinding {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub original_skeleton_name: String,
    // Offset: 24
    pub animation: Ptr<Animation>,
    // Offset: 32
    pub transform_track_to_bone_indices: Array<u16>,
    // Offset: 48
    pub float_tack_to_slot_indices: Array<u16>,
    // Offset: 64
    pub partition_indices: Array<u16>,
    // Offset: 80
    pub blend_hint: BlendHint,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendHint {
    Normal,
    AdditiveDeprecated,
    Additive,
}

impl Decode for BlendHint {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match u8::decode(reader)? {
            0 => Ok(Self::Normal),
            1 => Ok(Self::AdditiveDeprecated),
            2 => Ok(Self::Additive),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct SplineCompressedAnimation {
    // Offset: 0
    pub animation: Animation,
    // Offset: 56
    pub num_frames: u32,
    // Offset: 60
    pub num_blocks: u32,
    // Offset: 64
    pub max_frames_per_block: u32,
    // Offset: 68
    pub mask_and_quantization_size: u32,
    // Offset: 72
    pub block_duration: f32,
    // Offset: 76
    pub block_inverse_duration: f32,
    // Offset: 80
    pub frame_duration: f32,
    _pad0: u32,
    // Offset: 88
    pub block_offsets: Array<u32>,
    // Offset: 104
    pub float_block_offsets: Array<u32>,
    // Offset: 120
    pub transform_offsets: Array<u32>,
    // Offset: 136
    pub float_offsets: Array<u32>,
    // Offset: 152
    pub data: Array<u8>,
    // Offset: 168
    pub endian: u32,
}

impl SplineCompressedAnimation {
    pub fn decompress(&self) -> Vec<Transform> {
        spline_compressed::decompress(
            &self.data,
            self.animation.num_of_transform_tracks,
            self.num_blocks,
            self.num_frames,
            self.max_frames_per_block,
        )
        .unwrap()
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct AnimationContainer {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub skeletons: Array<OpaquePtr>,
    // Offset: 32
    pub animations: Array<OpaquePtr>,
    // Offset: 48
    pub bindings: Array<OpaquePtr>,
    // Offset: 64
    pub attachments: Array<OpaquePtr>,
    // Offset: 80
    pub skins: Array<OpaquePtr>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct AnimatedReferenceFrame {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub frame_type: i8,
}

#[derive(Clone, Debug, HkStruct)]
pub struct DefaultAnimatedReferenceFrame {
    pub animated_reference_frame: AnimatedReferenceFrame,
    // Offset: 32
    pub up: Vec4,
    // Offset: 48
    pub forward: Vec4,
    // Offset: 64
    pub duration: f32,
    // Offset: 72
    pub reference_frame_samples: Array<Vec4>,
}
