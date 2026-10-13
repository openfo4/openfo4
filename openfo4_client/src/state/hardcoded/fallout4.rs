//! Hardcoded defined in `Fallout4.esm`.

use openfo4_core::record::RecordId;
use openfo4_esp::FormId;

use crate::systems::global::GameSettings;

use super::HARDCODED_MODULE_ID;

macro_rules! define_forms {
    ($($id:ident => $form:expr),*$(,)?) => {
        $(
            pub const $id: RecordId = RecordId::new(HARDCODED_MODULE_ID, FormId($form));
        )*
    };
}

define_forms! {
    GOLD => 0xf,

    // Game Setting
    GAME_SETTING_F_PHYSICAL_DAMAGE_FACTOR => 0x769cb, // fPhyiscalDamageFactor
    GAME_SETTING_F_PHYSICAL_ARMOR_DMG_REDUCTION_EXP => 0x17d8a9, // fPhysicalArmorDmgReductionExp
    GAME_SETTING_F_PHYISCAL_MIN_DAMAGE_REDUCTION => 0x66dc7, // fPhysicalMinDamageReduction
    GAME_SETTING_F_PHYSICAL_MAX_DAMAGE_REDUCTION => 0x66dc6, // fPhysicalMaxDamageReduction

    GAME_SETTING_F_ENERGY_DAMAGE_FACTOR => 0x769c8, // fEnergyDamageFactor
    GAME_SETTING_F_ENERGY_ARMOR_DMG_REDUCTION_EXP => 0x17d8a6, // fEnergyArmorDmgReductionExp
    GAME_SETTING_F_ENERGY_MIN_DAMAGE_REDUCTION => 0x6461d, // fEnergyMinDamageReduction
    GAME_SETTING_F_ENERGY_MAX_DAMAGE_REDUCTION => 0x6461e, // fEnergyMaxDamageReduction

    GAME_SETTING_F_FIRE_DAMAGE_FACTOR => 0x769c9, // fFireDamageFactor
    GAME_SETTING_F_FIRE_ARMOR_DMG_REDUCTION_EXP => 0x17d8a7, // fFireArmorDmgReductionExp
    GAME_SETTING_F_FIRE_MIN_DAMAGE_REDUCTION => 0x6461c, // fFireMinDamageReduction
    GAME_SETTING_F_FIRE_MAX_DAMAGE_REDUCTION => 0x6461b, // fFireMaxDamageReduction

    GAME_SETTING_F_FROST_DAMAGE_FACTOR => 0x769cA, // fFrostDamageFactor
    GAME_SETTING_F_FROST_ARMOR_DMG_REDUCTION_EXP => 0x17d8a8, // fFrostArmorDmgReductionExp
    GAME_SETTING_F_FROST_MIN_DAMAGE_REDUCTION => 0x64620, // fFrostMinDamageReduction
    GAME_SETTING_F_FROST_MAX_DAMAGE_REDUCTION => 0x6461f, // fFrostMaxDamageReduction

    GAME_SETTING_F_POISON_DAMAGE_FACTOR => 0x769cc, // fPoisonDamageFactor
    GAME_SETTING_F_POISON_ARMOR_DMG_REDUCTION_EXP => 0x17d8aa, // fPoisonArmorDmgReductionExp

    GAME_SETTING_F_RADS_DAMAGE_FACTOR => 0x769cd, // fRadsDamageFactor
    GAME_SETTING_F_RADS_ARMOR_DMG_REDUCTION_EXP => 0x17d8ab, // fRadsArmorDmgReductionExp
    GAME_SETTING_F_RADS_MAX_DAMAGE_REDUCTION => 0x559a3, // fRadsMaxDamageReduction

    GAME_SETTING_F_SHOCK_DAMAGE_FACTOR => 0x769ce, // fShockDamageFactor
    GAME_SETTING_F_SHOCK_ARMOR_DMG_REDUCTION_EXP => 0x17d8ac, // fShockArmorDmgReductionExp
    GAME_SETTING_F_SHOCK_MIN_DAMAGE_REDUCTION => 0x64623, // fShockMinDamageReduction
    GAME_SETTING_F_SHOCK_MAX_DAMAGE_REDUCTION => 0x64624, // fShockMaxDamageReduction

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

    // Damage Type
    DAMAGE_TYPE_ELECTRICAL => 0x60a80, // dtElectrical
    DAMAGE_TYPE_ENERGY => 0x60a81, // dtEnergy
    DAMAGE_TYPE_FIRE => 0x60a82, // dtFire
    DAMAGE_TYPE_CRYO => 0x60a83, // dtCryo
    DAMAGE_TYPE_POISON => 0x60a84, // dtPoison
    DAMAGE_TYPE_RADIATION_EXPOSURE => 0x60a85, // dtRadiationExposure
    DAMAGE_TYPE_RADIATION_INGESTION => 0x60a86, // dtRadiationIngestion
    DAMAGE_TYPE_PHYSICAL => 0x60a87, // dtPhysical
}

pub fn apply_weapon_damage(
    dmg_type: RecordId,
    dmg_value: f32,
    resistance: f32,
    settings: &GameSettings,
) -> f32 {
    let (fac, exp, min, max) = match dmg_type {
        ty if ty == DAMAGE_TYPE_PHYSICAL => (
            GAME_SETTING_F_PHYSICAL_DAMAGE_FACTOR,
            GAME_SETTING_F_PHYSICAL_ARMOR_DMG_REDUCTION_EXP,
            GAME_SETTING_F_PHYISCAL_MIN_DAMAGE_REDUCTION,
            GAME_SETTING_F_PHYSICAL_MAX_DAMAGE_REDUCTION,
        ),
        ty if ty == DAMAGE_TYPE_ENERGY => (
            GAME_SETTING_F_ENERGY_DAMAGE_FACTOR,
            GAME_SETTING_F_ENERGY_ARMOR_DMG_REDUCTION_EXP,
            GAME_SETTING_F_ENERGY_MIN_DAMAGE_REDUCTION,
            GAME_SETTING_F_ENERGY_MAX_DAMAGE_REDUCTION,
        ),
        ty if ty == DAMAGE_TYPE_FIRE => (
            GAME_SETTING_F_FIRE_DAMAGE_FACTOR,
            GAME_SETTING_F_FIRE_ARMOR_DMG_REDUCTION_EXP,
            GAME_SETTING_F_FIRE_MIN_DAMAGE_REDUCTION,
            GAME_SETTING_F_FIRE_MAX_DAMAGE_REDUCTION,
        ),
        ty if ty == DAMAGE_TYPE_CRYO => (
            GAME_SETTING_F_FROST_DAMAGE_FACTOR,
            GAME_SETTING_F_FROST_ARMOR_DMG_REDUCTION_EXP,
            GAME_SETTING_F_FROST_MIN_DAMAGE_REDUCTION,
            GAME_SETTING_F_FROST_MAX_DAMAGE_REDUCTION,
        ),
        ty if ty == DAMAGE_TYPE_ELECTRICAL => (
            GAME_SETTING_F_SHOCK_DAMAGE_FACTOR,
            GAME_SETTING_F_SHOCK_ARMOR_DMG_REDUCTION_EXP,
            GAME_SETTING_F_SHOCK_MIN_DAMAGE_REDUCTION,
            GAME_SETTING_F_SHOCK_MAX_DAMAGE_REDUCTION,
        ),
        ty if ty == DAMAGE_TYPE_POISON => (
            GAME_SETTING_F_POISON_DAMAGE_FACTOR,
            GAME_SETTING_F_POISON_ARMOR_DMG_REDUCTION_EXP,
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x0)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x0)),
        ),
        // Dummy values that do not map to any GameSetting.
        _ => (
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x0)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x0)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x0)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x0)),
        ),
    };

    let fac = settings.get(fac).and_then(|v| v.as_f32()).unwrap_or(0.15);
    let exp = settings.get(exp).and_then(|v| v.as_f32()).unwrap_or(0.365);
    let min = settings.get(min).and_then(|v| v.as_f32()).unwrap_or(0.05);
    let max = settings.get(max).and_then(|v| v.as_f32()).unwrap_or(0.99);

    let coeff = (dmg_value * fac / resistance).powf(exp).min(max).max(min);
    dmg_value * coeff
}
