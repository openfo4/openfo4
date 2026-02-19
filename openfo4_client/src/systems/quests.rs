use std::collections::VecDeque;
use std::sync::{Arc, Once};

use crossbeam::queue::SegQueue;
use game_tasks::{Task, TaskPool};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use hecs::World;
use indexmap::IndexMap;
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{QuestAliasFlags, QuestDataFlags, QuestReferenceAlias, QuestStageFlags};
use openfo4_esp::{FormId, RecordData, RecordKind};

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
    quests: HashMap<RecordId, Quest>,
    events: Arc<SegQueue<Event>>,
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
        }
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus, executor: &mut Executor) {
        let _span = trace_span!("QuestManager::update").entered();

        ONCE.call_once(|| {
            let events = self.events.clone();
            let esp_loader = self.esp_loader.clone();
            let script_loader = self.script_loader.clone();

            self.pool.spawn(async move {
                let quests = esp_loader
                    .lookup_all(Some(RecordKind::Quest))
                    .await
                    .unwrap();

                // let quest = quests
                //     .iter()
                //     .find(|v| v.id == RecordId::new(HARDCODED_MODULE_ID, FormId(0x80e)))
                //     .unwrap();
                let quest = quests
                    .iter()
                    .find(|v| v.id == RecordId::new(HARDCODED_MODULE_ID, FormId(0x1069a9)))
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

                events.push(Event::Discover(
                    quest.id,
                    Quest {
                        state: QuestState::PreInit,
                        stages,
                        active_stage: 0,
                        script: script.map(|v| v.object),
                        aliases: HashMap::new(),
                        actions,
                    },
                ));
            });
        });

        while let Some(event) = self.events.pop() {
            match event {
                Event::Discover(id, quest) => {
                    self.quests.insert(id, quest);
                }
            }
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
                            QuestAction::SetStage(id) => {
                                tracing::debug!(
                                    "advancing quest {:?} from stage {:?} to {}",
                                    id,
                                    quest.active_stage,
                                    id
                                );

                                if let Some(script) = &quest.script {
                                    executor.invoke(QuestSetStage {
                                        quest: script.clone(),
                                        stage: id,
                                        item: 0,
                                    });

                                    if let Some(stage) = quest.stages.get(&id) {
                                        for name in &stage.fragment_functions {
                                            executor.invoke(QuestFragmentSetStage {
                                                quest: script.clone(),
                                                func_name: name.clone(),
                                            });
                                        }
                                    }
                                }
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

#[derive(Clone, Debug)]
enum AliasLoadError {
    NoLocation,
}

#[derive(Clone, Debug, Default)]
struct AliasState {
    entity: Option<EntityRef>,
    script: Option<ScriptObject>,
}
