use glam::Vec3;

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
    pub cycle_type: u32,
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
        let cycle_type = u32::decode(reader)?;
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
            cycle_type,
            frequency,
            start_time,
            stop_time,
            manager,
            accum_root_name,
        })
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
    #[derive(Clone, Debug)]
    pub struct NiFloatInterpolator {
        pub value: f32,
        pub data: BlockRef,
    }
}

#[derive(Clone, Debug)]
pub struct NiFloatData {
    pub data: NiDataKey<f32>,
}

impl Decode for NiFloatData {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let num_keys = reader.get_u32()?;
        let interpolation = reader.get_u32()?;

        let data = match interpolation {
            1 => {
                let keys = (0..num_keys)
                    .map(|_| NiDataLinearKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                NiDataKey::Linear(keys)
            }
            2 => {
                let keys = (0..num_keys)
                    .map(|_| NiDataQuadraticKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                NiDataKey::Quadratic(keys)
            }
            3 => {
                let keys = (0..num_keys)
                    .map(|_| NiDataTbcKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                NiDataKey::Tbc(keys)
            }
            5 => {
                let keys = (0..num_keys)
                    .map(|_| NiDataConstKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                NiDataKey::Const(keys)
            }
            v => return Err(Error::UnknownEnumValue(stringify!(NiFloatData), v.into())),
        };

        Ok(Self { data })
    }
}

#[derive(Clone, Debug)]
pub enum NiDataKey<T> {
    Linear(Vec<NiDataLinearKey<T>>),
    Quadratic(Vec<NiDataQuadraticKey<T>>),
    Tbc(Vec<NiDataTbcKey<T>>),
    Const(Vec<NiDataConstKey<T>>),
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
        pub forward: f32,
        pub backward: f32,
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
