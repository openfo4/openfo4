use hashbrown::HashMap;
use hecs::World;
use indexmap::IndexMap;
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::QuestStageFlags;
use openfo4_esp::{FormId, RecordData, RecordKind};

use crate::state::esp::EspLoader;
use crate::state::hardcoded::HARDCODED_MODULE_ID;
use crate::systems::scripting::Executor;
use crate::systems::scripting::invocations::{Init, QuestInit, QuestSetStage};
use crate::systems::scripting::loader::ScriptObject;

#[derive(Clone, Debug)]
pub struct QuestManager {
    quests: HashMap<RecordId, Quest>,
}

impl QuestManager {
    pub fn new(esp_loader: &EspLoader, executor: &mut Executor) -> Self {
        let quests =
            futures_lite::future::block_on(esp_loader.lookup_all(Some(RecordKind::Quest))).unwrap();

        let quest = quests
            .iter()
            .find(|v| v.id == RecordId::new(HARDCODED_MODULE_ID, FormId(0x80e)))
            .unwrap();

        let RecordData::Quest(data) = &quest.local.data else {
            todo!();
        };

        let script = futures_lite::future::block_on(executor.loader().load(quest.id));

        let mut stages = IndexMap::new();
        for stage in &*data.stages {
            stages.insert(
                stage.index.stage_index,
                QuestStage {
                    run_on_start: stage.index.flags.contains(QuestStageFlags::RUN_ON_START),
                    run_on_stop: stage.index.flags.contains(QuestStageFlags::RUN_ON_STOP),
                },
            );
        }

        let mut quests = HashMap::new();
        quests.insert(
            quest.id,
            Quest {
                state: QuestState::Stopping,
                is_init: false,
                stages,
                active_stage: None,
                next_stage: None,
                script,
            },
        );

        Self { quests }
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus, executor: &mut Executor) {
        for (id, quest) in &mut self.quests {
            let Some(script) = &quest.script else {
                continue;
            };

            if !quest.is_init {
                executor.invoke(Init {
                    object: script.clone(),
                });

                executor.invoke(QuestInit {
                    quest: script.clone(),
                });
                executor.invoke(QuestSetStage {
                    quest: script.clone(),
                    stage: 0,
                    item: 0,
                });
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

        if quest.state != QuestState::Stopped {
            return;
        }

        quest.state = QuestState::Starting;
        quest.is_init = false;
    }

    pub fn stop_quest(&mut self, quest: RecordId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.state = QuestState::Stopping;
        quest.is_init = false;
    }

    pub fn set_quest_stage(&mut self, quest: RecordId, id: StageId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.next_stage = Some(id);
    }
}

#[derive(Clone, Debug)]
pub struct Quest {
    pub state: QuestState,
    pub is_init: bool,
    pub stages: IndexMap<StageId, QuestStage>,
    pub active_stage: Option<StageId>,
    pub next_stage: Option<StageId>,
    pub script: Option<ScriptObject>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum QuestState {
    Starting,
    Running,
    Stopping,
    Stopped,
}

pub type StageId = u16;

#[derive(Clone, Debug)]
pub struct QuestStage {
    pub run_on_start: bool,
    pub run_on_stop: bool,
}
