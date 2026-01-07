use hashbrown::HashMap;
use hecs::World;
use indexmap::IndexMap;
use openfo4_core::events::EventBus;
use openfo4_esp::records::QuestStageFlags;
use openfo4_esp::{FormId, RecordData, RecordKind};

use crate::state::esp::EspLoader;
use crate::systems::scripting::{Executor, Invocation};

#[derive(Clone, Debug)]
pub struct QuestManager {
    quests: HashMap<FormId, Quest>,
}

impl QuestManager {
    pub fn new(esp_loader: &EspLoader, executor: &mut Executor) -> Self {
        let quests =
            futures_lite::future::block_on(esp_loader.lookup_all(Some(RecordKind::Quest))).unwrap();

        let quest = quests.iter().find(|v| v.id == FormId(0x89c)).unwrap();

        let RecordData::Quest(data) = &quest.data else {
            todo!();
        };

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
            },
        );

        Self { quests }
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus, executor: &mut Executor) {
        for (id, quest) in &mut self.quests {
            if !quest.is_init {
                executor.invoke(Invocation::QuestInit { quest: *id });
                executor.invoke(Invocation::QuestSetStage {
                    quest: *id,
                    stage: 0,
                    item: 0,
                });
                // executor.invoke(Invocation::QuestSetStage {
                //     quest: (),
                //     stage: (),
                //     item: (),
                // });
            }
        }
    }

    pub fn start_quest(&mut self, quest: FormId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        if quest.state != QuestState::Stopped {
            return;
        }

        quest.state = QuestState::Starting;
        quest.is_init = false;
    }

    pub fn stop_quest(&mut self, quest: FormId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.state = QuestState::Stopping;
        quest.is_init = false;
    }

    pub fn set_quest_stage(&mut self, quest: FormId, id: StageId) {
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
