use std::collections::VecDeque;
use std::sync::{Arc, Once};

use crossbeam::queue::SegQueue;
use game_common::collections::vec_map::VecMap;
use game_tasks::{Task, TaskPool};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use hecs::World;
use indexmap::IndexMap;
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{
    QuestAliasFlags, QuestDataFlags, QuestReferenceAlias, QuestStageFlags, SceneActionData,
    SceneActionFlags,
};
use openfo4_esp::{FormId, RecordChildren, RecordData, RecordKind};

use crate::events::quest::{StartQuestScene, StopQuestScene};
use crate::state::esp::EspLoader;
use crate::state::hardcoded::HARDCODED_MODULE_ID;
use crate::state::level::loader::{EntityLoader, EntityRef};
use crate::systems::scripting::invocations::{
    AliasInit, AliasShutdown, Init, QuestFragmentSetStage, QuestInit, QuestSetStage, QuestShutdown,
};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};
use crate::systems::scripting::{Executor, InvocationId};

static ONCE: Once = Once::new();

#[derive(Debug)]
pub struct QuestManager {
    pool: Arc<TaskPool>,
    esp_loader: Arc<EspLoader>,
    script_loader: Arc<ScriptLoader>,
    entity_loader: Arc<EntityLoader>,
    events: Arc<SegQueue<Event>>,
    quests: HashMap<RecordId, Quest>,
    scenes_to_quest: HashMap<RecordId, RecordId>,
}

impl QuestManager {
    pub fn new(
        pool: Arc<TaskPool>,
        esp_loader: &Arc<EspLoader>,
        executor: &mut Executor,
        loader: &Arc<EntityLoader>,
    ) -> Self {
        Self {
            pool,
            quests: HashMap::new(),
            esp_loader: esp_loader.clone(),
            script_loader: executor.loader().clone(),
            entity_loader: loader.clone(),
            events: Arc::default(),
            scenes_to_quest: HashMap::new(),
        }
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus, executor: &mut Executor) {
        let _span = trace_span!("QuestManager::update").entered();

        ONCE.call_once(|| {
            let events = self.events.clone();
            let esp_loader = self.esp_loader.clone();
            let script_loader = self.script_loader.clone();

            self.pool.spawn(async move {
                let id = RecordId::new(HARDCODED_MODULE_ID, FormId(0x1069a9));
                load_quest(&esp_loader, &script_loader, &events, id).await;
            });
        });

        while let Some(event) = self.events.pop() {
            match event {
                Event::Discover(id, quest) => {
                    for scene in quest.scenes.keys() {
                        self.scenes_to_quest.insert(*scene, id);
                    }

                    self.quests.insert(id, quest);
                }
            }
        }

        for event in events.read::<StartQuestScene>() {
            let Some(quest) = self.scenes_to_quest.get(&event.scene) else {
                continue;
            };

            let quest = self.quests.get_mut(quest).unwrap();
            quest
                .actions
                .push_back(QuestAction::StartScene(event.scene));
        }

        for event in events.read::<StopQuestScene>() {
            let Some(quest) = self.scenes_to_quest.get(&event.scene) else {
                continue;
            };

            let quest = self.quests.get_mut(quest).unwrap();
            quest.actions.push_back(QuestAction::StopScene(event.scene));
        }

        for (id, quest) in &mut self.quests {
            loop {
                match &mut quest.state {
                    QuestState::PreInit => {
                        if let Some(script) = &quest.script {
                            let id = executor.invoke(Init {
                                object: script.clone(),
                            });

                            quest.state = QuestState::Init(id);
                        } else {
                            quest.state = QuestState::Stopped;
                        }
                    }
                    QuestState::Init(invocation) => {
                        if !executor.is_invocation_done(*invocation) {
                            break;
                        }

                        quest.state = QuestState::Stopped;
                    }
                    QuestState::Stopped => {
                        if quest
                            .actions
                            .pop_front_if(|action| *action == QuestAction::Start)
                            .is_none()
                        {
                            break;
                        }

                        tracing::debug!("starting quest {:?}", id);

                        let id = *id;
                        let esp_loader = self.esp_loader.clone();
                        let entity_loader = self.entity_loader.clone();

                        // Let's load some aliases!
                        let task = self.pool.spawn(async move {
                            let quest = esp_loader
                                .lookup(Some(RecordKind::Quest), id)
                                .await
                                .unwrap();
                            let RecordData::Quest(data) = &quest.local.data else {
                                unreachable!();
                            };

                            load_aliases(&esp_loader, id, data, &entity_loader).await
                        });

                        quest.state = QuestState::Starting(QuestStarting::Load(task));
                    }
                    QuestState::Starting(QuestStarting::Load(task)) => {
                        let Some(aliases) = task.get_output() else {
                            break;
                        };

                        for (id, entity) in aliases {
                            quest.aliases.entry(id).or_default().entity = Some(entity);
                        }

                        let mut invocations = Vec::new();
                        for alias in quest.aliases.values() {
                            if let Some(script) = &alias.script {
                                let id = executor.invoke(AliasInit {
                                    object: script.clone(),
                                });
                                invocations.push(id);
                            }
                        }

                        quest.state = QuestState::Starting(QuestStarting::AliasInit(invocations));
                    }
                    QuestState::Starting(QuestStarting::AliasInit(invocations)) => {
                        invocations.retain(|id| !executor.is_invocation_done(*id));
                        if !invocations.is_empty() {
                            break;
                        }

                        if let Some(script) = &quest.script {
                            let id = executor.invoke(QuestInit {
                                quest: script.clone(),
                            });

                            quest.state = QuestState::Starting(QuestStarting::QuestInit(id));
                        } else {
                            quest.state = QuestState::Running;
                        }
                    }
                    QuestState::Starting(QuestStarting::QuestInit(id)) => {
                        if !executor.is_invocation_done(*id) {
                            break;
                        }

                        quest.state = QuestState::Running;
                    }
                    QuestState::Running => {
                        let Some(action) = quest.actions.pop_front() else {
                            break;
                        };

                        debug_assert_ne!(
                            action,
                            QuestAction::Start,
                            "received QuestAction::Start on quest that is already running"
                        );

                        match action {
                            QuestAction::Start => {}
                            QuestAction::Stop => {
                                tracing::debug!("stopping quest {:?}", id);

                                if let Some(script) = &quest.script {
                                    let id = executor.invoke(QuestShutdown {
                                        object: script.clone(),
                                    });

                                    quest.state =
                                        QuestState::Stopping(QuestStopping::QuestShutdown(id));
                                } else {
                                    let mut invocations = Vec::new();
                                    for alias in quest.aliases.values() {
                                        if let Some(script) = &alias.script {
                                            let id = executor.invoke(AliasShutdown {
                                                object: script.clone(),
                                            });
                                            invocations.push(id);
                                        }
                                    }

                                    quest.state = QuestState::Stopping(
                                        QuestStopping::AliasShutdown(invocations),
                                    );
                                }
                            }
                            QuestAction::SetStage(stage_id) => {
                                let Some(stage) = quest.stages.get_mut(&stage_id) else {
                                    continue;
                                };

                                if stage.is_done {
                                    continue;
                                }
                                stage.is_done = true;

                                tracing::debug!(
                                    "advancing quest {:?} from stage {:?} to {}",
                                    stage_id,
                                    quest.active_stage,
                                    stage_id
                                );

                                if let Some(script) = &quest.script {
                                    executor.invoke(QuestSetStage {
                                        quest: script.clone(),
                                        stage: stage_id,
                                        item: 0,
                                    });

                                    for name in &stage.fragment_functions {
                                        executor.invoke(QuestFragmentSetStage {
                                            quest: script.clone(),
                                            func_name: name.clone(),
                                        });
                                    }
                                }
                            }
                            QuestAction::StartScene(scene) => {
                                tracing::debug!("starting scene {:?} of quest {:?}", scene, id);
                            }
                            QuestAction::StopScene(scene) => {
                                tracing::debug!("stopping scene {:?} of quest {:?}", scene, id);
                            }
                        }
                    }
                    QuestState::Stopping(QuestStopping::QuestShutdown(id)) => {
                        if !executor.is_invocation_done(*id) {
                            break;
                        }

                        let mut invocations = Vec::new();
                        for alias in quest.aliases.values() {
                            if let Some(script) = &alias.script {
                                let id = executor.invoke(AliasShutdown {
                                    object: script.clone(),
                                });
                                invocations.push(id);
                            }
                        }

                        quest.state =
                            QuestState::Stopping(QuestStopping::AliasShutdown(invocations));
                    }
                    QuestState::Stopping(QuestStopping::AliasShutdown(invocations)) => {
                        invocations.retain(|id| !executor.is_invocation_done(*id));
                        if !invocations.is_empty() {
                            break;
                        }

                        quest.state = QuestState::Stopped;
                    }
                }
            }
        }
    }

    pub fn get_scene(&self, scene: RecordId) -> Option<&SceneState> {
        let quest = self.scenes_to_quest.get(&scene)?;
        let quest = self.quests.get(quest)?;
        quest.scenes.get(&scene)
    }

    pub fn get(&self, quest: RecordId) -> Option<&Quest> {
        self.quests.get(&quest)
    }

    pub fn start_quest(&mut self, quest: RecordId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.actions.push_back(QuestAction::Start);
    }

    pub fn stop_quest(&mut self, quest: RecordId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.actions.push_back(QuestAction::Stop);
    }

    pub fn set_quest_stage(&mut self, quest: RecordId, id: StageId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.actions.push_back(QuestAction::SetStage(id));
    }
}

#[derive(Debug)]
pub struct Quest {
    pub state: QuestState,
    pub stages: IndexMap<StageId, QuestStage>,
    pub active_stage: StageId,
    script: Option<ScriptObject>,
    aliases: HashMap<u32, AliasState>,
    actions: VecDeque<QuestAction>,
    scenes: HashMap<RecordId, SceneState>,
}

#[derive(Debug)]
pub enum QuestState {
    /// The quest is loaded, but `OnInit` was not called yet.
    PreInit,
    /// `OnInit` is being called on the quest.
    Init(InvocationId),
    Starting(QuestStarting),
    /// The quest is currently active.
    Running,
    Stopping(QuestStopping),
    Stopped,
}

#[derive(Debug)]
pub enum QuestStarting {
    /// The quest is loading some data before being started.
    Load(Task<HashMap<u32, EntityRef>>),
    /// `OnAliasInit` is being called on all aliases.
    AliasInit(Vec<InvocationId>),
    /// `OnQuestInit` is being called on the quest.
    QuestInit(InvocationId),
}

#[derive(Debug)]
pub enum QuestStopping {
    /// `OnQuest` is being called on the quest.
    QuestShutdown(InvocationId),
    /// `OnQuestShutdown` is being called on all aliases.
    AliasShutdown(Vec<InvocationId>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum QuestAction {
    Start,
    Stop,
    SetStage(StageId),
    StartScene(RecordId),
    StopScene(RecordId),
}

#[derive(Debug)]
enum Event {
    Discover(RecordId, Quest),
}

pub type StageId = u16;

#[derive(Clone, Debug)]
pub struct QuestStage {
    pub run_on_start: bool,
    pub run_on_stop: bool,
    pub is_done: bool,
    pub fragment_functions: HashSet<String>,
}

async fn load_quest(
    esp_loader: &EspLoader,
    script_loader: &ScriptLoader,
    events: &SegQueue<Event>,
    id: RecordId,
) {
    let quest = esp_loader
        .lookup(Some(RecordKind::Quest), id)
        .await
        .unwrap();

    let RecordData::Quest(data) = &quest.local.data else {
        todo!();
    };

    let mut script = script_loader.load(quest.id).await;

    let mut stages = IndexMap::new();
    for stage in &*data.stages {
        stages.insert(
            stage.index.stage_index,
            QuestStage {
                run_on_start: stage.index.flags.contains(QuestStageFlags::RUN_ON_START),
                run_on_stop: stage.index.flags.contains(QuestStageFlags::RUN_ON_STOP),
                is_done: false,
                fragment_functions: HashSet::new(),
            },
        );
    }

    if let Some(vmad) = &data.virtual_machine_adapter
        && let Some(fragments) = &vmad.fragments
    {
        for stage in &fragments.fragments {
            let Some(s) = stages.get_mut(&stage.quest_stage) else {
                continue;
            };

            // FIXME: This will call ALL scripts with the name,
            // however we only want to call the fragment script.
            s.fragment_functions.insert(stage.fragment_name.clone());
        }
    }

    let mut aliases = HashMap::new();
    if let Some(script) = &mut script {
        for (key, script) in script.aliases.drain() {
            // TODO: Figure out what happens when we touch aliases from
            // other quests.
            assert_eq!(key.accessor, quest.id);
            assert_eq!(key.owner, quest.id);

            aliases.insert(
                key.id,
                AliasState {
                    entity: None,
                    script: Some(script),
                },
            );
        }
    }

    let mut actions = VecDeque::new();
    if data.data.flags.contains(QuestDataFlags::START_GAME_ENABLED) {
        actions.push_back(QuestAction::Start);
    }

    let scenes = load_scenes(esp_loader, script_loader, id).await;

    events.push(Event::Discover(
        quest.id,
        Quest {
            state: QuestState::PreInit,
            stages,
            active_stage: 0,
            script: script.map(|v| v.object),
            aliases: HashMap::new(),
            actions,
            scenes,
        },
    ));
}

#[tracing::instrument(skip_all)]
async fn load_aliases(
    esp_loader: &EspLoader,
    quest_id: RecordId,
    quest: &openfo4_esp::records::Quest,
    entity_loader: &EntityLoader,
) -> HashMap<u32, EntityRef> {
    let mut aliases = HashMap::new();

    for alias in &quest.aliases.references {
        let entity = load_reference_alias(esp_loader, entity_loader, quest_id, quest, alias)
            .await
            .unwrap();

        if !alias.flags.contains(QuestAliasFlags::OPTIONAL) && entity.is_none() {
            todo!("alias was not optional, but didn't resolve");
        }

        if let Some(entity) = entity {
            tracing::debug!(
                "alias {} for quest {:?} bound to entity {:?}",
                alias.id,
                quest_id,
                entity.entity()
            );

            aliases.insert(alias.id, entity);
        }
    }

    for _alias in &quest.aliases.locations {
        todo!("TODO: location alias");
    }

    for _alias in &quest.aliases.ref_collections {
        todo!("TODO: ref coll alias");
    }

    aliases
}

async fn load_reference_alias(
    esp_loader: &EspLoader,
    loader: &EntityLoader,
    quest_id: RecordId,
    quest_data: &openfo4_esp::records::Quest,
    alias: &QuestReferenceAlias,
) -> Result<Option<EntityRef>, AliasLoadError> {
    if let Some(form) = alias.forced_reference {
        let id = esp_loader.resolve_form(form, quest_id).unwrap();
        let entity = loader.load(id).await.unwrap();
        return Ok(Some(entity));
    }

    // A unique actor is a reference to a NPC record.
    // In this case the Quest record must have a Location and that
    // location links the NPC record to a PlacedActor record.
    if let Some(npc_form) = alias.unique_actor {
        let npc_id = esp_loader.resolve_form(npc_form, quest_id).unwrap();

        let Some(location) = quest_data.location else {
            return Err(AliasLoadError::NoLocation);
        };

        let location_id = esp_loader.resolve_form(location, quest_id).unwrap();
        let RecordData::Location(location) = esp_loader
            .lookup(Some(RecordKind::Location), location_id)
            .await
            .unwrap()
            .local
            .data
        else {
            unreachable!();
        };

        for entry in location.unique_npcs.iter().flatten() {
            let entry_npc = esp_loader.resolve_form(entry.actor, location_id).unwrap();
            let entry_ref = esp_loader
                .resolve_form(entry.reference, location_id)
                .unwrap();

            if entry_npc == npc_id {
                let entity = loader.load(entry_ref).await.unwrap();
                return Ok(Some(entity));
            }
        }

        todo!();
    }

    todo!("TODO: {:?}", alias)
}

async fn load_scenes(
    esp_loader: &EspLoader,
    script_loader: &ScriptLoader,
    quest: RecordId,
) -> HashMap<RecordId, SceneState> {
    let records = esp_loader
        .children(RecordChildren::Scenes, quest)
        .await
        .unwrap();

    let mut scenes = HashMap::new();
    for record in records {
        let RecordData::Scene(data) = record.local.data else {
            continue;
        };

        let mut phases = VecMap::new();
        for (index, phase) in data.phases.iter().enumerate() {
            phases.insert(index as u32, Phase::default());
        }

        for action in &*data.actions {
            let Some(phase) = phases.get_mut(action.start_phase) else {
                continue;
            };

            let face_target = action
                .flags
                .is_some_and(|flags| flags.contains(SceneActionFlags::FACE_TARGET));

            match &action.data {
                SceneActionData::Dialogue(data) => {
                    // FIXME: What does facetracking multiple targets mean?
                    if let Some(targets) = &data.player_headtracking
                        && let Some(target) = targets.first()
                        && face_target
                    {
                        phase.actions.push(Action::FaceTarget {
                            actor: action.actor as u32,
                            target: *target,
                        });
                    }

                    let topic_id = esp_loader.resolve_form(data.topic, record.id).unwrap();
                    let infos = esp_loader
                        .children(RecordChildren::DialogueInfos, topic_id)
                        .await
                        .unwrap();

                    let mut options = Vec::new();
                    for info in infos {
                        let RecordData::DialogueInfo(data) = info.local.data else {
                            continue;
                        };

                        options.push(Dialogue {});
                    }

                    phase.actions.push(Action::PlayDialogue {
                        actor: action.actor as u32,
                        dialogue: PlayDialogueAction { options },
                    });
                }
                _ => {}
            }
        }

        scenes.insert(record.id, SceneState { quest, phases });
    }

    scenes
}

#[derive(Clone, Debug)]
enum AliasLoadError {
    NoLocation,
}

#[derive(Clone, Debug, Default)]
struct AliasState {
    entity: Option<EntityRef>,
    script: Option<ScriptObject>,
}

#[derive(Clone, Debug)]
pub struct SceneState {
    pub quest: RecordId,
    phases: VecMap<u32, Phase>,
}

#[derive(Clone, Debug, Default)]
struct Phase {
    actions: Vec<Action>,
}

#[derive(Clone, Debug)]
enum Action {
    /// Make the `actor` face the given `target`.
    FaceTarget { actor: u32, target: u32 },
    PlayDialogue {
        actor: u32,
        dialogue: PlayDialogueAction,
    },
}

#[derive(Clone, Debug)]
struct PlayDialogueAction {
    options: Vec<Dialogue>,
}

#[derive(Clone, Debug)]
struct Dialogue {}
