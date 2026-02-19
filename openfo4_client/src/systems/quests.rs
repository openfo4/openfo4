use std::sync::{Arc, Once};

use crossbeam::queue::SegQueue;
use game_tasks::{Task, TaskPool};
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
    Init, QuestFragmentSetStage, QuestInit, QuestSetStage,
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
        ONCE.call_once(|| {
            let events = self.events.clone();
            let esp_loader = self.esp_loader.clone();
            let loader = self.entity_loader.clone();
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

                let script = script_loader.load(quest.id).await;

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

                let aliases = load_aliases(&esp_loader, quest.id, data, &loader).await;
                dbg!(&aliases);
                todo!();

                events.push(Event::Discover(
                    quest.id,
                    Quest {
                        state: QuestState::Stopped,
                        is_init: false,
                        stages,
                        active_stage: None,
                        next_stage: None,
                        script: script.map(|v| v.object),
                        start_now: data.data.flags.contains(QuestDataFlags::START_GAME_ENABLED),
                        aliases: HashMap::new(),
                    },
                ));
            });
        });

        for (id, quest) in &mut self.quests {
            let Some(script) = &quest.script else {
                continue;
            };

            if !quest.is_init {
                quest.is_init = true;
                tracing::debug!("init quest {:?}", id);

                executor.invoke(Init {
                    object: script.clone(),
                });
            }

            if quest.start_now {
                quest.start_now = false;
                tracing::debug!("starting quest {:?}", id);

                executor.invoke(QuestInit {
                    quest: script.clone(),
                });

                quest.active_stage = Some(0);
                quest.next_stage = Some(0);
            }

            if let Some(stage_id) = quest.next_stage.take() {
                tracing::debug!(
                    "advancing quest {:?} from stage {:?} to {}",
                    id,
                    quest.active_stage,
                    stage_id
                );

                if let Some(prev_id) = quest.active_stage
                    && let Some(stage) = quest.stages.get_mut(&prev_id)
                {
                    stage.is_done = true;
                }

                if let Some(stage) = quest.stages.get(&stage_id) {
                    executor.invoke(QuestSetStage {
                        quest: script.clone(),
                        stage: stage_id,
                        // TODO
                        item: 0,
                    });

                    for func_name in &stage.fragment_functions {
                        executor.invoke(QuestFragmentSetStage {
                            quest: script.clone(),
                            func_name: func_name.clone(),
                        });
                    }

                    quest.active_stage = Some(stage_id);
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

        // if quest.state != QuestState::Stopped {
        //     return;
        // }

        // quest.state = QuestState::Starting;
        quest.is_init = false;
    }

    pub fn stop_quest(&mut self, quest: RecordId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        // quest.state = QuestState::Stopping;
        quest.is_init = false;
    }

    pub fn set_quest_stage(&mut self, quest: RecordId, id: StageId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.next_stage = Some(id);
    }
}

#[derive(Debug)]
pub struct Quest {
    pub state: QuestState,
    pub is_init: bool,
    pub start_now: bool,
    pub stages: IndexMap<StageId, QuestStage>,
    pub active_stage: Option<StageId>,
    pub next_stage: Option<StageId>,
    pub script: Option<ScriptObject>,
    pub aliases: HashMap<u32, EntityRef>,
}

#[derive(Debug)]
pub enum QuestState {
    /// The quest is loaded, but `OnInit` was not called yet.
    PreInit(Option<InvocationId>),
    Init,
    Starting(QuestStarting),
    /// The quest is currently active.
    Running,
    Stopping(QuestStopping),
    Stopped,
}

#[derive(Debug)]
pub enum QuestStarting {
    /// The quest is loading some data before being started.
    Load(Task<()>),
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
