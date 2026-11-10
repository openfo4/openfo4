use std::collections::HashMap;

use crate::sound::Frame;
use crate::track::TrackId;

#[derive(Debug, Default)]
pub struct Mixer {
    buffer_size: usize,
    tracks: HashMap<TrackId, Track>,
    windows: Vec<(TrackId, TrackId)>,
}

impl Mixer {
    pub fn get_buffer(&mut self, track: TrackId) -> &mut [Frame] {
        &mut self.tracks.get_mut(&track).unwrap().buffer
    }

    pub fn add_track(&mut self, track: TrackId, destination: Option<TrackId>) {
        self.tracks.insert(
            track,
            Track {
                buffer: vec![Frame::EQUILIBRIUM; self.buffer_size],
            },
        );

        if let Some(dst) = destination {
            self.windows.push((track, dst));
        }
    }

    /// Reserve capacity `len` empty frames for each track buffer.
    pub fn reserve(&mut self, len: usize) {
        if len > self.buffer_size {
            self.buffer_size = len;
        }

        for track in self.tracks.values_mut() {
            track.buffer.resize(len, Frame::EQUILIBRIUM);
            // Clear accumluated values from previous frame.
            track.buffer[..len].fill(Frame::EQUILIBRIUM);
        }
    }

    pub fn process(&mut self) {
        for (src, dst) in &self.windows {
            let [Some(src), Some(dst)] = self.tracks.get_disjoint_mut([src, dst]) else {
                unreachable!();
            };

            dst.buffer.copy_from_slice(&src.buffer);
        }
    }
}

#[derive(Debug)]
struct Track {
    buffer: Vec<Frame>,
}
