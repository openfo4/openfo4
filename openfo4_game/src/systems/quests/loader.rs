use std::collections::VecDeque;
use std::sync::Arc;

use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{QuestDataFlags, QuestStageFlags};
use openfo4_esp::{RecordChildren, RecordData, RecordKind};
use thiserror::Error;

use crate::state::esp::{self, EspLoader};
use crate::systems::conditions::ConditionCompiler;
use crate::systems::quests::scene::{DialogueTopicAction, SceneInfo, SceneState};
use crate::systems::quests::{AliasState, Quest, QuestAction, QuestStage, QuestState};
use crate::systems::scripting::loader::ScriptLoader;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Esp(#[from] esp::Error),
    #[error("record is not a quest")]
    NoQuest,
}

#[derive(Clone, Debug)]
pub struct QuestLoader {
    esp: Arc<EspLoader>,
    conditions: Arc<ConditionCompiler>,
    scripts: Arc<ScriptLoader>,
}

impl QuestLoader {
    pub fn new(
        esp: Arc<EspLoader>,
        conditions: Arc<ConditionCompiler>,
        scripts: Arc<ScriptLoader>,
    ) -> Self {
        Self {
            esp,
            conditions,
            scripts,
        }
    }

    pub async fn load(&self, id: RecordId) -> Result<Quest, Error> {
        let quest = self.esp.lookup(Some(RecordKind::Quest), id).await?;

        let RecordData::Quest(data) = quest.local.data else {
            return Err(Error::NoQuest);
        };

        let mut script = self.scripts.load(quest.id).await;

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

        let scenes = self.load_scenes(id).await?;

        let mut hello_topics = Vec::new();

        for topic in self
            .esp
            .children(RecordChildren::DialogueTopics, id)
            .await?
        {
            let RecordData::DialogueTopic(data) = &topic.local.data else {
                continue;
            };

            match data.subtype_name {
                Some(v) if v.as_bytes() == b"HELO" => {
                    let topic =
                        DialogueTopicAction::load(&self.esp, &self.conditions, topic.id).await?;

                    hello_topics.push(topic);
                }
                _ => continue,
            }
        }

        Ok(Quest {
            state: QuestState::PreInit,
            stages,
            active_stage: 0,
            script: script.map(|v| v.object),
            aliases: HashMap::new(),
            actions,
            scenes,
            hello_topics,
        })
    }

    async fn load_scenes(&self, quest: RecordId) -> Result<HashMap<RecordId, SceneState>, Error> {
        let records = self.esp.children(RecordChildren::Scenes, quest).await?;

        let mut scenes = HashMap::new();
        for record in records {
            let RecordData::Scene(data) = &record.local.data else {
                continue;
            };

            let info = SceneInfo::load(&self.esp, &self.conditions, data, record.id).await?;
            let state = SceneState::new(info);
            scenes.insert(record.id, state);
        }

        Ok(scenes)
    }
}
