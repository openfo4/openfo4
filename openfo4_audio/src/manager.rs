use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crossbeam_queue::SegQueue;
use event_listener::{Event, EventListener};
use futures_lite::FutureExt;
use glam::{Quat, Vec3};
use openfo4_common::collections::slab::Slab;

use crate::backend::{Backend, DefaultBackend};
use crate::effects::Volume;
use crate::renderer::{self, Command, Renderer};
use crate::source::AudioSource;
use crate::spatial;

#[derive(Debug)]
pub struct AudioManager<B = DefaultBackend>
where
    B: Backend,
{
    _backend: B,
    sample_rate: u32,
    queue: Arc<SegQueue<Command>>,
    sounds: Slab<usize, ()>,
    tracks: Slab<usize, ()>,
    emitters: Slab<usize, ()>,
    listeners: Slab<usize, ()>,
}

impl<B> AudioManager<B>
where
    B: Backend,
{
    pub fn new(mut backend: B) -> Self {
        let sample_rate = 48_000;

        let renderer = Renderer::new();
        let queue = renderer.queue().clone();

        backend.create_output_stream(renderer);

        Self {
            _backend: backend,
            sample_rate: sample_rate as u32,
            sounds: Slab::new(),
            tracks: Slab::new(),
            emitters: Slab::new(),
            listeners: Slab::new(),
            queue,
        }
    }

    /// Begins playback of a new sound.
    pub fn play(&mut self, mut source: AudioSource, settings: PlaybackSettings) -> Sound {
        source.set_sample_rate(self.sample_rate);

        let event = Event::new();
        let listen = event.listen();

        let id = self.sounds.insert(());

        self.queue.push(Command::PlaySound {
            id,
            sound: renderer::Sound {
                source,
                track: settings.track,
                looping: settings.looping,
                on_done: event,
                is_playing: true,
                emitter: settings.emitter.as_ref().map(|v| v.0.id),
            },
        });

        Sound {
            id,
            queue: self.queue.clone(),
            stopped: Some(listen),
            _emitter_ref: settings.emitter,
        }
    }

    /// Creates a new virtual audio track.
    pub fn add_track(&mut self, track: Track) -> TrackId {
        let id = TrackId::Track(self.tracks.insert(()));

        self.queue.push(Command::CreateTrack {
            id,
            destination: track.target,
        });

        id
    }

    pub fn create_emitter(&mut self, translation: Vec3) -> Emitter {
        let id = self.emitters.insert(());
        self.queue.push(Command::CreateEmitter {
            id,
            emitter: spatial::Emitter { translation },
        });

        Emitter(Arc::new(EmitterInner {
            id,
            queue: self.queue.clone(),
        }))
    }

    pub fn create_listener(&mut self, translation: Vec3, rotation: Quat) -> Listener {
        let id = self.listeners.insert(());
        self.queue.push(Command::CreateListener {
            id,
            listener: spatial::Listener {
                translation,
                rotation,
            },
        });

        Listener(Arc::new(ListenerInner {
            id,
            queue: self.queue.clone(),
        }))
    }
}

#[derive(Debug)]
pub struct Sound {
    id: usize,
    queue: Arc<SegQueue<Command>>,
    stopped: Option<EventListener>,
    /// The emitter that this sound is being played from.
    /// It must be kept alive until the sound is dropped.
    _emitter_ref: Option<Emitter>,
}

impl Sound {
    pub fn stop(&mut self) {
        if self.stopped.is_some() {
            self.queue.push(Command::StopSound { id: self.id });
            self.stopped = None;
        }
    }

    /// Returns `true` if the sound is currently playing.
    pub fn is_playing(&mut self) -> bool {
        match &mut self.stopped {
            Some(stopped) => {
                let waker = Waker::noop();
                let mut cx = Context::from_waker(waker);
                match stopped.poll(&mut cx) {
                    Poll::Ready(()) => {
                        self.stopped = None;
                        true
                    }
                    Poll::Pending => false,
                }
            }
            None => true,
        }
    }

    /// Blocks until the sound has finished playing.
    ///
    /// Note that this will never happen if the sound is set to loop.
    pub async fn wait_until_stopped(&mut self) {
        if let Some(stopped) = &mut self.stopped {
            stopped.await;
            self.stopped = None;
        }
    }
}

impl Drop for Sound {
    fn drop(&mut self) {
        self.stop();
        self.queue.push(Command::DropSound { id: self.id });
    }
}

#[derive(Clone, Debug)]
pub struct Emitter(Arc<EmitterInner>);

impl Emitter {
    pub fn update(&mut self, translation: Vec3) {
        self.0.queue.push(Command::CreateEmitter {
            id: self.0.id,
            emitter: spatial::Emitter { translation },
        });
    }
}

#[derive(Debug)]
struct EmitterInner {
    id: usize,
    queue: Arc<SegQueue<Command>>,
}

impl Drop for EmitterInner {
    fn drop(&mut self) {
        self.queue.push(Command::DestroyEmitter { id: self.id });
    }
}

#[derive(Clone, Debug)]
pub struct Listener(Arc<ListenerInner>);

impl Listener {
    pub fn update(&mut self, translation: Vec3, rotation: Quat) {
        self.0.queue.push(Command::UpdateListener {
            id: self.0.id,
            listener: spatial::Listener {
                translation,
                rotation,
            },
        });
    }
}

#[derive(Debug)]
pub struct ListenerInner {
    id: usize,
    queue: Arc<SegQueue<Command>>,
}

impl Drop for ListenerInner {
    fn drop(&mut self) {
        self.queue.push(Command::DestroyListener { id: self.id });
    }
}

#[derive(Clone, Debug)]
pub struct PlaybackSettings {
    /// Infinitely loop the sound.
    pub looping: bool,
    /// The destination track where the sound will be rendered to.
    pub track: TrackId,
    /// The spatial emitter from which the sound originates from.
    pub emitter: Option<Emitter>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrackId {
    #[default]
    Main,
    Track(usize),
}

#[derive(Clone, Debug)]
pub struct Track {
    pub target: TrackId,
    /// Modify the volume of all incoming sounds.
    pub volume: Volume,
}
