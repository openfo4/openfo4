//! Hardcoded resources

use hashbrown::HashMap;
use openfo4_esp::records::ActorValueInfo;
use openfo4_esp::{FormId, Record, RecordData, RecordHeaderFlags, ZString};

pub const DAMAGE_TYPE_PHYSICAL: FormId = FormId(0x00060A87);
pub const KEYWORD_FEMALE: FormId = FormId(0x00084486);

pub const ACTOR_VALUE_STRENGTH: FormId = FormId(0x2c2);
pub const ACTOR_VALUE_PERCEPTION: FormId = FormId(0x2c3);
pub const ACTOR_VALUE_ENDURANCE: FormId = FormId(0x2c4);
pub const ACTOR_VALUE_CHARISMA: FormId = FormId(0x2c5);
pub const ACTOR_VALUE_INTELLIGENCE: FormId = FormId(0x2c6);
pub const ACTOR_VALUE_AGILITY: FormId = FormId(0x2c7);
pub const ACTOR_VALUE_LUCK: FormId = FormId(0x2c8);
pub const ACTOR_VALUE_HEALTH: FormId = FormId(0x2d4);

pub const PATH_SKY_ATMOSPHERE: &str = "meshes/sky/atmosphere.nif";

pub fn load_hardcoded_forms() -> HashMap<FormId, Record> {
    let mut forms = HashMap::new();

    forms.insert(
        FormId(0x33c),
        Record {
            id: FormId(0x33c),
            flags: RecordHeaderFlags::empty(),
            data: RecordData::ActorValueInfo(ActorValueInfo {
                id: Some(ZString::new(b"WorkshopPlayerOwned".to_vec())),
            }),
        },
    );

    forms
}
