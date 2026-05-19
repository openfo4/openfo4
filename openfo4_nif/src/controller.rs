use glam::{Quat, Vec3};

use crate::{BlockRef, Decode, Error, Reader, nif_struct};

#[derive(Clone, Debug)]
pub struct NiControllerManager {
    pub next_controller: Option<BlockRef>,
    pub flags: TimeControllerFlags,
    pub frequency: f32,
    pub phase: f32,
    pub start_time: f32,
    pub stop_time: f32,
    pub target: Option<BlockRef>,
    pub cumulative: bool,
    pub sequences: Vec<BlockRef>,
    pub object_palette: Option<BlockRef>,
}

impl Decode for NiControllerManager {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let next_controller = <Option<BlockRef>>::decode(reader)?;
        let flags = TimeControllerFlags::decode(reader)?;
        let frequency = f32::decode(reader)?;
        let phase = f32::decode(reader)?;
        let start_time = f32::decode(reader)?;
        let stop_time = f32::decode(reader)?;
        let target = <Option<BlockRef>>::decode(reader)?;
        let cumulative = reader.get_bool()?;

        let sequence_count = u32::decode(reader)?;
        let sequences = (0..sequence_count)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        let object_palette = <Option<BlockRef>>::decode(reader)?;

        Ok(Self {
            next_controller,
            flags,
            frequency,
            phase,
            start_time,
            stop_time,
            target,
            cumulative,
            sequences,
            object_palette,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NiMultiTargetTransformController {
    pub next_controller: Option<BlockRef>,
    pub flags: TimeControllerFlags,
    pub frequency: f32,
    pub phase: f32,
    pub start_time: f32,
    pub stop_time: f32,
    pub targets: Vec<BlockRef>,
}

impl Decode for NiMultiTargetTransformController {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let next_controller = <Option<BlockRef>>::decode(reader)?;
        let flags = TimeControllerFlags::decode(reader)?;
        let frequency = f32::decode(reader)?;
        let phase = f32::decode(reader)?;
        let start_time = f32::decode(reader)?;
        let stop_time = f32::decode(reader)?;

        let target = <Option<BlockRef>>::decode(reader)?;

        let mut targets = target.map(|v| vec![v]).unwrap_or_default();
        let extra_target_count = u16::decode(reader)?;
        for _ in 0..extra_target_count {
            // Sometimes contains `None` targets for some reason.
            if let Some(target) = <Option<BlockRef>>::decode(reader)? {
                targets.push(target);
            }
        }

        Ok(Self {
            next_controller,
            flags,
            frequency,
            phase,
            start_time,
            stop_time,
            targets,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NiControllerSequence {
    pub name: String,
    pub blocks: Vec<NiControllerSequenceControlledBlock>,
    pub array_grow_by: u32,
    pub weight: f32,
    pub text_keys: Option<BlockRef>,
    pub cycle_mode: SequenceCycleMode,
    pub frequency: f32,
    pub start_time: f32,
    pub stop_time: f32,
    pub manager: Option<BlockRef>,
    pub accum_root_name: String,
}

impl Decode for NiControllerSequence {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_string()?.to_owned();

        let num_controlled_blocks = u32::decode(reader)?;
        let array_grow_by = u32::decode(reader)?;

        let blocks = (0..num_controlled_blocks)
            .map(|_| NiControllerSequenceControlledBlock::decode(reader))
            .collect::<Result<_, _>>()?;

        let weight = f32::decode(reader)?;
        let text_keys = <Option<BlockRef>>::decode(reader)?;
        let cycle_mode = SequenceCycleMode::decode(reader)?;
        let frequency = f32::decode(reader)?;
        let start_time = f32::decode(reader)?;
        let stop_time = f32::decode(reader)?;

        let manager = <Option<BlockRef>>::decode(reader)?;
        let accum_root_name = reader.get_string()?.to_owned();

        Ok(Self {
            name,
            blocks,
            array_grow_by,
            weight,
            text_keys,
            cycle_mode,
            frequency,
            start_time,
            stop_time,
            manager,
            accum_root_name,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SequenceCycleMode {
    Loop,
    Reverse,
    Clamp,
}

impl Decode for SequenceCycleMode {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match reader.get_u32()? {
            0 => Ok(Self::Loop),
            1 => Ok(Self::Reverse),
            2 => Ok(Self::Clamp),
            n => Err(Error::UnknownEnumValue(
                stringify!(SequenceCycleMode),
                n.into(),
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NiControllerSequenceControlledBlock {
    pub interpolator: BlockRef,
    pub controller: BlockRef,
    pub priority: u8,
    pub node_name: String,
    pub property_type: String,
    pub controller_type: String,
    pub controller_id: String,
    pub interpolator_id: String,
}

impl Decode for NiControllerSequenceControlledBlock {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let interpolator = BlockRef::decode(reader)?;
        let controller = BlockRef::decode(reader)?;
        let priority = u8::decode(reader)?;
        let node_name = reader.get_string()?.to_owned();
        let property_type = reader.get_string()?.to_owned();
        let controller_type = reader.get_string()?.to_owned();
        let controller_id = reader.get_string()?.to_owned();
        let interpolator_id = reader.get_string()?.to_owned();

        Ok(Self {
            interpolator,
            controller,
            priority,
            node_name,
            property_type,
            controller_type,
            controller_id,
            interpolator_id,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeControllerFlags {
    pub active: bool,
    pub mode: TimeControllerLoopMode,
}

impl Decode for TimeControllerFlags {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let bits = reader.get_u16()?;

        let active = bits & 8 != 0;
        let mode = match (bits >> 1) & 0b11 {
            0b00 => TimeControllerLoopMode::Cycle,
            0b01 => TimeControllerLoopMode::Reverse,
            0b10 => TimeControllerLoopMode::Clamp,
            0b11 => todo!(),
            _ => unreachable!(),
        };

        Ok(Self { active, mode })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeControllerLoopMode {
    Cycle,
    Reverse,
    Clamp,
}

nif_struct! {
    /// Controller that animates variables of a [`BSLightingShaderProperty`].
    ///
    /// [`BSLightingShaderProperty`]: crate::material::BSLightingShaderProperty
    #[derive(Clone, Debug)]
    pub struct BSLightingShaderPropertyFloatController {
        pub next_controller: Option<BlockRef>,
        pub flags: TimeControllerFlags,
        pub frequency: f32,
        pub phase: f32,
        pub start_time: f32,
        pub stop_time: f32,
        /// The node being controlled.
        pub target: Option<BlockRef>,
        pub interpolator: Option<BlockRef>,
        pub variable: BSLightingShaderPropertyFloatVariable,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BSLightingShaderPropertyFloatVariable {
    RefractionStrength,
    EmissiveStrenth,
    Alpha,
    UOffset,
    VOffset,
    UScale,
    VScale,
}

impl Decode for BSLightingShaderPropertyFloatVariable {
    fn decode(reader: &mut crate::Reader<'_>) -> Result<Self, crate::Error> {
        match reader.get_u8()? {
            0 => Ok(Self::RefractionStrength),
            11 => Ok(Self::EmissiveStrenth),
            12 => Ok(Self::Alpha),
            20 => Ok(Self::UOffset),
            22 => Ok(Self::VOffset),
            21 => Ok(Self::UScale),
            23 => Ok(Self::VScale),
            v => Err(Error::UnknownEnumValue(
                stringify!(BSLightingShaderPropertyFloatVariable),
                v.into(),
            )),
        }
    }
}

nif_struct! {
    /// Controller that animates variables of a [`BSLightingShaderProperty`].
    ///
    /// [`BSLightingShaderProperty`]: crate::material::BSLightingShaderProperty
    #[derive(Clone, Debug)]
    pub struct BSLightingShaderPropertyColorController {
        pub next_controller: Option<BlockRef>,
        pub flags: TimeControllerFlags,
        pub frequency: f32,
        pub phase: f32,
        pub start_time: f32,
        pub stop_time: f32,
        pub target: Option<BlockRef>,
        pub interpolator: Option<BlockRef>,
        pub variable: BSLightingShaderPropertyColorVariable,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BSLightingShaderPropertyColorVariable {
    EmissiveColor,
}

impl Decode for BSLightingShaderPropertyColorVariable {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match reader.get_u8()? {
            1 => Ok(Self::EmissiveColor),
            v => Err(Error::UnknownEnumValue(
                stringify!(BSLightingShaderPropertyColorVariable),
                v.into(),
            )),
        }
    }
}

nif_struct! {
    /// Controller that animates variables of a [`BSEffectShaderProperty`].
    ///
    /// [`BSEffectShaderProperty`]: crate::material::BSEffectShaderProperty
    #[derive(Clone, Debug)]
    pub struct BSEffectShaderPropertyFloatController {
        pub next_controller: Option<BlockRef>,
        pub flags: TimeControllerFlags,
        pub frequency: f32,
        pub phase: f32,
        pub start_time: f32,
        pub end_time: f32,
        pub target: BlockRef,
        pub interpolator: BlockRef,
        pub controlled_variable: BSEffectShaderPropertyFloatVariable,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BSEffectShaderPropertyFloatVariable {
    EmissiveStrength,
    Alpha,
    VOffset,
    UOffset,
    VScale,
    UScale,
    FalloffStopAngle,
    FalloffStartOpacity,
}

impl Decode for BSEffectShaderPropertyFloatVariable {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match reader.get_u8()? {
            0 => Ok(Self::EmissiveStrength),
            2 => Ok(Self::FalloffStopAngle),
            3 => Ok(Self::FalloffStartOpacity),
            5 => Ok(Self::Alpha),
            6 => Ok(Self::UOffset),
            8 => Ok(Self::VOffset),
            7 => Ok(Self::UScale),
            9 => Ok(Self::VScale),
            v => Err(Error::UnknownEnumValue(
                stringify!(BSEffectShaderPropertyFloatVariable),
                v.into(),
            )),
        }
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiFloatInterpolator {
        pub value: f32,
        pub data: BlockRef,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiFloatData {
        pub data: NiDataKey<f32>,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiBoolData {
        pub data: NiDataKey<bool>,
    }
}

#[derive(Clone, Debug)]
pub enum NiDataKey<T> {
    Linear(Vec<NiDataLinearKey<T>>),
    Quadratic(Vec<NiDataQuadraticKey<T>>),
    Tbc(Vec<NiDataTbcKey<T>>),
    Const(Vec<NiDataConstKey<T>>),
}

impl<T> Decode for NiDataKey<T>
where
    T: Decode,
{
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let num_keys = reader.get_u32()?;
        let interpolation = InterpolationMode::decode(reader)?;

        match interpolation {
            InterpolationMode::Linear => {
                let keys = (0..num_keys)
                    .map(|_| NiDataLinearKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                Ok(Self::Linear(keys))
            }
            InterpolationMode::Quadratic => {
                let keys = (0..num_keys)
                    .map(|_| NiDataQuadraticKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                Ok(Self::Quadratic(keys))
            }
            InterpolationMode::Tbc => {
                let keys = (0..num_keys)
                    .map(|_| NiDataTbcKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                Ok(Self::Tbc(keys))
            }
            InterpolationMode::Const => {
                let keys = (0..num_keys)
                    .map(|_| NiDataConstKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                Ok(Self::Const(keys))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum NiDataKeyRotation<T> {
    Linear(Vec<NiDataLinearKey<T>>),
    Quadratic(Vec<NiDataQuadraticKey<T>>),
    Tbc(Vec<NiDataTbcKey<T>>),
    Const(Vec<NiDataConstKey<T>>),
    XyzRotation(Vec<XyzRotationKey>),
}

nif_struct! {
    #[derive(Copy, Clone, Debug)]
    pub struct NiDataLinearKey<T> {
        pub time: f32,
        pub value: T,
    }
}

nif_struct! {
    #[derive(Copy, Clone, Debug)]
    pub struct NiDataQuadraticKey<T> {
        pub time: f32,
        pub value: T,
        pub forward: T,
        pub backward: T,
    }
}

nif_struct! {
    #[derive(Copy, Clone, Debug)]
    pub struct NiDataConstKey<T> {
        pub time: f32,
        pub value: T,
    }
}

nif_struct! {
    #[derive(Copy, Clone, Debug)]
    pub struct NiDataTbcKey<T> {
        pub time: f32,
        pub value: T,
        pub tbc: Vec3,
    }
}

#[derive(Clone, Debug)]
pub struct XyzRotationKey {
    pub keys: NiDataKey<f32>,
}

impl Decode for XyzRotationKey {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let num_keys = reader.get_u32()?;
        if num_keys == 0 {
            return Ok(Self {
                keys: NiDataKey::Const(Vec::new()),
            });
        }

        let interpolation = InterpolationMode::decode(reader)?;

        let keys = match interpolation {
            InterpolationMode::Linear => {
                let keys = (0..num_keys)
                    .map(|_| NiDataLinearKey::decode(reader))
                    .collect::<Result<_, _>>()?;
                NiDataKey::Linear(keys)
            }
            InterpolationMode::Quadratic => {
                let keys = (0..num_keys)
                    .map(|_| NiDataQuadraticKey::decode(reader))
                    .collect::<Result<_, _>>()?;
                NiDataKey::Quadratic(keys)
            }
            InterpolationMode::Tbc => {
                let keys = (0..num_keys)
                    .map(|_| NiDataTbcKey::decode(reader))
                    .collect::<Result<_, _>>()?;
                NiDataKey::Tbc(keys)
            }
            InterpolationMode::Const => {
                let keys = (0..num_keys)
                    .map(|_| NiDataConstKey::decode(reader))
                    .collect::<Result<_, _>>()?;
                NiDataKey::Const(keys)
            }
        };

        Ok(Self { keys })
    }
}

#[derive(Clone, Debug)]
pub struct NiTextKeyExtraData {
    pub name: String,
    pub keys: Vec<NiTextKey>,
}

impl Decode for NiTextKeyExtraData {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_string()?.to_owned();

        let num_keys = u32::decode(reader)?;
        let keys = (0..num_keys)
            .map(|_| NiTextKey::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self { name, keys })
    }
}

#[derive(Clone, Debug)]
pub struct NiTextKey {
    pub time: f32,
    pub value: String,
}

impl Decode for NiTextKey {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let time = f32::decode(reader)?;
        let value = reader.get_string()?.to_owned();

        Ok(Self { time, value })
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiTransformInterpolator {
        pub translation: Vec3,
        pub rotation: Quat,
        pub scale: f32,
        pub data: BlockRef,
    }
}

#[derive(Clone, Debug)]
pub struct NiTransformData {
    pub translations: Option<NiDataKey<Vec3>>,
    pub rotations: Option<NiDataKeyRotation<Quat>>,
    pub scales: Option<NiDataKey<f32>>,
}

impl Decode for NiTransformData {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let num_rotation_keys = reader.get_u32()?;

        let rotations = if num_rotation_keys != 0 {
            let interpolation = RotationInterpolationMode::decode(reader)?;
            let keys = match interpolation {
                RotationInterpolationMode::Linear => {
                    let keys = (0..num_rotation_keys)
                        .map(|_| NiDataLinearKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKeyRotation::Linear(keys)
                }
                RotationInterpolationMode::Quadratic => {
                    let keys = (0..num_rotation_keys)
                        .map(|_| NiDataQuadraticKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKeyRotation::Quadratic(keys)
                }
                RotationInterpolationMode::Tbc => {
                    let keys = (0..num_rotation_keys)
                        .map(|_| NiDataTbcKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKeyRotation::Tbc(keys)
                }
                RotationInterpolationMode::Const => {
                    let keys = (0..num_rotation_keys)
                        .map(|_| NiDataConstKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKeyRotation::Const(keys)
                }
                RotationInterpolationMode::XyzRotation => {
                    assert_eq!(num_rotation_keys, 1);

                    let keys = (0..num_rotation_keys * 3)
                        .map(|_| XyzRotationKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKeyRotation::XyzRotation(keys)
                }
            };

            Some(keys)
        } else {
            None
        };

        let num_translation_keys = reader.get_u32()?;
        let translations = if num_translation_keys != 0 {
            let interpolation = InterpolationMode::decode(reader)?;
            let keys = match interpolation {
                InterpolationMode::Linear => {
                    let keys = (0..num_translation_keys)
                        .map(|_| NiDataLinearKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Linear(keys)
                }
                InterpolationMode::Quadratic => {
                    let keys = (0..num_translation_keys)
                        .map(|_| NiDataQuadraticKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Quadratic(keys)
                }
                InterpolationMode::Tbc => {
                    let keys = (0..num_translation_keys)
                        .map(|_| NiDataTbcKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Tbc(keys)
                }
                InterpolationMode::Const => {
                    let keys = (0..num_translation_keys)
                        .map(|_| NiDataConstKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Const(keys)
                }
            };

            Some(keys)
        } else {
            None
        };

        let num_scale_keys = reader.get_u32()?;
        let scales = if num_scale_keys != 0 {
            let interpolation = InterpolationMode::decode(reader)?;
            let keys = match interpolation {
                InterpolationMode::Linear => {
                    let keys = (0..num_scale_keys)
                        .map(|_| NiDataLinearKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Linear(keys)
                }
                InterpolationMode::Quadratic => {
                    let keys = (0..num_scale_keys)
                        .map(|_| NiDataQuadraticKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Quadratic(keys)
                }
                InterpolationMode::Tbc => {
                    let keys = (0..num_scale_keys)
                        .map(|_| NiDataTbcKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Tbc(keys)
                }
                InterpolationMode::Const => {
                    let keys = (0..num_scale_keys)
                        .map(|_| NiDataConstKey::decode(reader))
                        .collect::<Result<_, _>>()?;
                    NiDataKey::Const(keys)
                }
            };

            Some(keys)
        } else {
            None
        };

        Ok(Self {
            translations,
            rotations,
            scales,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum InterpolationMode {
    Linear,
    Quadratic,
    Tbc,
    Const,
}

impl Decode for InterpolationMode {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match reader.get_u32()? {
            1 => Ok(Self::Linear),
            2 => Ok(Self::Quadratic),
            3 => Ok(Self::Tbc),
            5 => Ok(Self::Const),
            v => {
                return Err(Error::UnknownEnumValue(
                    stringify!(InterpolationMode),
                    v.into(),
                ));
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum RotationInterpolationMode {
    Linear,
    Quadratic,
    Tbc,
    XyzRotation,
    Const,
}

impl Decode for RotationInterpolationMode {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        match reader.get_u32()? {
            1 => Ok(Self::Linear),
            2 => Ok(Self::Quadratic),
            3 => Ok(Self::Tbc),
            4 => Ok(Self::XyzRotation),
            5 => Ok(Self::Const),
            v => {
                return Err(Error::UnknownEnumValue(
                    stringify!(InterpolationMode),
                    v.into(),
                ));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NiDefaultAVObjectPalette {
    pub scene: BlockRef,
    pub objects: Vec<NiDefaultAVObject>,
}

impl Decode for NiDefaultAVObjectPalette {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let scene = BlockRef::decode(reader)?;

        let num_objects = reader.get_u32()?;
        let objects = (0..num_objects)
            .map(|_| NiDefaultAVObject::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self { scene, objects })
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiDefaultAVObject {
        pub name: String,
        pub object: BlockRef,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiTransformController {
        pub next_controller: Option<BlockRef>,
        pub flags: TimeControllerFlags,
        pub frequency: f32,
        pub phase: f32,
        pub start_time: f32,
        pub end_time: f32,
        pub target: BlockRef,
        pub interpolator: BlockRef,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct BSNiAlphaPropertyTestRefController {
        pub next_controller: Option<BlockRef>,
        pub flags: TimeControllerFlags,
        pub frequency: f32,
        pub phase: f32,
        pub start_time: f32,
        pub end_time: f32,
        pub target: BlockRef,
        pub interpolator: BlockRef,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiBlendFloatInterpolator {
        pub flags: u8,
        pub array_size: u8,
        pub weight_threshold: f32,
        pub value: f32,
    }
}

#[derive(Clone, Debug)]
pub struct NiPSysEmitterSpeedCtlr {
    pub next_controller: Option<BlockRef>,
    pub flags: TimeControllerFlags,
    pub frequency: f32,
    pub phase: f32,
    pub start_time: f32,
    pub end_time: f32,
    pub target: BlockRef,
    pub interpolator: BlockRef,
    pub modifier_name: String,
}

impl Decode for NiPSysEmitterSpeedCtlr {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let next_controller = <Option<BlockRef>>::decode(reader)?;
        let flags = TimeControllerFlags::decode(reader)?;
        let frequency = f32::decode(reader)?;
        let phase = f32::decode(reader)?;
        let start_time = f32::decode(reader)?;
        let end_time = f32::decode(reader)?;
        let target = BlockRef::decode(reader)?;
        let interpolator = BlockRef::decode(reader)?;

        let modifier_name = reader.get_u32()?;
        let modifier_name = reader.string(modifier_name)?.to_owned();

        Ok(Self {
            next_controller,
            flags,
            frequency,
            phase,
            start_time,
            end_time,
            target,
            interpolator,
            modifier_name,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NiPSysEmitterCtlr {
    pub next_controller: Option<BlockRef>,
    pub flags: TimeControllerFlags,
    pub frequency: f32,
    pub phase: f32,
    pub start_time: f32,
    pub end_time: f32,
    pub target: BlockRef,
    pub interpolator: BlockRef,
    pub modifier_name: String,
    pub visibility_interpolator: BlockRef,
}

impl Decode for NiPSysEmitterCtlr {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let next_controller = <Option<BlockRef>>::decode(reader)?;
        let flags = TimeControllerFlags::decode(reader)?;
        let frequency = f32::decode(reader)?;
        let phase = f32::decode(reader)?;
        let start_time = f32::decode(reader)?;
        let end_time = f32::decode(reader)?;
        let target = BlockRef::decode(reader)?;
        let interpolator = BlockRef::decode(reader)?;

        let modifier_name = reader.get_u32()?;
        let modifier_name = reader.string(modifier_name)?.to_owned();

        let visibility_interpolator = BlockRef::decode(reader)?;

        Ok(Self {
            next_controller,
            flags,
            frequency,
            phase,
            start_time,
            end_time,
            target,
            interpolator,
            modifier_name,
            visibility_interpolator,
        })
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiPSysUpdateCtlr {
        pub next_controller: Option<BlockRef>,
        pub flags: TimeControllerFlags,
        pub frequency: f32,
        pub phase: f32,
        pub start_time: f32,
        pub end_time: f32,
        pub target: BlockRef,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiBoolInterpolator {
        pub value: bool,
        pub data: Option<BlockRef>,
    }
}

nif_struct! {
    #[derive(Clone, Debug)]
    pub struct NiBlendBoolInterpolator {
        pub flags: u8,
        pub array_size: u8,
        pub weight_threshold: f32,
        pub value: u8,
    }
}
