use bitflags::bitflags;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::conditions::Conditions;
use crate::records::{Array, impl_decode_bitflags};
use crate::vmad::VirtualMachineAdapter;
use crate::{FormId, LString, ZString};

#[derive(Clone, Debug, Record)]
pub struct MagicEffect {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adpater: Option<VirtualMachineAdapter>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MDOB")]
    pub menu_display_object: Option<FormId>,
    #[record("KSIZ")]
    _keywords_len: Option<u32>,
    /// A list of [`Keyword`] references.
    ///
    /// [`Keyword`]: crate::records::Keyword
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("DATA")]
    pub data: MagicEffectData,
    #[record("ESCE")]
    pub counter_effects: Vec<FormId>,
    #[record("SNDD")]
    pub sounds: Option<()>,
    #[record("DNAM")]
    pub magic_item_description: Option<LString>,
    #[record("CTDA")]
    pub conditions: Conditions,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct MagicEffectData {
    pub flags: MagicEffectFlags,
    pub base_cost: f32,
    pub associated_item: FormId,
    _pad0: u32,
    pub resistance: FormId,
    pub counter_effect_count: u16,
    _pad1: u16,
    pub casting_light: FormId,
    pub taper_weight: f32,
    pub hit_shader: FormId,
    pub enchant_shader: FormId,
    pub minimum_skil_level: u32,
    pub spellmaking_area: u32,
    pub spellmaking_casting_time: f32,
    pub taper_curve: f32,
    pub taper_duration: f32,
    pub second_av_weight: f32,
    pub r#type: MagicEffectType,
    pub actor_value: FormId,
    pub projectile: FormId,
    pub explosion: FormId,
    pub casting_type: MagicEffectCastType,
    pub delivery: MagicffectTargetType,
    pub second_actor_value: FormId,
    pub casting_art: FormId,
    pub hit_effect_shader: FormId,
    pub impact_data: FormId,
    pub skill_usage_factor: f32,
    pub dual_casting_art: FormId,
    pub dual_casting_scale: f32,
    pub enchant_art: FormId,
    pub hit_visuals: FormId,
    pub enchant_visuals: FormId,
    pub equip_ability: FormId,
    pub image_space_modifier: FormId,
    pub perk_to_apply: FormId,
    pub casting_sound_level: u32,
    pub script_effect_ai_score: f32,
    pub script_effect_ai_delay_time: f32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct MagicEffectFlags: u32 {
        const HOSTILE = 1 << 0;
        const RECOVER = 1 << 1;
        const DETRIMENTAL = 1 << 2;
        const SNAP_TO_NAVMESH = 1 << 3;
        const NO_HIT_EVENT = 1 << 4;
        const DISPEL_WITH_KEYWORDS = 1 << 8;
        const NO_DURATION = 1 << 9;
        const NO_MAGNITUDE = 1 << 10;
        const NO_AREA = 1 << 11;
        const FX_PERSIST = 1 << 12;
        const GORY_VISUAL = 1 << 13;
        const HIDE_IN_UI = 1 << 15;
        const NO_RECAST = 1 << 16;
        const POWER_AFFECTS_MAGNITUDE = 1 << 21;
        const POWER_AFFECTS_DURATION = 1 << 22;
        const PAINLESS = 1 << 26;
        const NO_HIT_EFFECT = 1 << 27;
        const NO_DEATH_DISPEL = 1 << 28;
    }
}

impl_decode_bitflags!(MagicEffectFlags: u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum MagicEffectType {
    ValueModifier = 0,
    Script = 1,
    Dispel = 2,
    CureDisease = 3,
    Absorb = 4,
    DualValueModifier = 5,
    Calm = 6,
    Demorialize = 7,
    Frenzy = 8,
    Disarm = 9,
    CommandSummoned = 10,
    Invisibility = 11,
    Light = 12,
    Darkness = 13,
    Nighteye = 14,
    Lock = 15,
    Open = 16,
    BoundWeapon = 17,
    SummonCreature = 18,
    DetectLife = 19,
    Telekinesis = 20,
    Paralysis = 21,
    Reanimate = 22,
    SoulTrap = 23,
    TurnUndead = 24,
    Guide = 25,
    _26 = 26,
    CureParalysis = 27,
    CureAddicition = 28,
    CurePoison = 29,
    Concussion = 30,
    Stimpack = 31,
    AccumulateMagnitude = 32,
    Stagger = 33,
    PeakValueModifier = 34,
    Cloak = 35,
    _36 = 36,
    SlowTime = 37,
    Rally = 38,
    EnhanceWeapon = 39,
    SpawnHazard = 40,
    Etherealize = 41,
    Banish = 42,
    SpawnScriptedRef = 43,
    Disguise = 44,
    Damage = 45,
    Immunity = 46,
    PermanentReanimate = 47,
    Jetpack = 48,
    Chameleion = 49,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum MagicEffectCastType {
    Constant = 0,
    FireAndForget = 1,
    Concentration = 2,
    Scroll = 3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum MagicffectTargetType {
    Self_ = 0,
    Touch = 1,
    Aimed = 2,
    TargetActor = 3,
    TargetLocation = 4,
}
