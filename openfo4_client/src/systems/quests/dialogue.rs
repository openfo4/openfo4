use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use openfo4_asset::Handle;
use openfo4_audio::AudioManager;
use openfo4_audio::manager::{PlaybackSettings, Sound, TrackId};
use openfo4_core::ecs::World;
use openfo4_core::events::EventBus;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_tasks::Task;

use crate::assets::Error;
use crate::assets::audio::FileAudioSource;
use crate::components::{ActorMarker, ActorProperties, DeadActor};
use crate::systems::ActivationEvent;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::scene::PlayingDialogue;
use crate::systems::quests::{QuestManager, StageId};
use crate::systems::scripting::Executor;

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
            on_end: descriptor.on_end,
        };
        self.queue.push_back((descriptor, sound.clone()));
        sound
    }
}

/// A dialogue sound that is currently queued or playing.
#[derive(Clone, Debug)]
pub struct DialogueSound {
    done: Arc<AtomicBool>,
    on_end: DialogueTriggers,
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
    /// Triggers to execute when the dialogue starts playback.
    pub on_start: DialogueTriggers,
    /// Triggers to execute when the dialogue finishes playback successfully.
    pub on_end: DialogueTriggers,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct DialogueTriggers {
    pub set_quest_stage: Option<(RecordId, StageId)>,
    pub start_scene: Option<RecordId>,
}

impl DialogueTriggers {
    fn run(&self, quests: &mut QuestManager) {
        if let Some((quest, stage)) = self.set_quest_stage {
            quests.set_quest_stage(quest, stage);
        }

        if let Some(scene) = self.start_scene {
            quests.start_scene(scene);
        }
    }
}

pub fn play_actor_dialogue(
    world: &mut World,
    audio: &mut AudioManager,
    globals: &GlobalProperties,
    quests: &mut QuestManager,
) {
    for (mut queue, props) in world.query_mut::<(&mut DialogueQueue, &ActorProperties)>() {
        while let Some((descriptor, sound)) = queue.queue.pop_front() {
            let mut path = DataPathBuf::new("sound/voice");
            let Some(plugin) = quests.esp_loader.file(descriptor.record.module) else {
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

            let loader = quests.loader.clone();
            let task = quests
                .pool
                .spawn(async move { loader.load::<FileAudioSource, _>(&path).await });
            queue
                .loading
                .push_back((LoadingDialogue { descriptor, task }, sound));
        }

        if let Some((sound, diag_sound)) = &mut queue.playing {
            if sound.is_playing() {
                continue;
            }

            diag_sound.done.store(true, Ordering::Relaxed);
            diag_sound.on_end.run(quests);
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

            diag.descriptor.on_start.run(quests);

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

pub fn initiate_player_npc_interactions(
    world: &mut World,
    events: &mut EventBus,
    quests: &mut QuestManager,
    executor: &mut Executor,
) {
    let mut topics = Vec::new();

    for event in events.read::<ActivationEvent>() {
        if world.get::<&ActorMarker>(event.target).is_err()
            || world.get::<&DeadActor>(event.target).is_ok()
        {
            continue;
        }

        let Some(ids) = quests.topics.hello.get(&event.target) else {
            continue;
        };

        for id in ids {
            let quest = quests.quests.get(id).unwrap();

            for topic in &quest.hello_topics {
                topics.push((event.target, *id, topic.clone()));
            }
        }
    }

    for (actor, quest, topic) in topics {
        // Run until completion.
        PlayingDialogue::new(actor, &topic, world, events, executor, quests, quest);
    }
}

#[derive(Debug)]
struct LoadingDialogue {
    descriptor: DialogueDescriptor,
    task: Task<Result<Handle<FileAudioSource>, Error>>,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerDialogueQueue(VecDeque<PlayerDialogue>);

impl PlayerDialogueQueue {
    pub fn push(&mut self, val: PlayerDialogue) {
        self.0.push_back(val);
    }

    pub fn front(&mut self) -> Option<&mut PlayerDialogue> {
        self.0.front_mut()
    }

    pub fn pop_front(&mut self) {
        self.0.pop_front();
    }
}

#[derive(Clone, Debug)]
pub struct PlayerDialogue {
    pub options: Vec<String>,
    pub selector: Arc<OnceLock<usize>>,
}
