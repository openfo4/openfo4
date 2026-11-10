use crate::effects::Volume;
use crate::sound::{Destination, Frame};
use crate::spatial::Emitter;
use crate::track::TrackId;

#[derive(Clone, Debug)]
pub struct SoundData {
    pub frames: Vec<Frame>,
    pub sample_rate: u32,
    pub volume: Volume,
}

#[derive(Clone, Debug, Default)]
pub struct Settings {
    /// The track on which the sound will be rendered on.
    pub track: TrackId,
    pub emitter: Option<usize>,
    pub destination: Destination,
    /// Whether the sound loops.
    pub looping: bool,
}
