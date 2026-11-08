use std::time::Duration;

use openfo4_common::collections::vec_map::VecMap;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{Scene, SceneActionData};
use openfo4_esp::{RecordChildren, RecordData};

use crate::state::esp::{self, EspLoader};
use crate::systems::quests::StageId;

pub struct SceneState {
    phases: VecMap<u32, Phase>,
    set_stage_begin: Option<StageId>,
    set_stage_end: Option<StageId>,
}

impl SceneState {
    pub async fn load(esp: &EspLoader, scene: &Scene, id: RecordId) -> Result<Self, esp::Error> {
        let mut phases = VecMap::new();
        for (index, phase) in scene.phases.iter().enumerate() {
            phases.insert(index as u32, Phase::default());
        }

        for action in &*scene.actions {
            let Some(phase) = phases.get_mut(action.start_phase) else {
                continue;
            };

            // match action.data {
            //     SceneActionData::Dialogue(data) => {
            //         let topic = esp.resolve_form(data.topic, id).unwrap();
            //         let infos = esp.children(RecordChildren::DialogueInfos, topic).await?;
            //         for info in infos {
            //             let RecordData::DialogueInfo(info) = info.local.data else {
            //                 continue;
            //             };

            //             for resp in &*info.responses {
            //                 phase.actions.push(Action::PlayDialogue {
            //                     actor: action.actor as u32,
            //                     dialogue: Dialogue {
            //                         text: "TODO".to_owned(),
            //                         // FIXME: Need to load this from the audio file.
            //                         duration: Duration::ZERO,
            //                     },
            //                 });
            //             }
            //         }
            //     }
            //     SceneActionData::PlayerDialogue(data) => {}
            // }
        }

        todo!()
    }

    pub fn run(&self) {}
}

#[derive(Clone, Debug, Default)]
pub struct Phase {
    actions: Vec<Action>,
}

#[derive(Clone, Debug)]
pub enum Action {
    FaceTarget { actor: u32, target: u32 },
    PlayDialogue { actor: u32, dialogue: Dialogue },
}

#[derive(Clone, Debug)]
pub struct Dialogue {
    pub text: String,
    pub duration: Duration,
}
