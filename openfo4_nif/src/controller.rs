use crate::{BlockRef, Decode, Error, Reader, nif_struct};

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
        pub variable: BSLightingShaderPropertyVariable,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BSLightingShaderPropertyVariable {
    UOffset,
    VOffset,
    UScale,
    VScale,
}

impl Decode for BSLightingShaderPropertyVariable {
    fn decode(reader: &mut crate::Reader<'_>) -> Result<Self, crate::Error> {
        match reader.get_u8()? {
            20 => Ok(Self::UOffset),
            22 => Ok(Self::VOffset),
            21 => Ok(Self::UScale),
            23 => Ok(Self::VScale),
            v => todo!("{:?}", v),
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
        dbg!(&interpolation);

        let data = match interpolation {
            2 => {
                let keys = (0..num_keys)
                    .map(|_| NiDataQuadraticKey::decode(reader))
                    .collect::<Result<_, _>>()?;

                NiDataKey::Quadratic(keys)
            }
            v => todo!("TODO: Unknown interpolation mode: {}", v),
        };

        Ok(Self { data })
    }
}

#[derive(Clone, Debug)]
pub enum NiDataKey<T> {
    Quadratic(Vec<NiDataQuadraticKey<T>>),
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
