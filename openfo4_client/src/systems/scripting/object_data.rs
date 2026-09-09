use hashbrown::HashSet;
use openfo4_core::record::RecordId;
use openfo4_esp::RecordData;
use openfo4_esp::records::NpcBaseFlags;

use crate::state::esp::{EspLoader, GlobalRecord};

#[derive(Clone, Debug)]
pub enum ObjectData {
    None,
    InputEnableLayer { layer: usize },
    ActorBase(ActorBase),
}

impl ObjectData {
    pub fn form(&self) -> Option<&Form> {
        match self {
            Self::ActorBase(data) => Some(&data.form),
            _ => None,
        }
    }

    pub fn actor_base(&self) -> Option<&ActorBase> {
        match self {
            Self::ActorBase(data) => Some(data),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Form {
    pub value: i32,
    pub keywords: HashSet<RecordId>,
}

#[derive(Clone, Debug)]
pub struct ActorBase {
    pub form: Form,
    pub class: Option<RecordId>,
    pub race: RecordId,
    pub is_unique: bool,
    pub is_essential: bool,
    pub is_protected: bool,
    pub is_invulnerable: bool,
}

pub fn load_record_object_data(esp_loader: &EspLoader, record: &GlobalRecord) -> ObjectData {
    match &record.local.data {
        RecordData::Npc(data) => ObjectData::ActorBase(ActorBase {
            form: Form {
                keywords: data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                    .collect(),
                value: 0,
            },
            class: data
                .class
                .map(|id| esp_loader.resolve_form(id, record.id).unwrap()),
            race: esp_loader.resolve_form(data.race, record.id).unwrap(),
            is_unique: data.base_stats.flags.contains(NpcBaseFlags::UNIQUE),
            is_essential: data.base_stats.flags.contains(NpcBaseFlags::ESSENTIAL),
            is_protected: data.base_stats.flags.contains(NpcBaseFlags::PROTECTED),
            is_invulnerable: data.base_stats.flags.contains(NpcBaseFlags::INVULNERABLE),
        }),
        _ => ObjectData::None,
    }
}
