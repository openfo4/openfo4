//! Hardcoded defined in `Fallout4.esm`.

use openfo4_core::record::RecordId;
use openfo4_esp::FormId;

use super::HARDCODED_MODULE_ID;

macro_rules! define_forms {
    ($($id:ident => $form:expr),*$(,)?) => {
        $(
            pub const $id: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId($form));
        )*
    };
}

define_forms! {
    GAME_SETTING_F_PHYSICAL_DAMAGE_FACTOR => 0x769cb,
    GAME_SETTING_F_PHYSICAL_ARMOR_DMG_REDUCTION_EXP => 0x17d8a9,

    ACTOR_VALUE_STRENGTH => 0x2c2,
    ACTOR_VALUE_PERCEPTION => 0x2c3,
    ACTOR_VALUE_ENDURANCE => 0x2c4,
    ACTOR_VALUE_CHARISMA => 0x2c5,
    ACTOR_VALUE_INTELLIGENCE => 0x2c6,
    ACTOR_VALUE_AGILITY => 0x2c7,
    ACTOR_VALUE_LUCK => 0x2c8,
    ACTOR_VALUE_HEALTH => 0x2d4,

    // Movement Type
    DEFAULT_MOVEMENT_TYPE_DEFAULT => 0x3580d,
    DEFAULT_MOVEMENT_TYPE_FLY => 0x3580d,
    DEFAULT_MOVEMENT_TYPE_SNEAK => 0x18a58,
    DEFAULT_MOVEMENT_TYPE_SWIM => 0x36d42,
}
