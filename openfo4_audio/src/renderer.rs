use std::sync::Arc;

use crossbeam_queue::SegQueue;
use event_listener::Event;
use openfo4_common::collections::vec_map::VecMap;
use openfo4_tracing::trace_span;

use crate::mixer::Mixer;
use crate::sound::Frame;
use crate::source::{AudioSource, Error};
use crate::spatial::{self, Emitter, Listener};
use crate::track::TrackId;

pub struct Renderer {
    mixer: Mixer,
    sounds: VecMap<usize, Sound>,
    queue: Arc<SegQueue<Command>>,
    scratch: Vec<Frame>,
    emitters: VecMap<usize, Emitter>,
    listeners: VecMap<usize, Listener>,
}

impl Renderer {
    pub fn new() -> Self {
        let mut mixer = Mixer::default();
        mixer.add_track(TrackId::Main, None);

        Self {
            mixer,
            sounds: VecMap::new(),
            queue: Arc::default(),
            scratch: Vec::new(),
            emitters: VecMap::new(),
            listeners: VecMap::new(),
        }
    }

    pub fn queue(&self) -> &Arc<SegQueue<Command>> {
        &self.queue
    }

    pub fn render(&mut self, buf: &mut [f32], num_channels: usize) {
        let _span = trace_span!("Renderer::render").entered();

        if num_channels == 0 {
            return;
        }

        let num_frames = buf.len() / num_channels;
        self.mixer.reserve(num_frames);
        self.scratch.resize(num_frames, Frame::EQUILIBRIUM);

        for sound in self.sounds.values_mut() {
            if !sound.is_playing {
                continue;
            }

            let scratch = &mut self.scratch[..num_frames];
            let scratch = match sound.source.read(scratch, sound.looping) {
                Ok(frames_written) => &mut scratch[..frames_written],
                Err(Error::Eof) if !sound.looping => {
                    sound.is_playing = false;

                    debug_assert_eq!(sound.on_done.total_listeners(), 1);
                    sound.on_done.notify(1);
                    continue;
                }
                Err(err) => {
                    tracing::error!("error while reading from audio source: {}", err);
                    continue;
                }
            };

            if let Some(emitter) = sound.emitter {
                let emitter = self.emitters.get(emitter).unwrap();
                for listener in self.listeners.values() {
                    for frame in &mut *scratch {
                        *frame = spatial::process(listener, emitter, *frame);
                    }
                }
            }

            for (dst, src) in self.mixer.get_buffer(sound.track).iter_mut().zip(scratch) {
                *dst += *src;
                *src = Frame::EQUILIBRIUM;
            }
        }

        self.mixer.process();

        let output = &self.mixer.get_buffer(TrackId::Main)[..num_frames];
        for (dst, src) in buf.chunks_mut(num_channels).zip(output) {
            match num_channels {
                1 => {
                    dst[0] = (src.left + src.right) / 2.0;
                }
                _ => {
                    dst[0] = src.left;
                    dst[1] = src.right;
                }
            }

            // Skip all channels over stereo.
            for i in 2..num_channels {
                dst[i as usize] = 0.0;
            }
        }
    }

    pub fn process_commands(&mut self) {
        let _span = trace_span!("Renderer::process_commands").entered();

        while let Some(cmd) = self.queue.pop() {
            match cmd {
                Command::CreateTrack { id, destination } => {
                    self.mixer.add_track(id, Some(destination));
                }
                Command::PlaySound { id, sound } => {
                    self.sounds.insert(id, sound);
                }
                Command::StopSound { id } => {
                    self.sounds.get_mut(id).unwrap().is_playing = false;
                }
                Command::DropSound { id } => {
                    self.sounds.remove(id);
                }
                Command::CreateListener { id, listener }
                | Command::UpdateListener { id, listener } => {
                    self.listeners.insert(id, listener);
                }
                Command::DestroyListener { id } => {
                    self.listeners.remove(id);
                }
                Command::CreateEmitter { id, emitter } | Command::UpdateEmitter { id, emitter } => {
                    self.emitters.insert(id, emitter);
                }
                Command::DestroyEmitter { id } => {
                    self.emitters.remove(id);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Sound {
    pub source: AudioSource,
    pub track: TrackId,
    pub looping: bool,
    pub on_done: Event,
    pub is_playing: bool,
    pub emitter: Option<usize>,
}

#[derive(Debug)]
pub enum Command {
    CreateTrack { id: TrackId, destination: TrackId },
    PlaySound { id: usize, sound: Sound },
    StopSound { id: usize },
    DropSound { id: usize },
    CreateEmitter { id: usize, emitter: Emitter },
    UpdateEmitter { id: usize, emitter: Emitter },
    DestroyEmitter { id: usize },
    CreateListener { id: usize, listener: Listener },
    UpdateListener { id: usize, listener: Listener },
    DestroyListener { id: usize },
}
