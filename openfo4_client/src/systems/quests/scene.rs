use std::sync::{Arc, OnceLock};

use hashbrown::HashMap;
use openfo4_common::collections::vec_map::VecMap;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{Scene, SceneActionData};
use openfo4_esp::{RecordChildren, RecordData};

use crate::components::Player;
use crate::state::esp::{self, EspLoader};
use crate::systems::conditions::{CompiledCondition, ConditionCompiler};
use crate::systems::quests::dialogue::{
    DialogueDescriptor, DialogueQueue, DialogueSound, PlayerDialogue, PlayerDialogueQueue,
};
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
                    let options = load_dialogue(esp, compiler, topic).await?;

                    Action::PlayDialogue {
                        actor: action.actor.cast_unsigned(),
                        options,
                    }
                }
                SceneActionData::PlayerDialogue(data) => {
                    let mut options = Vec::new();
                    for (dialogue, resp) in [
                        (data.player_positive_response, data.npc_positive_response),
                        (data.player_negative_response, data.npc_negative_response),
                        (data.player_neutral_response, data.npc_neutral_response),
                        (data.player_question_response, data.npc_question_response),
                    ] {
                        let Some(dialogue) = dialogue else {
                            continue;
                        };

                        let dialogue = esp.resolve_form(dialogue, id).unwrap();
                        let dialogue = load_dialogue(esp, compiler, dialogue).await?;

                        let mut response = Vec::new();
                        if let Some(resp) = resp {
                            let resp = esp.resolve_form(resp, id).unwrap();
                            let resp = load_dialogue(esp, compiler, resp).await?;
                            response = resp;
                        }

                        options.push(PlayerDialogueOption { dialogue, response });
                    }

                    Action::PlayerDialogue {
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

async fn load_dialogue(
    esp: &EspLoader,
    compiler: &ConditionCompiler,
    id: RecordId,
) -> Result<Vec<DialogueOption>, esp::Error> {
    let infos = esp.children(RecordChildren::DialogueInfos, id).await?;

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

    Ok(options)
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
    PlayerDialogue {
        /// The actor that the player will speak with, and the actor that will respond.
        actor: u32,
        options: Vec<PlayerDialogueOption>,
    },
}

#[derive(Clone, Debug)]
struct PlayerDialogueOption {
    /// The line that the player speaks.
    dialogue: Vec<DialogueOption>,
    /// The line that the other actor responds with.
    response: Vec<DialogueOption>,
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
        for (index, state) in &mut self.active_actions {
            let action = self.info.actions.get(index).unwrap();
            state.update(action, world, events, executor, quests);
        }

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

                    let Some(action) = PlayingDialogue::new(
                        actor.entity(),
                        options,
                        world,
                        events,
                        executor,
                        quests,
                    ) else {
                        continue;
                    };

                    self.active_actions
                        .insert(*action_id, PlayingAction::Dialogue(action));
                }
                Action::PlayerDialogue { actor, options } => {
                    let Some(actor) = aliases.get(actor).and_then(|alias| alias.entity.as_ref())
                    else {
                        continue;
                    };

                    let Some((player, _)) = world.query::<(Entity, &Player)>().iter().next() else {
                        continue;
                    };

                    if world.get::<&PlayerDialogueQueue>(player).is_err() {
                        world
                            .insert_one(player, PlayerDialogueQueue::default())
                            .unwrap();
                    }

                    let mut queue = world.get::<&mut PlayerDialogueQueue>(player).unwrap();

                    let mut opts = Vec::new();
                    for (index, _option) in options.iter().enumerate() {
                        opts.push(index.to_string());
                    }

                    let selector = Arc::new(OnceLock::new());
                    queue.push(PlayerDialogue {
                        options: opts,
                        selector: selector.clone(),
                    });

                    self.active_actions.insert(
                        *action_id,
                        PlayingAction::PlayerDialogue(PlayingPlayerDialogue {
                            player,
                            responder: actor.entity(),
                            state: PlayingPlayerDialogueState::Waiting(selector),
                            done: false,
                        }),
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
    PlayerDialogue(PlayingPlayerDialogue),
}

impl PlayingAction {
    fn update(
        &mut self,
        action: &Action,
        world: &mut World,
        events: &mut EventBus,
        executor: &mut Executor,
        quests: &mut QuestManager,
    ) {
        match (self, action) {
            (Self::Dialogue(_), _) => {}
            (Self::PlayerDialogue(action), Action::PlayerDialogue { actor: _, options }) => {
                match &mut action.state {
                    PlayingPlayerDialogueState::Waiting(selector) => {
                        let Some(index) = selector.get() else {
                            return;
                        };

                        let Some(dialogue) = PlayingDialogue::new(
                            action.player,
                            &options[*index].dialogue,
                            world,
                            events,
                            executor,
                            quests,
                        ) else {
                            action.done = true;
                            return;
                        };

                        action.state = PlayingPlayerDialogueState::PlayDialogue(dialogue, *index);
                    }
                    PlayingPlayerDialogueState::PlayDialogue(state, index) => {
                        if state.sounds.iter().any(|sound| !sound.is_done()) {
                            return;
                        };

                        let Some(dialogue) = PlayingDialogue::new(
                            action.responder,
                            &options[*index].response,
                            world,
                            events,
                            executor,
                            quests,
                        ) else {
                            action.done = true;
                            return;
                        };

                        action.state = PlayingPlayerDialogueState::PlayResponse(dialogue);
                    }
                    PlayingPlayerDialogueState::PlayResponse(state) => {
                        if state.sounds.iter().any(|sound| !sound.is_done()) {
                            return;
                        };

                        action.done = true;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn is_done(&self) -> bool {
        match self {
            Self::Dialogue(action) => action.sounds.iter().all(|sound| sound.is_done()),
            Self::PlayerDialogue(action) => action.done,
        }
    }
}

#[derive(Clone, Debug)]
struct PlayingDialogue {
    sounds: Vec<DialogueSound>,
}

impl PlayingDialogue {
    fn new(
        actor: Entity,
        options: &[DialogueOption],
        world: &mut World,
        events: &mut EventBus,
        executor: &mut Executor,
        quests: &mut QuestManager,
    ) -> Option<Self> {
        let opt = options.iter().find(|opt| {
            opt.condition
                .eval_entity(executor, world, events, quests, actor, None)
        })?;

        if world.get::<&DialogueQueue>(actor).is_err() {
            world.insert_one(actor, DialogueQueue::default()).unwrap();
        }

        let mut queue = world.get::<&mut DialogueQueue>(actor).unwrap();

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

        Some(Self { sounds })
    }
}

#[derive(Clone, Debug)]
struct PlayingPlayerDialogue {
    /// The player entity.
    player: Entity,
    /// The actor that will respond to the player.
    responder: Entity,
    state: PlayingPlayerDialogueState,
    done: bool,
}

#[derive(Clone, Debug)]
enum PlayingPlayerDialogueState {
    Waiting(Arc<OnceLock<usize>>),
    PlayDialogue(PlayingDialogue, usize),
    PlayResponse(PlayingDialogue),
}
