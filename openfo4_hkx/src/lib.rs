pub mod animation;
pub mod behavior;
pub mod character;
pub mod mesh;
pub mod physics;
pub mod project;
pub mod skeleton;

use std::collections::HashMap;
use std::ffi::CStr;
use std::fmt::Debug;
use std::io::Cursor;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::vec;

use animation::{
    AnimationBinding, AnimationContainer, DefaultAnimatedReferenceFrame, SplineCompressedAnimation,
};
use behavior::{
    BSAssignVariablesModifier, BSBehaviorGraphSwapGenerator, BSCyclicBlendTransitionGenerator,
    BSDirectAtCapturePoseModifier, BSDirectAtModifier, BSEventEveryNEventsModifier,
    BSEventOnDeactivateModifier, BSEventOnFalseToTrueModifier, BSGetTimeStepModifier,
    BSInterpValueModifier, BSIsActiveModifier, BSLookAtModifier, BSModifyOnceModifier,
    BSOffsetAnimationGenerator, BSRagdollContactListenerModifier, BSTimerModifier,
    BSiStateTaggingGenerator, BehaviorGraph, BehaviorGraphData, BehaviorGraphStringData,
    BehaviorReferenceGenerator, BlenderGenerator, BlenderGeneratorChild, BlendingTransitionEffect,
    BoneIndexArray, ClipGenerator, ClipTriggerArray, EvaluateExpressionModifier,
    EventDrivenModifier, EventRangeDataArray, EventsFromRangeModifier, ExpressionCondition,
    ExpressionDataArray, FootIkControlsModifier, Generator, GeneratorTransitionEffect,
    GetHandleOnBoneModifier, GetUpModifier, HandIkControlsModifier, KeyframesBonesModifier, Layer,
    LayerGenerator, ManualSelectorGenerator, MirrorModifier, ModifierGenerator, ModifierList, Node,
    PoseMatchingGenerator, PoweredRagdollControlsModifier, ReferencePoseGenerator,
    RigidBodyRagdollControlsModifier, StateMachine, StateMachineEventPropertyArray,
    StateMachineStateInfo, StateMachineTransitionInfoArray, StringEventPayload, TimerModifier,
    TwistModifier,
};
use bytes::Buf;
use character::{CharacterData, CharacterStringData};
use glam::{Mat3, Mat4, Quat, Vec3, Vec4};
use indexmap::IndexMap;
use mesh::{
    CapsuleShape, CompressedMeshShape, CompressedMeshShapeData, ConvexPolytopeShape,
    ScaledConvexShape,
};
use openfo4_hkx_derive::HkStruct;
use physics::{PhysicsSceneData, PhysicsSystemData, RefCountedProperties};
use project::{ProjectData, ProjectStringData};
use skeleton::{
    BoneWeightArray, BreakableConstraintData, FootIkDriverInfo, LimitedHingeConstraintData,
    MirroredSkeletionInfo, PositionConstraintMotor, RagdollConstraintData, RagdollData, Skeleton,
    SkeletonMapper, VariableBindingSet,
};
use thiserror::Error;

use crate::behavior::{BSRandomAlarmModifier, BSRootTwistModifier};
use crate::mesh::DynamicCompoundShape;
use crate::skeleton::HandIkDriverInfo;

#[derive(Clone, Debug, Error)]
#[error("invalid data at position {position}: {kind}")]
pub struct Error {
    kind: ErrorKind,
    position: u64,
    callstack: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
enum ErrorKind {
    #[error("unexpected eof: expected {expected} bytes, got {got}")]
    UnexpectedEof { expected: usize, got: usize },
    #[error("invalid local key: {0}")]
    InvalidLocalKey(u32),
    #[error("invalid global key: {0}")]
    InvalidGlobalKey(u32),
    #[error("invalid virtual key: {0}")]
    InvalidVirtualKey(u32),
    #[error("bad magic: {0:x}")]
    BadMagic(u64),
    #[error("no class key at location {0}")]
    InvalidClassName(u32),
    #[error("invalied string: {0}")]
    InvalidString(std::string::FromUtf8Error),
    #[error("unknown class name {1} (signature {0:x})")]
    UnknownClassName(u32, String),
    #[error("location at pointer is not zero")]
    PtrNotZero,
    #[error("unknown value {value} for enum {ty}")]
    UnknownEnumValue { ty: &'static str, value: String },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Endianess {
    Little,
    Big,
}

#[derive(Clone, Debug)]
struct Reader<'a> {
    cursor: Cursor<&'a [u8]>,
    class_names: &'a HashMap<u32, ClassName>,
    local_fixups: &'a HashMap<u32, u32>,
    global_fixups: &'a HashMap<u32, u32>,
    virtual_fixups: &'a IndexMap<u32, u32>,
    endian: Endianess,
    ptr_size: u8,
}

impl<'a> Reader<'a> {
    /// Forks the `Reader`, returning a new `Reader` that will start reading at the given starting
    /// `position`. The original `Reader` will be unaffected by reads on the new `Reader`.
    fn fork(&self, position: u64) -> Self {
        let mut cursor = self.cursor.clone();
        cursor.set_position(position);

        Self {
            cursor,
            class_names: self.class_names,
            local_fixups: self.local_fixups,
            global_fixups: self.global_fixups,
            virtual_fixups: self.virtual_fixups,
            endian: self.endian,
            ptr_size: self.ptr_size,
        }
    }

    fn pad(&mut self, align: u64) {
        self.cursor.pad(align);
    }

    fn get_local(&self, key: u32) -> Result<u32, Error> {
        self.local_fixups.get(&key).copied().ok_or_else(|| Error {
            kind: ErrorKind::InvalidLocalKey(key),
            position: self.cursor.position(),
            callstack: Vec::new(),
        })
    }

    fn get_global(&self, key: u32) -> Result<u32, Error> {
        self.global_fixups.get(&key).copied().ok_or_else(|| Error {
            kind: ErrorKind::InvalidGlobalKey(key),
            position: self.cursor.position(),
            callstack: Vec::new(),
        })
    }

    fn get_virtual(&self, key: u32) -> Result<u32, Error> {
        self.virtual_fixups.get(&key).copied().ok_or_else(|| Error {
            kind: ErrorKind::InvalidVirtualKey(key),
            position: self.cursor.position(),
            callstack: Vec::new(),
        })
    }
}

trait Decode: Sized {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error>;
}

macro_rules! impl_decode_for_primitive {
    ($($t:ty),*) => {
        $(
            impl Decode for $t {
                fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
                    if reader.cursor.remaining() < size_of::<Self>() {
                        return Err(Error {
                            kind: ErrorKind::UnexpectedEof {
                                expected: size_of::<Self>(),
                                got: reader.cursor.remaining(),
                            },
                            position: reader.cursor.position(),
                            callstack: vec![stringify!($t)],
                        });
                    }

                    let mut bytes = [0; size_of::<Self>()];
                    reader.cursor.copy_to_slice(&mut bytes);

                    match reader.endian {
                        Endianess::Little => Ok(Self::from_le_bytes(bytes)),
                        Endianess::Big => Ok(Self::from_be_bytes(bytes)),
                    }
                }
            }
        )*
    };
}

impl_decode_for_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, half::f16, f32, f64);

macro_rules! impl_decode_for_bitflags {
    ($t:ty : $primitive:ty) => {
        impl crate::Decode for $t {
            fn decode(reader: &mut crate::Reader<'_>) -> Result<Self, crate::Error> {
                <$primitive as crate::Decode>::decode(reader).map(Self::from_bits_truncate)
            }
        }
    };
}

pub(crate) use impl_decode_for_bitflags;

impl Decode for bool {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        u8::decode(reader).map(|v| v != 0)
    }
}

impl Decode for () {
    fn decode(_reader: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(())
    }
}

impl<T, const N: usize> Decode for [T; N]
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        // Guard that will drop all initialized elements.
        // This must be done via a `Drop` impl, so that
        // panics in `T::decode` will still cause all initialized
        // elements to be dropped.
        struct DropGuard<T, const N: usize> {
            arr: [MaybeUninit<T>; N],
            len: usize,
        }

        impl<T, const N: usize> Drop for DropGuard<T, N> {
            fn drop(&mut self) {
                for index in 0..self.len {
                    unsafe {
                        self.arr[index].assume_init_drop();
                    }
                }
            }
        }

        let mut guard = DropGuard {
            arr: [const { MaybeUninit::uninit() }; N],
            len: 0,
        };

        for index in 0..N {
            let elem = T::decode(reader)?;
            guard.arr[index].write(elem);
            guard.len += 1;
        }

        // Safety:
        // All `N` elements have been initialized.
        // Note that we cannot move out of `DropGuard` because it has a `Drop` impl.
        // Instead we copy the array and transmute it to `[T; N]` in the process,
        // then forget the `DropGuard` to prevent it to drop the elements.
        let arr = unsafe { core::mem::transmute_copy::<[MaybeUninit<T>; N], [T; N]>(&guard.arr) };
        core::mem::forget(guard);
        Ok(arr)
    }
}

impl Decode for String {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(reader.ptr_size.into());

        let key = reader.cursor.position();
        // Ptr sized
        u64::decode(reader)?;

        let Ok(ptr) = reader.get_local(key as u32) else {
            return Ok(String::new());
        };

        let mut reader = reader.fork(ptr as u64);
        let s = ZString::decode(&mut reader)?;

        String::from_utf8(s.0).map_err(|err| Error {
            kind: ErrorKind::InvalidString(err),
            position: reader.cursor.position(),
            callstack: vec!["String"],
        })
    }
}

#[derive(Clone)]
struct ZString(Vec<u8>);

impl ZString {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Decode for ZString {
    fn decode(mut reader: &mut Reader<'_>) -> Result<Self, Error> {
        let mut bytes = Vec::new();
        loop {
            let b = u8::decode(&mut reader)?;
            if b == 0 {
                return Ok(Self(bytes));
            }

            bytes.push(b);
        }
    }
}

impl Debug for ZString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&String::from_utf8_lossy(&self.0), f)
    }
}

impl Decode for Vec3 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        <[f32; 3]>::decode(reader).map(Self::from_array)
    }
}

impl Decode for Vec4 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(16);
        <[f32; 4]>::decode(reader).map(Self::from_array)
    }
}

impl Decode for Quat {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(16);
        <[f32; 4]>::decode(reader).map(Self::from_array)
    }
}

impl Decode for Mat3 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x_axis = Vec4::decode(reader)?;
        let y_axis = Vec4::decode(reader)?;
        let z_axis = Vec4::decode(reader)?;

        Ok(Self {
            x_axis: x_axis.truncate(),
            y_axis: y_axis.truncate(),
            z_axis: z_axis.truncate(),
        })
    }
}

impl Decode for Mat4 {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let x_axis = Vec4::decode(reader)?;
        let y_axis = Vec4::decode(reader)?;
        let z_axis = Vec4::decode(reader)?;
        let w_axis = Vec4::decode(reader)?;

        Ok(Self {
            x_axis,
            y_axis,
            z_axis,
            w_axis,
        })
    }
}

const MAGIC: [u8; 8] = [0x57, 0xe0, 0xe0, 0x57, 0x10, 0xc0, 0xc0, 0x10];

#[derive(Clone, Debug)]
pub struct HkxFile {
    objects: IndexMap<u32, HkObject>,
}

impl HkxFile {
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader {
            cursor: Cursor::new(buf),
            endian: Endianess::Little,
            class_names: &HashMap::new(),
            local_fixups: &HashMap::new(),
            global_fixups: &HashMap::new(),
            virtual_fixups: &IndexMap::new(),
            ptr_size: 0,
        };

        let magic = <[u8; 8]>::decode(&mut reader)?;
        if magic != MAGIC {
            return Err(Error {
                kind: ErrorKind::BadMagic(u64::from_le_bytes(magic)),
                position: reader.cursor.position(),
                callstack: Vec::new(),
            });
        }

        // User tag
        u32::decode(&mut reader)?;
        let version = u32::decode(&mut reader)?;

        let ptr_size = u8::decode(&mut reader)?;
        let is_little_endian = bool::decode(&mut reader)?;
        let padding_option = bool::decode(&mut reader)?;
        let base_class = u8::decode(&mut reader)?;

        assert!(!padding_option);

        reader.ptr_size = ptr_size;
        reader.endian = if is_little_endian {
            Endianess::Little
        } else {
            Endianess::Big
        };

        let section_count = u32::decode(&mut reader)?;
        let contents_section_index = u32::decode(&mut reader)?;
        let contents_section_offset = u32::decode(&mut reader)?;
        let contents_class_name_section_index = u32::decode(&mut reader)?;
        let contents_class_name_section_offset = u32::decode(&mut reader)?;

        // Version string "hk_2014..."
        <[u8; 16]>::decode(&mut reader)?;
        let flags = u32::decode(&mut reader)?;
        let max_predicate = u16::decode(&mut reader)?;
        let section_offset = u16::decode(&mut reader)?;

        reader.cursor.advance(section_offset.into());

        let section_size = match version {
            11 => 64,
            _ => 48,
        };

        let mut class_names = HashMap::new();
        let mut local_fixups = HashMap::new();
        let mut global_fixups = HashMap::new();
        let mut virtual_fixups = IndexMap::new();

        let mut objects = IndexMap::new();

        // let mut secions = Vec::new();
        for _ in 0..section_count {
            // last byte always 0xFF?
            let section_tag = <[u8; 20]>::decode(&mut reader)?;
            let abs_data_start = u32::decode(&mut reader)?;
            let local_fixup_offset = u32::decode(&mut reader)?;
            let global_fixup_offset = u32::decode(&mut reader)?;
            let virtual_fixup_offset = u32::decode(&mut reader)?;
            let exports_offset = u32::decode(&mut reader)?;
            let imports_offset = u32::decode(&mut reader)?;
            let end_offset = u32::decode(&mut reader)?;

            let num_local_fixups = (global_fixup_offset - local_fixup_offset) / 8;
            let num_global_fixups = (virtual_fixup_offset - global_fixup_offset) / 12;
            let num_virtual_fixups = (exports_offset - virtual_fixup_offset) / 12;

            {
                let mut reader =
                    reader.fork(u64::from(abs_data_start) + u64::from(local_fixup_offset));
                for _ in 0..num_local_fixups {
                    let src = u32::decode(&mut reader)?;
                    let dst = u32::decode(&mut reader)?;

                    if src != u32::MAX {
                        local_fixups.insert(src, dst);
                    }
                }
            }

            {
                let mut reader =
                    reader.fork(u64::from(abs_data_start) + u64::from(global_fixup_offset));
                for _ in 0..num_global_fixups {
                    let src = u32::decode(&mut reader)?;
                    let dst_section_index = u32::decode(&mut reader)?;
                    let dst = u32::decode(&mut reader)?;

                    if src != u32::MAX {
                        global_fixups.insert(src, dst);
                    }
                }
            }

            {
                let mut reader =
                    reader.fork(u64::from(abs_data_start) + u64::from(virtual_fixup_offset));
                for _ in 0..num_virtual_fixups {
                    let src = u32::decode(&mut reader)?;
                    let dst_section_index = u32::decode(&mut reader)?;
                    let dst = u32::decode(&mut reader)?;

                    if src != u32::MAX {
                        virtual_fixups.insert(src, dst);
                    }
                }
            }

            match CStr::from_bytes_until_nul(&section_tag)
                .map(|s| s.to_str().ok())
                .ok()
                .flatten()
            {
                Some("__classnames__") => {
                    let mut reader = reader.fork(abs_data_start.into());

                    while reader.cursor.position() - u64::from(abs_data_start)
                        < u64::from(end_offset)
                    {
                        let signature = ClassSignature::decode(&mut reader);
                        if u8::decode(&mut reader)? != 0x09 {
                            break;
                        }

                        let src = reader.cursor.position() - u64::from(abs_data_start);
                        let name = ZString::decode(&mut reader)?;

                        let signature = signature.map_err(|mut err| {
                            if let ErrorKind::UnknownClassName(_, class_name) = &mut err.kind {
                                *class_name = String::from_utf8_lossy(&name.0).to_string();
                            }
                            err
                        });

                        class_names.insert(
                            src as u32,
                            ClassName {
                                signature: signature?,
                                name,
                            },
                        );
                    }
                }
                Some("__data__") => {
                    // Create a new cursor over only the current section.
                    // This is required for the keys to be valid.
                    let Some(data) = buf.get(
                        abs_data_start as usize..abs_data_start as usize + end_offset as usize,
                    ) else {
                        return Err(Error {
                            kind: ErrorKind::UnexpectedEof {
                                expected: end_offset as usize,
                                got: buf
                                    .get(abs_data_start as usize..)
                                    .map(|v| v.len())
                                    .unwrap_or_default(),
                            },
                            position: abs_data_start.into(),
                            callstack: Vec::new(),
                        });
                    };

                    // Recreate the reader with the new fixup tables.
                    // Note that we cannot just assign to the current reader
                    // since the tables may be mutated in the next section.
                    let reader = Reader {
                        cursor: Cursor::new(data),
                        endian: reader.endian,
                        ptr_size: reader.ptr_size,
                        class_names: &class_names,
                        local_fixups: &local_fixups,
                        global_fixups: &global_fixups,
                        virtual_fixups: &virtual_fixups,
                    };

                    for src in virtual_fixups.keys() {
                        let mut reader = reader.fork((*src).into());

                        let object = HkObject::decode(&mut reader)?;
                        objects.insert(*src, object);
                    }
                }
                _ => (),
            }

            let padding_bytes = section_size - 48;
            reader.cursor.advance(padding_bytes);
        }

        Ok(Self { objects })
    }

    pub fn iter(&self) -> impl Iterator<Item = &HkObject> {
        self.objects.values()
    }

    pub fn get(&self, ptr: OpaquePtr) -> Option<&HkObject> {
        self.objects.get(&ptr.0)
    }
}

/// An opaque pointer to another [`HkObject`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpaquePtr(u32);

impl Decode for OpaquePtr {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(reader.ptr_size.into());

        let key = reader.cursor.position();
        // Ptr sized
        if u64::decode(reader)? != 0 {
            return Err(Error {
                kind: ErrorKind::PtrNotZero,
                position: reader.cursor.position() - u64::from(reader.ptr_size),
                callstack: vec![stringify!(OpaquePtr)],
            });
        }

        let ptr = reader.get_global(key as u32)?;
        Ok(Self(ptr))
    }
}

impl Decode for Option<OpaquePtr> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(reader.ptr_size.into());

        let key = reader.cursor.position();
        // Ptr sized
        if u64::decode(reader)? != 0 {
            return Err(Error {
                kind: ErrorKind::PtrNotZero,
                position: reader.cursor.position() - u64::from(reader.ptr_size),
                callstack: vec![stringify!(Option<OpaquePtr>)],
            });
        }

        let Ok(ptr) = reader.get_global(key as u32) else {
            return Ok(None);
        };
        Ok(Some(OpaquePtr(ptr)))
    }
}

#[derive(Clone, Debug)]
struct ClassName {
    signature: ClassSignature,
    name: ZString,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ClassSignature {
    Class,
    ClassMember,
    ClassEnum,
    ClassEnumItem,
    RootLevelContainer,
    AnimationContainer,
    SplineCompressedAnimation,
    DefaultAnimatedReferenceFrame,
    AnimationBinding,
    MemoryResourceContainer,
    ProjectStringData,
    ProjectData,
    CharacterData,
    VariableValueSet,
    MirroredSkeletionInfo,
    CharacterStringData,
    FootIkDriverInfo,
    BoneWeightArray,
    RefCountedProperties,
    PhysicsSystemData,
    CompressedMeshShape,
    CompressedMeshShapeData,
    ScaledConvexShape,
    ConvexPolytopeShape,
    ShapeMassProperties,
    DynamicCompoundShape,
    DynamicCompoundShapeData,
    Skeleton,
    PhysicsSceneData,
    RagdollData,
    CapsuleShape,
    RagdollConstraintData,
    PositionConstraintMotor,
    LimitedHingeConstraintData,
    SkeletonMapper,
    BsMaterialProperties,
    Node,
    Generator,
    BehaviorGraph,
    StateMachine,
    StateMachineStateInfo,
    ModifierGenerator,
    ModifierList,
    EventDrivenModifier,
    BSGetTimeStepModifier,
    VariableBindingSet,
    EvaluateExpressionModifier,
    ExpressionDataArray,
    TwistModifier,
    BSInterpValueModifier,
    EventsFromRangeModifier,
    EventRangeDataArray,
    FootIkControlsModifier,
    BSLookAtModifier,
    BSOffsetAnimationGenerator,
    BlenderGenerator,
    BlenderGeneratorChild,
    BSBehaviorGraphSwapGenerator,
    ClipGenerator,
    StateMachineEventPropertyArray,
    KeyframesBonesModifier,
    BoneIndexArray,
    BSRagdollContactListenerModifier,
    BSIsActiveModifier,
    ClipTriggerArray,
    ManualSelectorGenerator,
    StateMachineTransitionInfoArray,
    BlendingTransitionEffect,
    StringEventPayload,
    GetUpModifier,
    RigidBodyRagdollControlsModifier,
    BSModifyOnceModifier,
    PoseMatchingGenerator,
    PoweredRagdollControlsModifier,
    TimerModifier,
    BehaviorGraphData,
    BehaviorGraphStringData,
    BSAssignVariablesModifier,
    BSEventOnDeactivateModifier,
    LayerGenerator,
    Layer,
    BehaviorReferenceGenerator,
    BSEventEveryNEventsModifier,
    BSCyclicBlendTransitionGenerator,
    BSiStateTaggingGenerator,
    HandIkControlsModifier,
    GetHandleOnBoneModifier,
    BSTimerModifier,
    BSDirectAtModifier,
    ExpressionCondition,
    GeneratorTransitionEffect,
    ReferencePoseGenerator,
    BSDirectAtCapturePoseModifier,
    MirrorModifier,
    BSEventOnFalseToTrueModifier,
    BreakableConstraintData,
    BSRootTwistModifier,
    BSRandomAlarmModifier,
    HandIkDriverInfo,
}

impl Decode for ClassSignature {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match u32::decode(reader)? {
            0x33d42383 => Ok(Self::Class),
            0xb0efa719 => Ok(Self::ClassMember),
            0x8a3609cf => Ok(Self::ClassEnum),
            0xce6f8a6c => Ok(Self::ClassEnumItem),
            0x2772c11e => Ok(Self::RootLevelContainer),
            0x26859f4c => Ok(Self::AnimationContainer),
            0x8c3b5f7e => Ok(Self::SplineCompressedAnimation),
            0x60f8e0b8 => Ok(Self::DefaultAnimatedReferenceFrame),
            0x0faf9150 => Ok(Self::AnimationBinding),
            0x1de13a73 => Ok(Self::MemoryResourceContainer),
            0xca08c2ba => Ok(Self::ProjectStringData),
            0x363c1159 => Ok(Self::ProjectData),
            0xfec46c1f => Ok(Self::CharacterData),
            0xeb5f7e25 => Ok(Self::VariableValueSet),
            0x9f13052e => Ok(Self::MirroredSkeletionInfo),
            0x0b9d8a52 => Ok(Self::CharacterStringData),
            0xf79c429d => Ok(Self::FootIkDriverInfo),
            0x2c9b5751 => Ok(Self::BoneWeightArray),
            0x7c574867 => Ok(Self::RefCountedProperties),
            0xb857718b => Ok(Self::PhysicsSystemData),
            0x5f60d536 => Ok(Self::CompressedMeshShape),
            0xa2bdfc59 => Ok(Self::CompressedMeshShapeData),
            0x0f14e5f2 => Ok(Self::ScaledConvexShape),
            0x3ce9b3e3 => Ok(Self::ConvexPolytopeShape),
            0xe9191728 => Ok(Self::ShapeMassProperties),
            0x4620d11c => Ok(Self::DynamicCompoundShape),
            0xf33dc3cc => Ok(Self::DynamicCompoundShapeData),
            0xfec1cedb => Ok(Self::Skeleton),
            0x701ce72c => Ok(Self::PhysicsSceneData),
            0xdc8f20ab => Ok(Self::RagdollData),
            0x60a75f4c => Ok(Self::CapsuleShape),
            0xb77d2036 => Ok(Self::RagdollConstraintData),
            0x143dd400 => Ok(Self::PositionConstraintMotor),
            0x51ea603a => Ok(Self::LimitedHingeConstraintData),
            0xace9849c => Ok(Self::SkeletonMapper),
            0xa3e47a9a => Ok(Self::BsMaterialProperties),
            Node::ID => Ok(Self::Node),
            Generator::ID => Ok(Self::Generator),
            BehaviorGraph::ID => Ok(Self::BehaviorGraph),
            StateMachine::ID => Ok(Self::StateMachine),
            StateMachineStateInfo::ID => Ok(Self::StateMachineStateInfo),
            ModifierGenerator::ID => Ok(Self::ModifierGenerator),
            ModifierList::ID => Ok(Self::ModifierList),
            EventDrivenModifier::ID => Ok(Self::EventDrivenModifier),
            BSGetTimeStepModifier::ID => Ok(Self::BSGetTimeStepModifier),
            VariableBindingSet::ID => Ok(Self::VariableBindingSet),
            EvaluateExpressionModifier::ID => Ok(Self::EvaluateExpressionModifier),
            ExpressionDataArray::ID => Ok(Self::ExpressionDataArray),
            TwistModifier::ID => Ok(Self::TwistModifier),
            BSInterpValueModifier::ID => Ok(Self::BSInterpValueModifier),
            EventsFromRangeModifier::ID => Ok(Self::EventsFromRangeModifier),
            EventRangeDataArray::ID => Ok(Self::EventRangeDataArray),
            FootIkControlsModifier::ID => Ok(Self::FootIkControlsModifier),
            BSLookAtModifier::ID => Ok(Self::BSLookAtModifier),
            BSOffsetAnimationGenerator::ID => Ok(Self::BSOffsetAnimationGenerator),
            BlenderGenerator::ID => Ok(Self::BlenderGenerator),
            BlenderGeneratorChild::ID => Ok(Self::BlenderGeneratorChild),
            BSBehaviorGraphSwapGenerator::ID => Ok(Self::BSBehaviorGraphSwapGenerator),
            ClipGenerator::ID => Ok(Self::ClipGenerator),
            StateMachineEventPropertyArray::ID => Ok(Self::StateMachineEventPropertyArray),
            KeyframesBonesModifier::ID => Ok(Self::KeyframesBonesModifier),
            BoneIndexArray::ID => Ok(Self::BoneIndexArray),
            BSRagdollContactListenerModifier::ID => Ok(Self::BSRagdollContactListenerModifier),
            BSIsActiveModifier::ID => Ok(Self::BSIsActiveModifier),
            ClipTriggerArray::ID => Ok(Self::ClipTriggerArray),
            ManualSelectorGenerator::ID => Ok(Self::ManualSelectorGenerator),
            StateMachineTransitionInfoArray::ID => Ok(Self::StateMachineTransitionInfoArray),
            BlendingTransitionEffect::ID => Ok(Self::BlendingTransitionEffect),
            StringEventPayload::ID => Ok(Self::StringEventPayload),
            GetUpModifier::ID => Ok(Self::GetUpModifier),
            RigidBodyRagdollControlsModifier::ID => Ok(Self::RigidBodyRagdollControlsModifier),
            BSModifyOnceModifier::ID => Ok(Self::BSModifyOnceModifier),
            PoseMatchingGenerator::ID => Ok(Self::PoseMatchingGenerator),
            PoweredRagdollControlsModifier::ID => Ok(Self::PoweredRagdollControlsModifier),
            TimerModifier::ID => Ok(Self::TimerModifier),
            BehaviorGraphData::ID => Ok(Self::BehaviorGraphData),
            BehaviorGraphStringData::ID => Ok(Self::BehaviorGraphStringData),
            BSAssignVariablesModifier::ID => Ok(Self::BSAssignVariablesModifier),
            BSEventOnDeactivateModifier::ID => Ok(Self::BSEventOnDeactivateModifier),
            LayerGenerator::ID => Ok(Self::LayerGenerator),
            Layer::ID => Ok(Self::Layer),
            BehaviorReferenceGenerator::ID => Ok(Self::BehaviorReferenceGenerator),
            BSEventEveryNEventsModifier::ID => Ok(Self::BSEventEveryNEventsModifier),
            BSCyclicBlendTransitionGenerator::ID => Ok(Self::BSCyclicBlendTransitionGenerator),
            BSiStateTaggingGenerator::ID => Ok(Self::BSiStateTaggingGenerator),
            HandIkControlsModifier::ID => Ok(Self::HandIkControlsModifier),
            GetHandleOnBoneModifier::ID => Ok(Self::GetHandleOnBoneModifier),
            BSTimerModifier::ID => Ok(Self::BSTimerModifier),
            BSDirectAtModifier::ID => Ok(Self::BSDirectAtModifier),
            ExpressionCondition::ID => Ok(Self::ExpressionCondition),
            GeneratorTransitionEffect::ID => Ok(Self::GeneratorTransitionEffect),
            ReferencePoseGenerator::ID => Ok(Self::ReferencePoseGenerator),
            BSDirectAtCapturePoseModifier::ID => Ok(Self::BSDirectAtCapturePoseModifier),
            MirrorModifier::ID => Ok(Self::MirrorModifier),
            BSEventOnFalseToTrueModifier::ID => Ok(Self::BSEventOnFalseToTrueModifier),
            BreakableConstraintData::ID => Ok(Self::BreakableConstraintData),
            BSRootTwistModifier::ID => Ok(Self::BSRootTwistModifier),
            BSRandomAlarmModifier::ID => Ok(Self::BSRandomAlarmModifier),
            HandIkDriverInfo::ID => Ok(Self::HandIkDriverInfo),
            val => Err(Error {
                kind: ErrorKind::UnknownClassName(val, "".to_owned()),
                position: reader.cursor.position(),
                callstack: Vec::new(),
            }),
        }
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct ReferencedObject {
    // Offset: 0
    // Ptr sized
    id: u64,
    // Offset: 8
    mem_size_and_flags: u16,
    // Offset: 10
    ref_count: u16,
    // Offset: 12
    _pad0: u32,
}

#[derive(Clone, Debug)]
pub enum HkObject {
    RootLevelContainer(RootLevelContainer),
    AnimationContainer(AnimationContainer),
    SplineCompressedAnimation(SplineCompressedAnimation),
    DefaultAnimatedReferenceFrame(DefaultAnimatedReferenceFrame),
    AnimationBinding(AnimationBinding),
    ProjectStringData(ProjectStringData),
    ProjectData(ProjectData),
    // CharacterData(CharacterData),
    MirroredSkeletionInfo(MirroredSkeletionInfo),
    // CharacterStringData(CharacterStringData),
    FootIkDriverInfo(FootIkDriverInfo),
    BoneWeightArray(BoneWeightArray),
    RefCountedProperties(RefCountedProperties),
    PhysicsSystemData(PhysicsSystemData),
    CompressedMeshShape(CompressedMeshShape),
    CompressedMeshShapeData(CompressedMeshShapeData),
    ConvexPolytopeShape(ConvexPolytopeShape),
    ScaledConvexShape(ScaledConvexShape),
    Skeleton(Skeleton),
    PhysicsSceneData(PhysicsSceneData),
    RagdollData(RagdollData),
    CapsuleShape(CapsuleShape),
    RagdollConstraintData(RagdollConstraintData),
    PositionConstraintMotor(PositionConstraintMotor),
    LimitedHingeConstraintData(LimitedHingeConstraintData),
    SkeletonMapper(SkeletonMapper),
    Node(Node),
    Generator(Generator),
    BehaviorGraph(BehaviorGraph),
    StateMachine(StateMachine),
    StateMachineStateInfo(StateMachineStateInfo),
    ModifierGenerator(ModifierGenerator),
    ModifierList(ModifierList),
    EventDrivenModifier(EventDrivenModifier),
    BSGetTimeStepModifier(BSGetTimeStepModifier),
    VariableBindingSet(VariableBindingSet),
    EvaluateExpressionModifier(EvaluateExpressionModifier),
    ExpressionDataArray(ExpressionDataArray),
    TwistModifier(TwistModifier),
    BSInterpValueModifier(BSInterpValueModifier),
    EventsFromRangeModifier(EventsFromRangeModifier),
    EventRangeDataArray(EventRangeDataArray),
    FootIkControlsModifier(FootIkControlsModifier),
    BSLookAtModifier(BSLookAtModifier),
    BSOffsetAnimationGenerator(BSOffsetAnimationGenerator),
    BlenderGenerator(BlenderGenerator),
    BlenderGeneratorChild(BlenderGeneratorChild),
    BSBehaviorGraphSwapGenerator(BSBehaviorGraphSwapGenerator),
    ClipGenerator(ClipGenerator),
    StateMachineEventPropertyArray(StateMachineEventPropertyArray),
    KeyframesBonesModifier(KeyframesBonesModifier),
    BoneIndexArray(BoneIndexArray),
    BSRagdollContactListenerModifier(BSRagdollContactListenerModifier),
    BSIsActiveModifier(BSIsActiveModifier),
    ClipTriggerArray(ClipTriggerArray),
    ManualSelectorGenerator(ManualSelectorGenerator),
    StateMachineTransitionInfoArray(StateMachineTransitionInfoArray),
    BlendingTransitionEffect(BlendingTransitionEffect),
    StringEventPayload(StringEventPayload),
    GetUpModifier(GetUpModifier),
    RigidBodyRagdollControlsModifier(RigidBodyRagdollControlsModifier),
    BSModifyOnceModifier(BSModifyOnceModifier),
    PoseMatchingGenerator(PoseMatchingGenerator),
    PoweredRagdollControlsModifier(PoweredRagdollControlsModifier),
    TimerModifier(TimerModifier),
    BehaviorGraphData(BehaviorGraphData),
    BehaviorGraphStringData(BehaviorGraphStringData),
    BSAssignVariablesModifier(BSAssignVariablesModifier),
    BSEventOnDeactivateModifier(BSEventOnDeactivateModifier),
    LayerGenerator(LayerGenerator),
    Layer(Layer),
    BehaviorReferenceGenerator(BehaviorReferenceGenerator),
    BSEventEveryNEventsModifier(BSEventEveryNEventsModifier),
    BSCyclicBlendTransitionGenerator(BSCyclicBlendTransitionGenerator),
    BSiStateTaggingGenerator(BSiStateTaggingGenerator),
    HandIkControlsModifier(HandIkControlsModifier),
    GetHandleOnBoneModifier(GetHandleOnBoneModifier),
    BSTimerModifier(BSTimerModifier),
    BSDirectAtModifier(BSDirectAtModifier),
    ExpressionCondition(ExpressionCondition),
    GeneratorTransitionEffect(GeneratorTransitionEffect),
    ReferencePoseGenerator(ReferencePoseGenerator),
    BSDirectAtCapturePoseModifier(BSDirectAtCapturePoseModifier),
    MirrorModifier(MirrorModifier),
    BSEventOnFalseToTrueModifier(BSEventOnFalseToTrueModifier),
    CharacterData(CharacterData),
    CharacterStringData(CharacterStringData),
    BreakableConstraintData(BreakableConstraintData),
    BSRootTwistModifier(BSRootTwistModifier),
    BSRandomAlarmModifier(BSRandomAlarmModifier),
    HandIkDriverInfo(HandIkDriverInfo),
    DynamicCompoundShape(DynamicCompoundShape),
    Ignored,
}

impl Decode for HkObject {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let dst = reader.get_virtual(reader.cursor.position() as u32)?;
        let class = reader.class_names.get(&dst).ok_or_else(|| Error {
            kind: ErrorKind::InvalidClassName(dst),
            position: reader.cursor.position(),
            callstack: Vec::new(),
        })?;

        match class.signature {
            ClassSignature::Class => Ok(Self::Ignored),
            ClassSignature::ClassMember => Ok(Self::Ignored),
            ClassSignature::ClassEnum => Ok(Self::Ignored),
            ClassSignature::ClassEnumItem => Ok(Self::Ignored),
            ClassSignature::RootLevelContainer => {
                RootLevelContainer::decode(reader).map(Self::RootLevelContainer)
            }
            ClassSignature::AnimationContainer => {
                AnimationContainer::decode(reader).map(Self::AnimationContainer)
            }
            ClassSignature::SplineCompressedAnimation => {
                SplineCompressedAnimation::decode(reader).map(Self::SplineCompressedAnimation)
            }
            ClassSignature::DefaultAnimatedReferenceFrame => {
                DefaultAnimatedReferenceFrame::decode(reader)
                    .map(Self::DefaultAnimatedReferenceFrame)
            }
            ClassSignature::AnimationBinding => {
                AnimationBinding::decode(reader).map(Self::AnimationBinding)
            }
            ClassSignature::MemoryResourceContainer => Ok(Self::Ignored),
            ClassSignature::ProjectStringData => {
                ProjectStringData::decode(reader).map(Self::ProjectStringData)
            }
            ClassSignature::ProjectData => ProjectData::decode(reader).map(Self::ProjectData),
            ClassSignature::CharacterData => CharacterData::decode(reader).map(Self::CharacterData),
            ClassSignature::CharacterStringData => {
                CharacterStringData::decode(reader).map(Self::CharacterStringData)
            }
            ClassSignature::VariableValueSet => Ok(Self::Ignored),
            ClassSignature::MirroredSkeletionInfo => {
                MirroredSkeletionInfo::decode(reader).map(Self::MirroredSkeletionInfo)
            }
            ClassSignature::FootIkDriverInfo => {
                FootIkDriverInfo::decode(reader).map(Self::FootIkDriverInfo)
            }
            ClassSignature::BoneWeightArray => {
                BoneWeightArray::decode(reader).map(Self::BoneWeightArray)
            }
            ClassSignature::RefCountedProperties => Ok(Self::Ignored),
            ClassSignature::PhysicsSystemData => {
                PhysicsSystemData::decode(reader).map(Self::PhysicsSystemData)
            }
            ClassSignature::BsMaterialProperties => Ok(Self::Ignored),
            ClassSignature::CompressedMeshShape => {
                CompressedMeshShape::decode(reader).map(Self::CompressedMeshShape)
            }
            ClassSignature::CompressedMeshShapeData => {
                CompressedMeshShapeData::decode(reader).map(Self::CompressedMeshShapeData)
            }
            ClassSignature::ConvexPolytopeShape => {
                ConvexPolytopeShape::decode(reader).map(Self::ConvexPolytopeShape)
            }
            ClassSignature::ShapeMassProperties => Ok(Self::Ignored),
            ClassSignature::DynamicCompoundShape => {
                DynamicCompoundShape::decode(reader).map(Self::DynamicCompoundShape)
            }
            ClassSignature::DynamicCompoundShapeData => Ok(Self::Ignored),
            ClassSignature::Skeleton => Skeleton::decode(reader).map(Self::Skeleton),
            ClassSignature::PhysicsSceneData => {
                PhysicsSceneData::decode(reader).map(Self::PhysicsSceneData)
            }
            ClassSignature::RagdollData => RagdollData::decode(reader).map(Self::RagdollData),
            ClassSignature::CapsuleShape => CapsuleShape::decode(reader).map(Self::CapsuleShape),
            ClassSignature::RagdollConstraintData => {
                RagdollConstraintData::decode(reader).map(Self::RagdollConstraintData)
            }
            ClassSignature::PositionConstraintMotor => {
                PositionConstraintMotor::decode(reader).map(Self::PositionConstraintMotor)
            }
            ClassSignature::LimitedHingeConstraintData => {
                LimitedHingeConstraintData::decode(reader).map(Self::LimitedHingeConstraintData)
            }
            ClassSignature::SkeletonMapper => {
                SkeletonMapper::decode(reader).map(Self::SkeletonMapper)
            }
            ClassSignature::ScaledConvexShape => {
                ScaledConvexShape::decode(reader).map(Self::ScaledConvexShape)
            }
            ClassSignature::Node => Node::decode(reader).map(Self::Node),
            ClassSignature::Generator => Generator::decode(reader).map(Self::Generator),
            ClassSignature::BehaviorGraph => BehaviorGraph::decode(reader).map(Self::BehaviorGraph),
            ClassSignature::StateMachine => StateMachine::decode(reader).map(Self::StateMachine),
            ClassSignature::StateMachineStateInfo => {
                StateMachineStateInfo::decode(reader).map(Self::StateMachineStateInfo)
            }
            ClassSignature::ModifierGenerator => {
                ModifierGenerator::decode(reader).map(Self::ModifierGenerator)
            }
            ClassSignature::ModifierList => ModifierList::decode(reader).map(Self::ModifierList),
            ClassSignature::EventDrivenModifier => {
                EventDrivenModifier::decode(reader).map(Self::EventDrivenModifier)
            }
            ClassSignature::BSGetTimeStepModifier => {
                BSGetTimeStepModifier::decode(reader).map(Self::BSGetTimeStepModifier)
            }
            ClassSignature::VariableBindingSet => {
                VariableBindingSet::decode(reader).map(Self::VariableBindingSet)
            }
            ClassSignature::EvaluateExpressionModifier => {
                EvaluateExpressionModifier::decode(reader).map(Self::EvaluateExpressionModifier)
            }
            ClassSignature::ExpressionDataArray => {
                ExpressionDataArray::decode(reader).map(Self::ExpressionDataArray)
            }
            ClassSignature::TwistModifier => TwistModifier::decode(reader).map(Self::TwistModifier),
            ClassSignature::BSInterpValueModifier => {
                BSInterpValueModifier::decode(reader).map(Self::BSInterpValueModifier)
            }
            ClassSignature::EventsFromRangeModifier => {
                EventsFromRangeModifier::decode(reader).map(Self::EventsFromRangeModifier)
            }
            ClassSignature::EventRangeDataArray => {
                EventRangeDataArray::decode(reader).map(Self::EventRangeDataArray)
            }
            ClassSignature::FootIkControlsModifier => {
                FootIkControlsModifier::decode(reader).map(Self::FootIkControlsModifier)
            }
            ClassSignature::BSLookAtModifier => {
                BSLookAtModifier::decode(reader).map(Self::BSLookAtModifier)
            }
            ClassSignature::BSOffsetAnimationGenerator => {
                BSOffsetAnimationGenerator::decode(reader).map(Self::BSOffsetAnimationGenerator)
            }
            ClassSignature::BlenderGenerator => {
                BlenderGenerator::decode(reader).map(Self::BlenderGenerator)
            }
            ClassSignature::BlenderGeneratorChild => {
                BlenderGeneratorChild::decode(reader).map(Self::BlenderGeneratorChild)
            }
            ClassSignature::BSBehaviorGraphSwapGenerator => {
                BSBehaviorGraphSwapGenerator::decode(reader).map(Self::BSBehaviorGraphSwapGenerator)
            }
            ClassSignature::ClipGenerator => ClipGenerator::decode(reader).map(Self::ClipGenerator),
            ClassSignature::StateMachineEventPropertyArray => {
                StateMachineEventPropertyArray::decode(reader)
                    .map(Self::StateMachineEventPropertyArray)
            }
            ClassSignature::KeyframesBonesModifier => {
                KeyframesBonesModifier::decode(reader).map(Self::KeyframesBonesModifier)
            }
            ClassSignature::BoneIndexArray => {
                BoneIndexArray::decode(reader).map(Self::BoneIndexArray)
            }
            ClassSignature::BSRagdollContactListenerModifier => {
                BSRagdollContactListenerModifier::decode(reader)
                    .map(Self::BSRagdollContactListenerModifier)
            }
            ClassSignature::BSIsActiveModifier => {
                BSIsActiveModifier::decode(reader).map(Self::BSIsActiveModifier)
            }
            ClassSignature::ClipTriggerArray => {
                ClipTriggerArray::decode(reader).map(Self::ClipTriggerArray)
            }
            ClassSignature::ManualSelectorGenerator => {
                ManualSelectorGenerator::decode(reader).map(Self::ManualSelectorGenerator)
            }
            ClassSignature::StateMachineTransitionInfoArray => {
                StateMachineTransitionInfoArray::decode(reader)
                    .map(Self::StateMachineTransitionInfoArray)
            }
            ClassSignature::BlendingTransitionEffect => {
                BlendingTransitionEffect::decode(reader).map(Self::BlendingTransitionEffect)
            }
            ClassSignature::StringEventPayload => {
                StringEventPayload::decode(reader).map(Self::StringEventPayload)
            }
            ClassSignature::GetUpModifier => GetUpModifier::decode(reader).map(Self::GetUpModifier),
            ClassSignature::RigidBodyRagdollControlsModifier => {
                RigidBodyRagdollControlsModifier::decode(reader)
                    .map(Self::RigidBodyRagdollControlsModifier)
            }
            ClassSignature::BSModifyOnceModifier => {
                BSModifyOnceModifier::decode(reader).map(Self::BSModifyOnceModifier)
            }
            ClassSignature::PoseMatchingGenerator => {
                PoseMatchingGenerator::decode(reader).map(Self::PoseMatchingGenerator)
            }
            ClassSignature::PoweredRagdollControlsModifier => {
                PoweredRagdollControlsModifier::decode(reader)
                    .map(Self::PoweredRagdollControlsModifier)
            }
            ClassSignature::TimerModifier => TimerModifier::decode(reader).map(Self::TimerModifier),
            ClassSignature::BehaviorGraphData => {
                BehaviorGraphData::decode(reader).map(Self::BehaviorGraphData)
            }
            ClassSignature::BehaviorGraphStringData => {
                BehaviorGraphStringData::decode(reader).map(Self::BehaviorGraphStringData)
            }
            ClassSignature::BSAssignVariablesModifier => {
                BSAssignVariablesModifier::decode(reader).map(Self::BSAssignVariablesModifier)
            }
            ClassSignature::BSEventOnDeactivateModifier => {
                BSEventOnDeactivateModifier::decode(reader).map(Self::BSEventOnDeactivateModifier)
            }
            ClassSignature::LayerGenerator => {
                LayerGenerator::decode(reader).map(Self::LayerGenerator)
            }
            ClassSignature::Layer => Layer::decode(reader).map(Self::Layer),
            ClassSignature::BehaviorReferenceGenerator => {
                BehaviorReferenceGenerator::decode(reader).map(Self::BehaviorReferenceGenerator)
            }
            ClassSignature::BSEventEveryNEventsModifier => {
                BSEventEveryNEventsModifier::decode(reader).map(Self::BSEventEveryNEventsModifier)
            }
            ClassSignature::BSCyclicBlendTransitionGenerator => {
                BSCyclicBlendTransitionGenerator::decode(reader)
                    .map(Self::BSCyclicBlendTransitionGenerator)
            }
            ClassSignature::BSiStateTaggingGenerator => {
                BSiStateTaggingGenerator::decode(reader).map(Self::BSiStateTaggingGenerator)
            }
            ClassSignature::HandIkControlsModifier => {
                HandIkControlsModifier::decode(reader).map(Self::HandIkControlsModifier)
            }
            ClassSignature::GetHandleOnBoneModifier => {
                GetHandleOnBoneModifier::decode(reader).map(Self::GetHandleOnBoneModifier)
            }
            ClassSignature::BSTimerModifier => {
                BSTimerModifier::decode(reader).map(Self::BSTimerModifier)
            }
            ClassSignature::BSDirectAtModifier => {
                BSDirectAtModifier::decode(reader).map(Self::BSDirectAtModifier)
            }
            ClassSignature::ExpressionCondition => {
                ExpressionCondition::decode(reader).map(Self::ExpressionCondition)
            }
            ClassSignature::GeneratorTransitionEffect => {
                GeneratorTransitionEffect::decode(reader).map(Self::GeneratorTransitionEffect)
            }
            ClassSignature::ReferencePoseGenerator => {
                ReferencePoseGenerator::decode(reader).map(Self::ReferencePoseGenerator)
            }
            ClassSignature::BSDirectAtCapturePoseModifier => {
                BSDirectAtCapturePoseModifier::decode(reader)
                    .map(Self::BSDirectAtCapturePoseModifier)
            }
            ClassSignature::MirrorModifier => {
                MirrorModifier::decode(reader).map(Self::MirrorModifier)
            }
            ClassSignature::BSEventOnFalseToTrueModifier => {
                BSEventOnFalseToTrueModifier::decode(reader).map(Self::BSEventOnFalseToTrueModifier)
            }
            ClassSignature::BreakableConstraintData => {
                BreakableConstraintData::decode(reader).map(Self::BreakableConstraintData)
            }
            ClassSignature::BSRootTwistModifier => {
                BSRootTwistModifier::decode(reader).map(Self::BSRootTwistModifier)
            }
            ClassSignature::BSRandomAlarmModifier => {
                BSRandomAlarmModifier::decode(reader).map(Self::BSRandomAlarmModifier)
            }
            ClassSignature::HandIkDriverInfo => {
                HandIkDriverInfo::decode(reader).map(Self::HandIkDriverInfo)
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

trait PadCursor {
    fn pad(&mut self, align: u64);
}

impl<T> PadCursor for Cursor<T> {
    fn pad(&mut self, align: u64) {
        while self.position() % align != 0 {
            // :)
            self.set_position(self.position() + 1);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Ptr<T> {
    pub ptr: OpaquePtr,
    pub value: T,
}

impl<T> Decode for Ptr<T>
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(reader.ptr_size.into());

        let key = reader.cursor.position();
        // Ptr sized
        if u64::decode(reader)? != 0 {
            return Err(Error {
                kind: ErrorKind::PtrNotZero,
                position: reader.cursor.position() - u64::from(reader.ptr_size),
                callstack: vec![stringify!(Ptr<T>)],
            });
        }

        let ptr = reader.get_global(key as u32)?;

        let mut reader = reader.fork(ptr as u64);
        let value = T::decode(&mut reader)?;
        Ok(Self {
            ptr: OpaquePtr(ptr),
            value,
        })
    }
}

impl<T> Decode for Option<Ptr<T>>
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(reader.ptr_size.into());

        let key = reader.cursor.position();
        // Ptr sized
        if u64::decode(reader)? != 0 {
            return Err(Error {
                kind: ErrorKind::PtrNotZero,
                position: reader.cursor.position() - u64::from(reader.ptr_size),
                callstack: vec![stringify!(Option<Ptr<T>>)],
            });
        }

        let Ok(ptr) = reader.get_global(key as u32) else {
            return Ok(None);
        };

        let mut reader = reader.fork(ptr as u64);
        let value = T::decode(&mut reader)?;
        Ok(Some(Ptr {
            value,
            ptr: OpaquePtr(ptr),
        }))
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Clone, Debug, Default)]
pub struct Array<T>(pub Vec<T>);

impl<T> Decode for Array<T>
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.pad(reader.ptr_size.into());

        let key = reader.cursor.position();

        // Ptr sized
        if u64::decode(reader)? != 0 {
            return Err(Error {
                kind: ErrorKind::PtrNotZero,
                position: reader.cursor.position() - u64::from(reader.ptr_size),
                callstack: vec![stringify!(Array<T>)],
            });
        }

        let len = u32::decode(reader)?;
        // ??
        u32::decode(reader)?;
        // Seems like it is possible the key may not
        // exist when the len is zero?
        if len == 0 {
            return Ok(Self(Vec::new()));
        }

        let ptr = reader.get_local(key as u32)?;

        let mut reader = reader.fork(ptr as u64);

        (0..len)
            .map(|_| T::decode(&mut reader))
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

impl<T> Deref for Array<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An array that is read from the current position with an offset.
#[derive(Clone, Debug, Default)]
pub struct RelArray<T>(pub Vec<T>);

impl<T> Decode for RelArray<T>
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let start = reader.cursor.position();
        let len = u16::decode(reader)?;
        let offset = u16::decode(reader)?;

        let mut reader = reader.fork(start + u64::from(offset));

        (0..len)
            .map(|_| T::decode(&mut reader))
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

impl<T> Deref for RelArray<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, HkStruct)]
pub struct RootLevelContainer {
    // Offset: 0
    pub named_variants: Array<RootLevelContainerNamedVariant>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct RootLevelContainerNamedVariant {
    // Offset: 0
    pub name: String,
    // Offset: 8
    pub class_name: String,
    // Offset: 16
    pub variant: Ptr<ReferencedObject>,
}

pub trait Signature {
    const ID: u32;
}
