//! Hardcoded resources

use std::num::NonZeroU32;

use game_common::components::actor;
use hashbrown::HashMap;
use openfo4_core::record::{ModuleId, RecordId};
use openfo4_esp::records::ActorValueInfo;
use openfo4_esp::{FormId, Record, RecordData, RecordHeaderFlags, ZString};

pub const HARDCODED_MODULE_ID: ModuleId = ModuleId(NonZeroU32::MIN);

pub const DAMAGE_TYPE_PHYSICAL: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId(0x00060A87));
pub const KEYWORD_FEMALE: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId(0x00084486));

pub const ACTOR_VALUE_STRENGTH: FormId = FormId(0x2c2);
pub const ACTOR_VALUE_PERCEPTION: FormId = FormId(0x2c3);
pub const ACTOR_VALUE_ENDURANCE: FormId = FormId(0x2c4);
pub const ACTOR_VALUE_CHARISMA: FormId = FormId(0x2c5);
pub const ACTOR_VALUE_INTELLIGENCE: FormId = FormId(0x2c6);
pub const ACTOR_VALUE_AGILITY: FormId = FormId(0x2c7);
pub const ACTOR_VALUE_LUCK: FormId = FormId(0x2c8);
pub const ACTOR_VALUE_HEALTH: FormId = FormId(0x2d4);

pub const PATH_SKY_ATMOSPHERE: &str = "meshes/sky/atmosphere.nif";

pub const ACTOR_VALUE_FATIGUE: FormId = FormId(0x34f);
pub const ACTOR_VALUE_POWER_ARMOR_HEAD_CONDITION: FormId = FormId(0x2ef);
pub const ACTOR_VALUE_POWER_ARMOR_TORSO_CONDITION: FormId = FormId(0x2f0);
pub const ACTOR_VALUE_POWER_ARMOR_LEFT_ARM_CONDITION: FormId = FormId(0x2f1);
pub const ACTOR_VALUE_POWER_ARMOR_RIGHT_ARM_CONDITION: FormId = FormId(0x35d);
pub const ACTOR_VALUE_POWER_ARMOR_LEFT_LEG_CONDITION: FormId = FormId(0x35e);
pub const ACTOR_VALUE_POWER_ARMOR_RIGHT_LEG_CONDITION: FormId = FormId(0x388);
pub const ACTOR_VALUE_POWER_ARMOR_BATTERY: FormId = FormId(0x35c);

pub const ACTOR_VALUE_PARALYSIS: FormId = FormId(0x2f2);

pub const ACTOR_VALUE_WAITING_FOR_PLAYER: FormId = FormId(0x31c);

pub const ACTOR_VALUE_WORKSHOP_ID: FormId = FormId(0x2d1);
pub const ACTOR_VALUE_WORKSHOP_ACTOR_WOUNDED: FormId = FormId(0x33b);
pub const ACTOR_VALUE_WORKSHOP_PLAYER_OWNED: FormId = FormId(0x33c);

pub const ACTOR_VALUE_WORKSHOP_CURRENT_DRAWS: FormId = FormId(0x34a);
pub const ACTOR_VALUE_WORKSHOP_MAX_DRAWS: FormId = FormId(0x34b);
pub const ACTOR_VALUE_WORKSHOP_MAX_TRIANGLES: FormId = FormId(0x349);
pub const ACTOR_VALUE_WORKSHOP_CURRENT_TRIANGLES: FormId = FormId(0x348);

// Workshop resources?
pub const ACTOR_VALUE_FOOD: FormId = FormId(0x331);
pub const ACTOR_VALUE_WATER: FormId = FormId(0x332);
pub const ACTOR_VALUE_BED: FormId = FormId(0x334);
pub const ACTOR_VALUE_HAPPINESS: FormId = FormId(0x335);
pub const ACTOR_VALUE_ARTILLERY: FormId = FormId(0x336);

pub const ACTOR_VALUE_POWER_GENERATED: FormId = FormId(0x32e);
pub const ACTOR_VALUE_POWER_RADIATION: FormId = FormId(0x32f);
pub const ACTOR_VALUE_POWER_REQUIRED: FormId = FormId(0x330);

pub fn load_hardcoded_forms() -> HashMap<FormId, Record> {
    fn actor_value(id: FormId, name: &str) -> (FormId, Record) {
        (
            id,
            Record {
                id,
                flags: RecordHeaderFlags::empty(),
                data: RecordData::ActorValueInfo(ActorValueInfo {
                    id: Some(ZString::new(name.as_bytes().to_vec())),
                }),
            },
        )
    }

    [
        actor_value(ACTOR_VALUE_WORKSHOP_PLAYER_OWNED, "WorkshopPlayerOwned"),
        actor_value(ACTOR_VALUE_FATIGUE, "Fatigue"),
        actor_value(ACTOR_VALUE_BED, "Bed"),
        actor_value(
            ACTOR_VALUE_POWER_ARMOR_HEAD_CONDITION,
            "PowerArmorHeadCondition",
        ),
        actor_value(
            ACTOR_VALUE_POWER_ARMOR_TORSO_CONDITION,
            "PowerArmorTorsoCondition",
        ),
        actor_value(
            ACTOR_VALUE_POWER_ARMOR_RIGHT_LEG_CONDITION,
            "PowerArmorRightLegCondition",
        ),
        actor_value(
            ACTOR_VALUE_POWER_ARMOR_LEFT_LEG_CONDITION,
            "PowerArmorLeftLegCondition",
        ),
        actor_value(
            ACTOR_VALUE_POWER_ARMOR_LEFT_ARM_CONDITION,
            "PowerArmorLeftArmCondition",
        ),
        actor_value(
            ACTOR_VALUE_POWER_ARMOR_RIGHT_ARM_CONDITION,
            "PowerArmorRightArmCondition",
        ),
        actor_value(ACTOR_VALUE_POWER_ARMOR_BATTERY, "PowerArmorBattery"),
        actor_value(ACTOR_VALUE_PARALYSIS, "Paralysis"),
        actor_value(ACTOR_VALUE_WAITING_FOR_PLAYER, "WaitingForPlayer"),
        actor_value(ACTOR_VALUE_WORKSHOP_CURRENT_DRAWS, "WorkshopCurrentDraws"),
        actor_value(ACTOR_VALUE_WORKSHOP_MAX_TRIANGLES, "WorkshopMaxTriangles"),
        actor_value(ACTOR_VALUE_POWER_GENERATED, "PowerGenerated"),
        actor_value(ACTOR_VALUE_FOOD, "Food"),
        actor_value(ACTOR_VALUE_WATER, "Water"),
        actor_value(ACTOR_VALUE_BED, "Bed"),
        actor_value(ACTOR_VALUE_HAPPINESS, "Happiness"),
        actor_value(ACTOR_VALUE_ARTILLERY, "Artillery"),
        actor_value(ACTOR_VALUE_WORKSHOP_ACTOR_WOUNDED, "WorkshopActorWounded"),
        actor_value(ACTOR_VALUE_WORKSHOP_ID, "WorkshopID"),
        actor_value(
            ACTOR_VALUE_WORKSHOP_CURRENT_TRIANGLES,
            "WorkshopCurrentTriangles",
        ),
        actor_value(ACTOR_VALUE_POWER_RADIATION, "PowerRadiation"),
        actor_value(ACTOR_VALUE_POWER_REQUIRED, "PowerRequired"),
        actor_value(ACTOR_VALUE_WORKSHOP_MAX_DRAWS, "WorkshopMaxDraws"),
    ]
    .into_iter()
    .collect()
}
