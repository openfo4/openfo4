use hashbrown::HashMap;
use openfo4_common::collections::vec_map::VecMap;
use openfo4_core::ecs::World;
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{Scene, SceneActionData};
use openfo4_esp::{RecordChildren, RecordData};

use crate::state::esp::{self, EspLoader};
use crate::systems::conditions::{CompiledCondition, ConditionCompiler};
use crate::systems::quests::dialogue::{DialogueDescriptor, DialogueQueue, DialogueSound};
use crate::systems::quests::{AliasState, QuestManager, StageId};
use crate::systems::scripting::Executor;

#[derive(Clone, Debug)]
pub struct SceneInfo {
    actions: VecMap<u32, Action>,
    phases: VecMap<u32, Phase>,
    /// Quest stage to set when this scene is started.
    pub set_stage_begin: Option<StageId>,
    /// Quest stage to setwhen this scene finished playing.
    pub set_stage_end: Option<StageId>,
    pub quest: RecordId,
}

impl SceneInfo {
    pub async fn load(
        esp: &EspLoader,
        compiler: &ConditionCompiler,
        scene: &Scene,
        id: RecordId,
    ) -> Result<Self, esp::Error> {
        let mut phase_start = HashMap::<_, Vec<_>>::new();
        let mut phase_end = HashMap::<_, Vec<_>>::new();

        let mut actions = VecMap::new();
        for (index, action) in scene.actions.iter().enumerate() {
            let data = match &action.data {
                SceneActionData::Dialogue(data) => {
                    let topic = esp.resolve_form(data.topic, id).unwrap();
                    let infos = esp.children(RecordChildren::DialogueInfos, topic).await?;

                    let mut options = Vec::new();

                    for info in infos {
                        let info_id = info.id;
                        let RecordData::DialogueInfo(info) = info.local.data else {
                            continue;
                        };

                        let num_conditions = info.conditions.conditions.len();
                        let condition = compiler
                            .compile(info_id, 0, &info.conditions.conditions)
                            .await;

                        let mut dialogue = Vec::new();
                        for (index, _resp) in info.responses.iter().enumerate() {
                            dialogue.push(Dialogue {
                                id: info_id,
                                respnum: index + 1,
                            });
                        }

                        options.push((
                            DialogueOption {
                                dialogue,
                                condition,
                            },
                            num_conditions,
                        ));
                    }

                    // Dialogue options are chosen based on the number of conditions,
                    // options with more conditions are attempted first.
                    options.sort_by_key(|(_, num_conditions)| *num_conditions);
                    options.reverse();
                    let options = options.into_iter().map(|(v, _)| v).collect();

                    Action::PlayDialogue {
                        actor: action.actor.cast_unsigned(),
                        options,
                    }
                }
                _ => continue,
            };

            let index = index as u32;
            actions.insert(index, data);
            phase_start
                .entry(action.start_phase)
                .or_default()
                .push(index);
            phase_end.entry(action.end_phase).or_default().push(index);
        }

        let mut phases = VecMap::new();
        for (index, _phase) in scene.phases.iter().enumerate() {
            let index = index as u32;
            let start_actions = phase_start.remove(&index).unwrap_or_default();
            let end_actions = phase_end.remove(&index).unwrap_or_default();
            phases.insert(
                index,
                Phase {
                    start_actions,
                    end_actions,
                },
            );
        }

        let quest = esp.resolve_form(scene.parent_quest, id).unwrap();
        let set_stage_begin = scene
            .set_parent_quest_stage
            .and_then(|s| StageId::try_from(s.on_start).ok());
        let set_stage_end = scene
            .set_parent_quest_stage
            .and_then(|s| StageId::try_from(s.on_completion).ok());

        Ok(Self {
            actions,
            phases,
            quest,
            set_stage_begin,
            set_stage_end,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Phase {
    /// List of actions that are started when entering this `Phase`.
    start_actions: Vec<u32>,
    /// List of actions that must end before exiting this `Phase`.
    end_actions: Vec<u32>,
}

#[derive(Clone, Debug)]
enum Action {
    PlayDialogue {
        actor: u32,
        /// Choose the first dialogue that matches the conditions.
        options: Vec<DialogueOption>,
    },
}

#[derive(Clone, Debug)]
struct DialogueOption {
    /// Only play the dialogue when this condition is `true`.
    condition: CompiledCondition,
    dialogue: Vec<Dialogue>,
}

#[derive(Clone, Debug)]
struct Dialogue {
    id: RecordId,
    respnum: usize,
}

#[derive(Clone, Debug)]
pub struct SceneState {
    pub info: SceneInfo,
    active_phase: Option<u32>,
    active_actions: VecMap<u32, PlayingAction>,
    pub is_playing: bool,
}

impl SceneState {
    pub fn new(info: SceneInfo) -> Self {
        Self {
            info,
            active_phase: None,
            active_actions: VecMap::new(),
            is_playing: false,
        }
    }

    pub(super) fn advance(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        executor: &mut Executor,
        quests: &mut QuestManager,
        aliases: &HashMap<u32, AliasState>,
    ) -> bool {
        self.active_actions.retain(|_, action| !action.is_done());

        if let Some(phase) = self.active_phase {
            let phase = self.info.phases.get(phase).unwrap();

            // Current phase still has some actions active.
            if phase
                .end_actions
                .iter()
                .any(|action| self.active_actions.contains_key(*action))
            {
                return false;
            }
        }

        let next_phase = self.active_phase.map(|v| v + 1).unwrap_or_default();

        let Some(phase) = self.info.phases.get(next_phase) else {
            // No more phases, scene is complete.
            return true;
        };

        tracing::debug!("starting scene phase {next_phase}");
        self.active_phase = Some(next_phase);

        for action_id in &phase.start_actions {
            let action = self.info.actions.get(*action_id).unwrap();

            match action {
                Action::PlayDialogue { actor, options } => {
                    let Some(actor) = aliases.get(actor).and_then(|alias| alias.entity.as_ref())
                    else {
                        continue;
                    };

                    let Some(opt) = options.iter().find(|opt| {
                        opt.condition.eval_entity(
                            executor,
                            world,
                            events,
                            quests,
                            actor.entity(),
                            None,
                        )
                    }) else {
                        continue;
                    };

                    if world.get::<&DialogueQueue>(actor.entity()).is_err() {
                        world
                            .insert_one(actor.entity(), DialogueQueue::default())
                            .unwrap();
                    }

                    let mut queue = world.get::<&mut DialogueQueue>(actor.entity()).unwrap();

                    let sounds = opt
                        .dialogue
                        .iter()
                        .map(|dialogue| {
                            queue.push(DialogueDescriptor {
                                record: dialogue.id,
                                respnum: dialogue.respnum,
                            })
                        })
                        .collect();

                    self.active_actions.insert(
                        *action_id,
                        PlayingAction::Dialogue(PlayingDialogue { sounds }),
                    );
                }
            }
        }

        false
    }
}

#[derive(Clone, Debug)]
enum PlayingAction {
    Dialogue(PlayingDialogue),
}

impl PlayingAction {
    fn is_done(&self) -> bool {
        match self {
            Self::Dialogue(action) => action.sounds.iter().all(|sound| sound.is_done()),
        }
    }
}

#[derive(Clone, Debug)]
struct PlayingDialogue {
    sounds: Vec<DialogueSound>,
}
