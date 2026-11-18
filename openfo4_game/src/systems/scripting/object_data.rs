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
    Item(Item),
}

impl ObjectData {
    pub fn form(&self) -> Option<&Form> {
        match self {
            Self::ActorBase(data) => Some(&data.form),
            Self::Item(data) => Some(&data.form),
            _ => None,
        }
    }

    pub fn actor_base(&self) -> Option<&ActorBase> {
        match self {
            Self::ActorBase(data) => Some(data),
            _ => None,
        }
    }

    pub fn keywords(&self) -> Option<&HashSet<RecordId>> {
        match self {
            Self::Item(item) => Some(&item.form.keywords),
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

#[derive(Clone, Debug)]
pub struct Item {
    pub form: Form,
    pub weight: f32,
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

        RecordData::Armor(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.value,
                    keywords,
                },
                weight: data.data.weight,
            })
        }
        RecordData::Weapon(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.value.cast_signed(),
                    keywords,
                },
                weight: data.data.weight,
            })
        }
        RecordData::Ammo(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.value.cast_signed(),
                    keywords,
                },
                weight: data.data.weight,
            })
        }
        RecordData::Book(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.unwrap_or_default().value.cast_signed(),
                    keywords,
                },
                weight: data.data.unwrap_or_default().weight,
            })
        }
        RecordData::Ingestible(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.effect_data.value,
                    keywords,
                },
                weight: data.weight,
            })
        }
        RecordData::Ingredient(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.value.cast_signed(),
                    keywords,
                },
                weight: data.data.weight,
            })
        }
        RecordData::Key(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.value.cast_signed(),
                    keywords,
                },
                weight: data.data.weight,
            })
        }
        RecordData::MiscObject(data) => {
            let keywords = data
                .keywords
                .iter()
                .flatten()
                .map(|id| esp_loader.resolve_form(*id, record.id).unwrap())
                .collect();

            ObjectData::Item(Item {
                form: Form {
                    value: data.data.value.cast_signed(),
                    keywords,
                },
                weight: data.data.weight,
            })
        }
        RecordData::Note(data) => ObjectData::Item(Item {
            form: Form {
                value: data.data.value.cast_signed(),
                keywords: HashSet::new(),
            },
            weight: data.data.weight,
        }),
        _ => ObjectData::None,
    }
}
