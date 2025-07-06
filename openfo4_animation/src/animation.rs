use std::collections::HashMap;
use std::time::Duration;

use bitflags::bitflags;
use game_common::components::Transform;

#[derive(Clone, Debug)]
pub struct AnimationClip {
    pub duration: Duration,
    /// Number of bones in this `AnimationClip`.
    pub num_bones: u32,
    pub num_frames: u32,
    /// Keyframes stored as two nested array where the outer represents the keyframe and the inner
    /// represents the bone index.
    pub keyframes: Vec<Transform>,
    pub skeleton_bone_to_animation_bone: HashMap<usize, usize>,
    pub annotations: Vec<Annotation>,
    pub flags: AnimationClipFlags,
}

impl AnimationClip {
    /// Returns the duration of a single frame.
    ///
    /// Returns [`Duration::ZERO`] if the animation contains no frames.
    pub fn frame_duration(&self) -> Duration {
        self.duration
            .checked_div(self.num_frames)
            .unwrap_or_default()
    }

    pub fn get(&self, frame: u32, bone: u32) -> Option<Transform> {
        let index = self.num_bones as usize * frame as usize + bone as usize;
        self.keyframes.get(index).copied()
    }
}

#[derive(Clone, Debug)]
pub enum Annotation {
    /// Synchronization point with the given name.
    Sync(Duration, String),
}

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct AnimationClipFlags: u8 {
        /// Trim the last frame of the [`AnimationClip`] if the clip is played looping.
        ///
        /// This is useful to prevent one-frame stutters if the start and end frames contain the
        /// same data.
        const TRIM_LAST_FRAME_IF_LOOPING = 0x01;
    }
}
