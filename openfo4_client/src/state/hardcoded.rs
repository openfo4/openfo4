//! Hardcoded resources

pub mod fallout4;

use std::num::NonZeroU32;

use hashbrown::HashMap;
use openfo4_core::path::DataPath;
use openfo4_core::record::{ModuleId, RecordId};
use openfo4_esp::records::{ActorValueInfo, PlacedNpc, PositionRotation};
use openfo4_esp::{FormId, Record, RecordData, RecordHeaderFlags, ZString};

pub const HARDCODED_MODULE_ID: ModuleId = ModuleId(NonZeroU32::MIN);

pub const DAMAGE_TYPE_PHYSICAL: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId(0x00060A87));
pub const KEYWORD_FEMALE: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId(0x00084486));

pub const ACTOR_VALUE_AGGRESSION: FormId = FormId(0x2bc);
pub const ACTOR_VALUE_CONFIDENCE: FormId = FormId(0x2bd);
pub const ACTOR_VALUE_ENERGY: FormId = FormId(0x2be);
pub const ACTOR_VALUE_MORALITY: FormId = FormId(0x2bf);
pub const ACTOR_VALUE_ASSISTANCE: FormId = FormId(0x2c1);

pub const PATH_SKY_ATMOSPHERE: &DataPath = DataPath::from_str("meshes/sky/atmosphere.nif");
pub const PATH_SKY_CLOUDS: &DataPath = DataPath::from_str("meshes/sky/clouds.nif");

pub const PATH_LAND_TEXTURE_DEFAULT_DIFFUSE: &DataPath =
    DataPath::from_str("textures/landscape/ground/commonwealthdefault01_d.dds");
pub const PATH_LAND_TEXTURE_DEFAULT_NORMAL: &DataPath =
    DataPath::from_str("textures/landscape/ground/commonwealthdefault01_n.dds");
pub const PATH_LAND_TEXTURE_DEFAULT_SPEC_GLOSS: &DataPath =
    DataPath::from_str("textures/landscape/ground/commonwealthdefault01_s.dds");

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

pub const ACTOR_PLAYER: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId(0x7));

pub const PLACED_ACTOR_PLAYER_REF: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId(0x14));

pub fn load_hardcoded_forms() -> HashMap<RecordId, Record> {
    fn actor_value(id: FormId, name: &str) -> (FormId, Record) {
        (
            id,
            Record {
                id,
                flags: RecordHeaderFlags::empty(),
                data: RecordData::ActorValueInfo(ActorValueInfo {
                    id: Some(ZString::new(name.as_bytes().to_vec())),
                    full_name: None,
                    description: None,
                    abbreviation: None,
                    default_value: None,
                    flags: None,
                    ty: None,
                }),
            },
        )
    }

    [
        actor_value(ACTOR_VALUE_WORKSHOP_PLAYER_OWNED, "WorkshopPlayerOwned"),
        actor_value(ACTOR_VALUE_AGGRESSION, "Aggression"),
        actor_value(ACTOR_VALUE_CONFIDENCE, "Confidence"),
        actor_value(ACTOR_VALUE_ENERGY, "Energy"),
        actor_value(ACTOR_VALUE_MORALITY, "Morality"),
        actor_value(ACTOR_VALUE_ASSISTANCE, "Assistance"),
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
        (
            PLACED_ACTOR_PLAYER_REF.form,
            Record {
                id: PLACED_ACTOR_PLAYER_REF.form,
                flags: RecordHeaderFlags::empty(),
                data: RecordData::PlacedNpc(PlacedNpc {
                    id: Some(ZString::new(b"PlayerRef".to_vec())),
                    virtual_machine_adapter: None,
                    base_npc: ACTOR_PLAYER.form,
                    encounter_zone: None,
                    level_modifier: None,
                    count: None,
                    radius: None,
                    health: None,
                    linked_references: Vec::new(),
                    activate_parent: Vec::new(),
                    activation_parent_flags: None,
                    position: PositionRotation {
                        position_x: 0.0,
                        position_y: 0.0,
                        position_z: 0.0,
                        rotation_x: 0.0,
                        rotation_y: 0.0,
                        rotation_z: 0.0,
                    },
                }),
            },
        ),
    ]
    .into_iter()
    .map(|(form, record)| (RecordId::new(HARDCODED_MODULE_ID, form), record))
    .collect()
}

/// Returns the cumulative XP required to reach the given level.
pub fn cumulative_xp_for_level(level: u32) -> u64 {
    if level == 0 {
        return 0;
    }

    let level = u64::from(level);

    // From https://fallout.wiki/wiki/Level_%28Fallout_4%29
    // XPn = 37.5 * (n^2) + 87.5 * n - 124
    // Rewritten to avoid floats
    let a = 37 * level * level;
    let b = 87 * level;
    let c = (level * level) / 2;
    let d = level / 2;
    let e = level % 2;
    a + b + c + d + e - 124
}
