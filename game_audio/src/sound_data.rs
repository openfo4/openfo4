use crate::effects::Volume;
use crate::sound::{Destination, Frame};

#[derive(Clone, Debug)]
pub struct SoundData {
    pub frames: Vec<Frame>,
    pub sample_rate: u32,
    pub volume: Volume,
}

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub destination: Destination,
    /// Whether the sound loops.
    pub looping: bool,
}
