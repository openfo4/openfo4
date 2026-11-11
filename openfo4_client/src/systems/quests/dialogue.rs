use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use openfo4_asset::{AssetLoader, Handle};
use openfo4_audio::AudioManager;
use openfo4_audio::manager::{PlaybackSettings, Sound, TrackId};
use openfo4_core::ecs::World;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_tasks::{Task, TaskPool};

use crate::assets::Error;
use crate::assets::audio::FileAudioSource;
use crate::components::ActorProperties;
use crate::state::esp::EspLoader;
use crate::systems::global::GlobalProperties;

#[derive(Debug, Default)]
pub struct DialogueQueue {
    queue: VecDeque<(DialogueDescriptor, DialogueSound)>,
    playing: Option<(Sound, DialogueSound)>,
    loading: VecDeque<(LoadingDialogue, DialogueSound)>,
}

impl DialogueQueue {
    pub fn push(&mut self, descriptor: DialogueDescriptor) -> DialogueSound {
        let sound = DialogueSound {
            done: Arc::default(),
        };
        self.queue.push_back((descriptor, sound.clone()));
        sound
    }
}

/// A dialogue sound that is currently queued or playing.
#[derive(Clone, Debug)]
pub struct DialogueSound {
    done: Arc<AtomicBool>,
}

impl DialogueSound {
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug)]
pub struct DialogueDescriptor {
    /// The line of dialogue to speak.
    pub record: RecordId,
    /// Number of the response.
    pub respnum: usize,
}

pub fn play_actor_dialogue(
    world: &mut World,
    loader: &AssetLoader,
    audio: &mut AudioManager,
    esp_loader: &EspLoader,
    pool: &TaskPool,
    globals: &GlobalProperties,
) {
    for (mut queue, props) in world.query_mut::<(&mut DialogueQueue, &ActorProperties)>() {
        while let Some((descriptor, sound)) = queue.queue.pop_front() {
            let mut path = DataPathBuf::new("sound/voice");
            let Some(plugin) = esp_loader.file(descriptor.record.module) else {
                tracing::debug!(
                    "skipping playback of {}, no parent module",
                    descriptor.record
                );

                continue;
            };

            let Some(voice_type) = props.voice_type.and_then(|id| globals.voice_types.get(&id))
            else {
                tracing::debug!("actor has no voice type, skipping dialogue");
                continue;
            };

            path.push(plugin.as_ref());
            path.push(voice_type.as_str().as_inner());
            path.push(&*format!(
                "00{}_{}.fuz",
                descriptor.record.form, descriptor.respnum
            ));

            let loader = loader.clone();
            let task = pool.spawn(async move { loader.load::<FileAudioSource, _>(&path).await });
            queue
                .loading
                .push_back((LoadingDialogue { descriptor, task }, sound));
        }

        if let Some((sound, diag_sound)) = &mut queue.playing {
            if sound.is_playing() {
                continue;
            }

            diag_sound.done.store(true, Ordering::Relaxed);
            queue.playing = None;
        }

        if let Some((mut diag, diag_sound)) = queue.loading.pop_front() {
            let Some(source) = diag.task.get_output() else {
                queue.loading.push_front((diag, diag_sound));
                continue;
            };

            let source = match source {
                Ok(source) => source,
                Err(err) => {
                    tracing::error!(
                        "skipping playback of {}, failed to load audio: {}",
                        diag.descriptor.record,
                        err
                    );
                    continue;
                }
            };

            let sound = audio.play(
                source.stream(),
                PlaybackSettings {
                    looping: false,
                    track: TrackId::Main,
                    emitter: None,
                },
            );
            queue.playing = Some((sound, diag_sound));
        }
    }
}

#[derive(Debug)]
struct LoadingDialogue {
    descriptor: DialogueDescriptor,
    task: Task<Result<Handle<FileAudioSource>, Error>>,
}
