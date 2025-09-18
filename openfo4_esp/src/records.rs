use bitflags::bitflags;
use bytes::Buf;
use glam::Vec3;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::fields::{FieldMap, Read, SubRecordArray, SubRecordSection};
use crate::vmad::{QuestFragments, VirtualMachineAdapter};
use crate::{Decode, Error, FormId, LString, Rgb, RgbPadded, Text, ZString};

macro_rules! impl_decode_bitflags {
    ($($t:ty:$i:ty),*) => {
        $(
            impl Decode for $t {
                type Error = <$i as Decode>::Error;

                fn decode<B>(buf: B) -> Result<Self, Self::Error>
                where
                    B: Buf,
                {
                    <$i>::decode(buf).map(Self::from_bits_truncate)
                }
            }
        )*
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Empty;

impl Read for Empty {
    type Error = Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Ok(Empty)
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Rgba<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

#[derive(Clone, Debug, Record)]
pub struct Tes4 {
    #[record("HEDR")]
    pub header: Tes4Header,
    #[record("CNAM")]
    pub author: Option<ZString>,
    #[record("SNAM")]
    pub description: Option<ZString>,
    #[record("INTV")]
    pub intv: u32,
    #[record("INCC")]
    pub incc: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Tes4Header {
    pub version: f32,
    pub number_of_records: u32,
    pub next_available_object_id: u32,
}

#[derive(Clone, Debug)]
pub struct GameSetting {
    pub id: ZString,
    pub data: TypedValue,
}

impl Read for GameSetting {
    type Error = Error;

    fn read(mut map: &mut FieldMap, _key: Text<4>) -> Result<Self, Self::Error> {
        let id = ZString::read(&mut map, Text::EDID)?;
        let data = match id.as_bytes().get(0) {
            Some(b'b') => {
                let value = u32::read(&mut map, Text::DATA)?;
                TypedValue::Boolean(value == 0)
            }
            Some(b'i') => {
                let value = i32::read(&mut map, Text::DATA)?;
                TypedValue::Int(value)
            }
            Some(b'f') => {
                let value = f32::read(&mut map, Text::DATA)?;
                TypedValue::Float(value)
            }
            Some(b's') | Some(b'S') => {
                let value = LString::read(&mut map, Text::DATA)?;
                TypedValue::LString(value)
            }
            Some(b'u') => {
                let value = u32::read(&mut map, Text::DATA)?;
                TypedValue::UnsignedInt(value)
            }
            _ => todo!(),
        };

        Ok(Self { id, data })
    }
}

#[derive(Clone, Debug)]
pub enum TypedValue {
    Boolean(bool),
    Int(i32),
    Float(f32),
    String(String),
    Char(char),
    HexChar(char),
    UnsignedInt(u32),
    LString(LString),
    Rgb(),
}

/// A keyword.
#[derive(Clone, Debug, Record)]
#[non_exhaustive]
pub struct Keyword {
    /// The unique editor ID.
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("CNAM")]
    pub color: Option<RgbPadded<u8>>,
    #[record("DNAM")]
    pub notes: Option<ZString>,
    #[record("TNAM")]
    pub r#type: Option<u32>,
    #[record("DATA")]
    pub attraction_rule: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
}

#[derive(Clone, Debug, Record)]
pub struct LocationReferenceType {
    #[record("EDID")]
    pub id: ZString,
    #[record("CNAM")]
    pub color: Option<RgbPadded<u8>>,
    #[record("TNAM")]
    tnam_unknown: Option<u32>,
}

#[derive(Clone, Debug, Record)]
pub struct Action {
    #[record("EDID")]
    pub edid: Option<ZString>,
    #[record("CNAM")]
    pub cnam: Option<RgbPadded<u8>>,
    #[record("TNAM")]
    pub tnam: Option<u32>,
}

#[derive(Clone, Debug, Record)]
pub struct TRNS {
    #[record("EDID")]
    pub edid: ZString,
    #[record("DATA")]
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Record)]
pub struct CMPO {
    #[record("EDID")]
    pub edid: ZString,
    #[record("GNAM")]
    pub gnam: Option<u32>,
    #[record("OBND")]
    pub obnd: Option<Bounds>,
    #[record("FULL")]
    pub full: Option<u32>,
    /// Probably use "Crafting Use Sound"
    #[record("CUSD")]
    pub cusd: Option<FormId>,
    #[record("MNAM")]
    pub mnam: Option<u32>,
}

#[derive(Copy, Clone, Debug)]
pub struct Bounds {
    x1: i16,
    y1: i16,
    z1: i16,
    x2: i16,
    y2: i16,
    z2: i16,
}

impl Decode for Bounds {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        Ok(Self {
            x1: i16::decode(&mut buf)?,
            y1: i16::decode(&mut buf)?,
            z1: i16::decode(&mut buf)?,
            x2: i16::decode(&mut buf)?,
            y2: i16::decode(&mut buf)?,
            z2: i16::decode(&mut buf)?,
        })
    }
}

#[derive(Clone, Debug, Record)]
pub struct TextureSet {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("TX00")]
    pub diffuse_texture: Option<ZString>,
    #[record("TX01")]
    pub normal_texture: Option<ZString>,
    #[record("TX02")]
    pub emissive_texture: Option<ZString>,
    #[record("TX03")]
    pub height_texture: Option<ZString>,
    #[record("TX04")]
    pub environment_texture: Option<ZString>,
    #[record("TX05")]
    pub tx05: Option<ZString>,
    #[record("TX06")]
    pub tx06: Option<ZString>,
    #[record("TX07")]
    pub smooth_spec_texture: Option<ZString>,
    #[record("DODT")]
    pub dodt: Option<DecalData>,
    #[record("DNAM")]
    pub flags: TextureSetFlags,
    #[record("MNAM")]
    pub material: Option<ZString>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct DecalData {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub depth: f32,
    pub shininess: f32,
    pub parallax_scale: f32,
    pub parallax_passes: u8,
    pub flags: u8,
    pub _unknown0: u16,
    pub color: RgbPadded<u8>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct TextureSetFlags: u16 {
        const NO_SPECULAR_MAP = 0x0001;
        const FACEGEN_TEXTURES = 0x0002;
        const HAS_MODEL_SPACE_NORMAL_MAP = 0x0004;
    }
}

impl_decode_bitflags!(TextureSetFlags: u16);

#[derive(Clone, Debug, Record)]
pub struct GLOB {
    #[record("EDID")]
    pub edid: ZString,
    #[record("FNAM")]
    pub fnam: Option<u8>,
    #[record("FLTV")]
    pub fltv: f32,
}

#[derive(Clone, Debug, Record)]
pub struct DMGT {
    #[record("EDID")]
    pub edid: ZString,
    #[record("DNAM")]
    pub dnam: u64,
}

#[derive(Clone, Debug, Record)]
pub struct CLAS {
    #[record("EDID")]
    pub edid: ZString,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("DESC")]
    pub desc: LString,
    #[record("ICON")]
    pub icon: Option<ZString>,
    // Doesnt fit
    // #[record("DATA")]
    // pub data: ClassData,
    // pub prps: (),
}

#[derive(Clone, Debug)]
pub struct ClassData {
    _unknown0: u32,
    training_skill: u8,
    training_level: u8,
    skill_weights: [u8; 18],
    bleedout_default: f32,
    voice_points: u32,
    health_weight: u8,
    magicka_weight: u8,
    stamina_weight: u8,
    flags: u8,
}

impl Decode for ClassData {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        Ok(Self {
            _unknown0: u32::decode(&mut buf)?,
            training_skill: u8::decode(&mut buf)?,
            training_level: u8::decode(&mut buf)?,
            skill_weights: <[u8; 18]>::decode(&mut buf)?,
            bleedout_default: f32::decode(&mut buf)?,
            voice_points: u32::decode(&mut buf)?,
            health_weight: u8::decode(&mut buf)?,
            magicka_weight: u8::decode(&mut buf)?,
            stamina_weight: u8::decode(&mut buf)?,
            flags: u8::decode(&mut buf)?,
        })
    }
}

#[derive(Clone, Debug, Record)]
pub struct FACT {
    #[record("EDID")]
    pub edid: ZString,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("XNAM")]
    pub xnam: Vec<InterfactionRelation>,
    #[record("DATA")]
    pub data: u32,
    #[record("JAIL")]
    pub jail: Option<FormId>,
    #[record("WAIT")]
    pub wait: Option<FormId>,
    #[record("STOL")]
    pub stol: Option<FormId>,
    #[record("PLCN")]
    pub plcn: Option<FormId>,
    #[record("CRGR")]
    pub crgr: Option<FormId>,
    #[record("JOUT")]
    pub jout: Option<FormId>,
    #[record("CRVA")]
    pub crva: Option<FACT_CRVA>,
    #[record("RNAM")]
    pub rnam: Vec<u32>,
    #[record("MNAM")]
    pub mnam: Vec<LString>,
    #[record("FNAM")]
    pub fnam: Vec<LString>,
    #[record("VEND")]
    pub vend: Option<FormId>,
    #[record("VENC")]
    pub venc: Option<FormId>,
    #[record("VENV")]
    pub venv: Option<FACT_VENV>,
    #[record("PLVD")]
    pub plvd: Option<FACT_PLVD>,
}

#[derive(Copy, Clone, Debug)]
pub struct InterfactionRelation {
    faction: FormId,
    _unknown0: i32,
    combat: i32,
}

impl Decode for InterfactionRelation {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: bytes::Buf,
    {
        Ok(Self {
            faction: FormId::decode(&mut buf)?,
            _unknown0: i32::decode(&mut buf)?,
            combat: i32::decode(&mut buf)?,
        })
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FACT_CRVA {
    arrest: u8,
    attack_on_sight: u8,
    murder: u16,
    assault: u16,
    trespass: u16,
    pickpocket: u16,
    _unused: u16,
    steal_mult: f32,
    escape: u16,
    werewolf: u16,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FACT_VENV {
    pub start_hour: u16,
    pub end_hour: u16,
    pub radius: u32,
    pub buys_stolen: u8,
    pub invert_sell_buy_list: u8,
    _unused0: u16,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FACT_PLVD {
    pub specification_type: u32,
    pub form: FormId,
    _unused: u32,
}

#[derive(Clone, Debug, Record)]
pub struct HDPT {
    #[record("EDID")]
    pub edid: ZString,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("MODL")]
    pub modl: Option<ZString>,
    #[record("MODT")]
    pub modt: Option<FormId>,
    #[record("DATA")]
    pub data: u8,
    #[record("PNAM")]
    pub pnam: u32,
    #[record("HNAM")]
    pub hnam: Vec<FormId>,
    #[record("NAM0")]
    pub nam0: Vec<u32>,
    #[record("NAM1")]
    pub nam1: Vec<ZString>,
    #[record("TNAM")]
    pub tnam: Option<FormId>,
    #[record("RNAM")]
    pub rnam: Option<FormId>,
    #[record("CNAM")]
    pub cnam: Option<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct Model {
    #[record("MODL")]
    pub model: Option<ZString>,
    #[record("MODT")]
    pub model_texture: Option<ModelInfo>,
}

#[derive(Clone, Debug)]
pub struct ModelInfo {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    textures: Vec<ZString>,
}

impl Decode for ModelInfo {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let header_size = u32::decode(&mut buf)?;
        let texture_count = if header_size >= 1 {
            u32::decode(&mut buf)?
        } else {
            0
        };
        let addon_count = if header_size >= 2 {
            u32::decode(&mut buf)?
        } else {
            0
        };
        let texture_sets = if header_size >= 3 {
            u32::decode(&mut buf)?
        } else {
            0
        };
        let material_count = if header_size >= 4 {
            u32::decode(&mut buf)?
        } else {
            0
        };

        let mut textures = Vec::new();
        for _ in 0..texture_count {
            // let x = u32::decode(&mut buf)?;
            // let x = <[u8; 4]>::decode(&mut buf)?;
            // let texture = ZString::decode(&mut buf)?;
            // textures.push(texture);
        }

        Ok(Self {
            a: header_size,
            b: texture_count,
            c: addon_count,
            d: texture_sets,
            e: material_count,
            textures,
        })
    }
}

#[derive(Clone, Debug, Record)]
pub struct Race {
    /// The editor ID.
    #[record("EDID")]
    pub id: ZString,
    /// The localized name of the record.
    #[record("FULL")]
    pub full_name: Option<LString>,
    /// The description of the record.
    #[record("DESC")]
    pub description: Option<LString>,
    #[record("SPCT")]
    effect_count: Option<u32>,
    #[record("SPLO")]
    pub effects: Vec<FormId>,
    /// Link to [`Armor`] that contains the skin of the race.
    #[record("WNAM")]
    pub skin: Option<FormId>,
    #[record("BODT")]
    pub bodt: Option<RACE_BODT>,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    /// List of [`KYWD`] records.
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("DATA")]
    pub data: RaceData,
    #[record("MNAM")]
    pub male_skeletal_model: Option<SubRecordSection<RaceSkeletalModel>>,
    #[record("FNAM")]
    pub female_skeletal_model: Option<SubRecordSection<RaceSkeletalModel>>,
    #[record("MTMN")]
    pub movement_type_names: Vec<Text<4>>,
    /// Voice types for male and female.
    #[record("VTCK")]
    pub voices: Option<PerGender<FormId>>,
    /// Default hair colors for male and female.
    #[record("HCLF")]
    pub default_hair_colors: Option<PerGender<FormId>>,
    #[record("TINL")]
    pub total_number_of_tints: Option<u16>,
    #[record("PNAM")]
    pub face_gen_main_clamp: f32,
    #[record("UNAM")]
    pub face_gen_face_clamp: f32,
    #[record("ATKR")]
    pub attack_race: Option<FormId>,
    #[record("ATKD")]
    pub attacks: Vec<RACE_ATKD>,
    #[record("NAM1")]
    pub body_parts: Option<SubRecordSection<RaceBodyParts>>,
    #[record("GNAM")]
    pub body_part_data: Option<FormId>,
    #[record("NAM3")]
    pub behavior_graph: Option<SubRecordSection<RaceBehaviorGraph>>,
    #[record("NAM4")]
    pub impact_material_set: Option<FormId>,
    #[record("NAM5")]
    pub impact_data_set: Option<FormId>,
    #[record("NAM7")]
    pub dismember_blood_art: Option<FormId>,
    #[record("CNAM")]
    pub meat_cap_texture_set: Option<FormId>,
    #[record("NAM2")]
    pub collar_texture_set: Option<FormId>,
    #[record("ONAM")]
    pub sound_open_corpse: Option<FormId>,
    #[record("LNAM")]
    pub sound_close_corpse: Option<FormId>,
    #[record("NAME")]
    pub biped_object_names: SubRecordArray<ZString>,
    #[record("RBPC")]
    pub biped_object_conditions: Option<Array<FormId>>,
    // movement_data_overrides
    // pub equipment_flags:
    #[record("UNWP")]
    pub unarmed_weapon: Option<FormId>,
    #[record("PHTN")]
    pub phenome_target_names: SubRecordArray<ZString>,
    // PHWT
    /// Reference to [`MovementType`] that is active by default.
    #[record("WKMV")]
    pub base_movement_defaults_default: Option<FormId>,
    /// Reference to [`MovementType`] that is active when the actor is swimming.
    #[record("SWMV")]
    pub base_movement_defaults_swim: Option<FormId>,
    /// Reference to [`MovementType`] that is active when the actor is flying.
    #[record("FLMV")]
    pub base_movement_defaults_fly: Option<FormId>,
    /// Reference to [`MovementType`] that is active when the actor is sneaking.
    #[record("SNMV")]
    pub base_movement_defaults_sneak: Option<FormId>,
    /// Reference to another `Race` record which contains all the subgraph data that should be used
    /// for this `Race`.
    #[record("SRAC")]
    pub subgraph_template_race: Option<FormId>,
    /// Reference to another `Race` to which the subgraph data of the current `Race` will be added.
    ///
    /// If set to `Some(..)` the `Race` is not an ordinary race. Instead the subgraph data from the
    /// `Race` is additively added to referenced `Race`.
    #[record("SADD")]
    pub subgraph_additive_race: Option<FormId>,
    #[record("SGNM")]
    pub subgraph_data: SubRecordArray<RaceSubgraphData>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceSubgraphData {
    #[record("SAKD")]
    pub actor_keywords: Vec<FormId>,
    #[record("SGNM")]
    pub behavior_graph: ZString,
    #[record("STKD")]
    pub target_keywords: Vec<FormId>,
    #[record("SRAF")]
    pub flags: SubgraphFlags,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct SubgraphFlags {
    pub role: SubgraphRole,
    pub perspective: SubgraphPerspective,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u16)]
pub enum SubgraphRole {
    Mt = 0,
    Weapon = 1,
    Furniture = 2,
    Idle = 3,
    Pipboy = 4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u16)]
pub enum SubgraphPerspective {
    Third = 0,
    First = 1,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RaceData {
    pub male_height: f32,
    pub female_height: f32,
    pub male_default_weight: RaceDefaultWeight,
    pub female_default_weight: RaceDefaultWeight,
    pub flags: RaceDataFlags,
    pub acceleration_rate: f32,
    pub deceleration_rate: f32,
    pub size: RaceDataSize,
    _unknown0: u32,
    _unknown1: u32,
    pub injured_health_percent: f32,
    pub shield_biped_object: i32,
    pub beard_biped_object: i32,
    pub body_biped_object: i32,
    pub aim_angle_tolerance: f32,
    pub flight_radius: f32,
    pub angular_acceleration_rate: f32,
    pub angular_tolerance: f32,
    pub flags2: RaceDataFlags2,
    _unknown2: u32,
    _unknown3: u32,
    _unknown4: u32,
    _unknown5: u32,
    _unknwon6: u32,
    _unknown7: u32,
    _unknown8: u32,
    _unknown9: u32,
    _unknown10: u32,
    pub pipbody_biped_object: i32,
    pub xp_value: i32,
    pub severable_debris_scale: f32,
    pub severable_debris_count: u8,
    pub severable_decal_count: u8,
    pub explodable_debris_scale: f32,
    pub explodable_debris_count: u8,
    pub explodable_decal_count: u8,
    pub severable_explosion: FormId,
    pub severable_debris: FormId,
    pub explodable_explosion: FormId,
    pub explodable_debris: FormId,
    pub explodable_impact_data_set: FormId,
    pub on_cripple_debris_scale: f32,
    pub on_cripple_debris_count: u8,
    pub on_cripple_decal_count: u8,
    pub on_cripple_explosion: FormId,
    pub on_cripple_debris: FormId,
    pub on_cripple_impact_data_set: FormId,
    pub explodable_subsegment_explosion: FormId,
    pub orientation_limits_pitch: f32,
    pub orientation_limits_roll: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RaceDefaultWeight {
    pub thin: f32,
    pub muscular: f32,
    pub fat: f32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct RaceDataFlags: u32 {
        const PLAYABLE = 0x00000001;
        const FACEGEN_HEAD = 0x00000002;
        const CHILD = 0x00000004;
        const TILT_FRONT_BACK = 0x00000008;
        const TILT_LEFT_RIGHT = 0x00000010;
        const NO_SHADOW = 0x00000020;
        const SWIMS = 0x00000040;
        const FLIES = 0x00000080;
        const WALKS = 0x00000100;
        const IMMOBILE = 0x00000200;
        const NOT_PUSHABLE = 0x00000400;
        const NO_COMBAT_IN_WATER = 0x00000800;
        const NO_ROTATION_TO_HEAD_TRACK = 0x00001000;
        const DONT_SHOW_BLOOD_SPRAY = 0x00002000;
        const DONT_SHOW_BLOOD_DECAL = 0x00004000;
        const USES_HEAD_TRACK_ANIMS = 0x00008000;
        const SPELLS_ALIGN_WITH_MAGIC_NODE = 0x00010000;
        const USE_WORLD_RAYCAST_FOR_FOOT_IK = 0x00020000;
        const ALLOW_RAGDOLL_COLLISION = 0x00040000;
        const REGEN_HP_IN_COMBAT = 0x00080000;
        const CANT_OPEN_DOORS = 0x00100000;
        const ALLOW_PC_DIALOGUE = 0x00200000;
        const NO_KNOCKDOWNS = 0x00400000;
        const ALLOW_PICKPOCKET = 0x00800000;
        const ALWAYS_USE_PROXY_CONTROLLER = 0x01000000;
        const DONT_SHOW_WEAPON_BLOOD = 0x02000000;
        const OVERLAY_HEAD_PART_LIST = 0x04000000;
        const OVERRIDE_HEAD_PART_LIST = 0x08000000;
        const CAN_PICKUP_ITEMS = 0x10000000;
        const ALLOW_MULTIPLE_MEMBRANE_SHADERS = 0x20000000;
        const CAN_DUAL_WIELD = 0x40000000;
        const AVOIDS_ROADS = 0x80000000;
    }
}

impl_decode_bitflags!(RaceDataFlags: u32);

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct RaceDataFlags2: u32 {
        const USE_ADVANCED_AVOIDANCE = 0x00000001;
        const NON_HOSTILE = 0x00000002;
        const FLOATS = 0x00000004;
        const HEAD_AXIS_BIT_0 = 0x00000020;
        const HEAD_AXIS_BIT_1 = 0x00000040;
        const CAN_MELEE_WHEN_KNOCKED_DOWN = 0x00000080;
        const USE_IDLE_CHATTER_DURING_COMBAT = 0x00000100;
        const UNGENDERED = 0x00000200;
        const CAN_MOVE_WHEN_KNOCKED_DOWN = 0x00000400;
        const USE_LARGE_ACTOR_PATHING = 0x00000800;
        const USE_SUBSEGMENTED_DAMAGE = 0x00001000;
        const FLIGHT_DEFER_KILL = 0x00002000;
        const FLIGHT_ALLOW_PROCEDURAL_CRASH_LAND = 0x00008000;
        const DISABLE_WEAPON_CULLING = 0x00010000;
        const USE_OPTIMAL_SPEEDS = 0x00020000;
        const HAS_FACIAL_RIG = 0x00040000;
        const CAN_USE_CRIPPLED_LIMBS = 0x00080000;
        const USE_QUADRUPED_CONTROLLER = 0x00100000;
        const LOW_PRIORITY_PUSHABLE = 0x00200000;
        const CANNOT_USE_PLAYABLE_ITEMS = 0x00400000;
    }
}

impl_decode_bitflags!(RaceDataFlags2: u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum RaceDataSize {
    Small = 0,
    Medium = 1,
    Large = 2,
    ExtraLarge = 3,
}

#[derive(Clone, Debug, Record)]
pub struct RaceSkeletalModel {
    #[record("ANAM")]
    pub path: ZString,
    #[record("MODT")]
    pub info: ModelInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct PerGender<T> {
    pub male: T,
    pub female: T,
}

#[derive(Clone, Debug, Record)]
pub struct BodyPart {
    #[record("INDX")]
    pub index: Option<()>,
    #[record("MODL")]
    pub model: Model,
}

#[derive(Clone, Debug, Record)]
pub struct BodyPartSection {
    #[record("MODL")]
    pub parts: SubRecordArray<BodyPart>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceBodyParts {
    #[record("MNAM")]
    pub male: Option<SubRecordSection<BodyPartSection>>,
    #[record("FNAM")]
    pub female: Option<SubRecordSection<BodyPartSection>>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceBehaviorGraph {
    #[record("MNAM")]
    pub male: Option<SubRecordSection<Model>>,
    #[record("FNAM")]
    pub female: Option<SubRecordSection<Model>>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RACE_BODT {
    pub biped_object_flags: u32,
    pub general_flags: u32,
    pub armor_type: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RACE_DATA {
    pub skill_1: u8,
    pub racial_bonus_for_skill_1: u8,
    pub skill_2: u8,
    pub racial_bonus_for_skill_2: u8,
    pub skill_3: u8,
    pub racial_bonus_for_skill_3: u8,
    pub skill_4: u8,
    pub racial_bonus_for_skill_4: u8,
    pub skill_5: u8,
    pub racial_bonus_for_skill_5: u8,
    pub skill_6: u8,
    pub racial_bonus_for_skill_6: u8,
    pub skill_7: u8,
    pub racial_bonus_for_skill_7: u8,
    _pad0: u16,
    pub height_male: f32,
    pub height_female: f32,
    pub weight_male: f32,
    pub weight_female: f32,
    pub flags: u32,
    pub starting_health: f32,
    pub starting_magicka: f32,
    pub starting_stamina: f32,
    pub base_mass: f32,
    pub acceleration_rate: f32,
    pub deceleration_rate: f32,
    /// Size
    /// 0=Small, 1=Medium,2=Large,3=Extra Large
    pub size: u32,
    pub head_biped_object: u32,
    pub hair_biped_object: u32,
    pub injured_health_pct: f32,
    pub shield_biped_object: u32,
    pub health_regen: f32,
    pub magicka_regen: f32,
    pub stamina_regen: f32,
    pub body_biped_object: u32,
    pub aim_angle_tolerance: f32,
    _unknown0: u32,
    pub angular_acceleration_rate: f32,
    pub angular_tolerance: f32,
    pub flag2: u32,
    pub mount_data_offset_x: u32,
    pub mount_data_offset_y: u32,
    _unknown1: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RACE_MODT {}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RACE_ATKD {
    pub damage_mult: f32,
    pub attack_chance: f32,
    pub attack_spell: FormId,
    pub flags: u32,
    pub attack_angle: f32,
    pub strike_angle: f32,
    pub stagger: f32,
    pub attack_type: FormId,
    pub knockdown: f32,
    pub recovery_time: f32,
    pub fatigue_mult: f32,
}

#[derive(Clone, Debug, Record)]
pub struct SOUN {
    #[record("EDID")]
    pub edid: ZString,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("SDSC")]
    pub sdsc: FormId,
}

/// An acoustic Space defines the ambient sounds played in a cell.
#[derive(Clone, Debug, Record)]
pub struct AcousticSpace {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    /// Looping ambient sound.
    ///
    /// Reference to a [`SoundDescriptor`] record.
    #[record("SNAM")]
    pub looping_sound: Option<FormId>,
    /// Use the sound from a region.
    ///
    /// Reference to a [`Region`] record.
    #[record("TDAT")]
    pub region_sound: Option<FormId>,
    /// Reveberation data to use.
    ///
    /// Reference to a [`ReverbParameters`] record.
    #[record("BNAM")]
    pub environment_type: Option<FormId>,
    #[record("XTRI")]
    pub is_interior: Option<bool>,
    /// Attenuation in db as a factor of 100.
    #[record("WNAM")]
    pub weather_attenuation: Option<u16>,
}

#[derive(Clone, Debug, Record)]
pub struct MGEF {
    #[record("EDID")]
    pub edid: ZString,
    /// TODO
    #[record("VMAD")]
    pub vmad: Option<()>,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("MDOB")]
    pub mdob: Option<FormId>,
    #[record("KSIZ")]
    pub ksiz: Option<u32>,
    #[record("KWDA")]
    pub kwda: Vec<FormId>,
    #[record("DATA")]
    pub data: MGEF_DATA,
    #[record("ESCE")]
    pub esce: Vec<FormId>,
    // TODO: Does not parse
    // #[record("SNDD")]
    // pub sndd: Vec<MGEF_SNDD>,
    #[record("DNAM")]
    pub dnam: LString,
    /// TODO
    #[record("CTDA")]
    pub ctda: Vec<()>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MGEF_DATA {
    pub flags: u32,
    pub base_cost: f32,
    pub related_id: FormId,
    pub skill: i32,
    pub resistance_av: u32,
    _unknown0: u32,
    pub light_id: FormId,
    pub taper_weight: f32,
    pub hit_shader: FormId,
    pub enchant_shader: FormId,
    pub skill_level: u32,
    pub area: u32,
    pub casting_time: f32,
    pub taper_curve: f32,
    pub taper_duration: f32,
    pub second_av_weight: f32,
    pub effect_type: u32,
    pub primary_av: i32,
    pub projectile_id: FormId,
    pub explosion_id: FormId,
    pub cast_type: u32,
    pub delivery_type: u32,
    pub second_av: i32,
    pub casting_art_id: FormId,
    pub hit_effect_art_id: FormId,
    pub impact_data_id: FormId,
    pub skill_usage_mult: f32,
    pub dual_cast_id: FormId,
    pub dual_cast_scale: f32,
    pub enchant_art_id: FormId,
    _pad0: u32,
    _pad1: u32,
    pub equip_ability: FormId,
    pub image_space_mod_id: FormId,
    pub perk_id: FormId,
    pub sound_volume: u32,
    pub script_ai_data_score: f32,
    pub script_ai_data_delay_time: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MGEF_SNDD {
    pub typ: u32,
    pub sound_desc: FormId,
}

#[derive(Clone, Debug, Record)]
pub struct LandTexture {
    #[record("EDID")]
    pub id: ZString,
    #[record("TNAM")]
    pub texture_set: FormId,
    #[record("MNAM")]
    pub material_type: FormId,
    #[record("HNAM")]
    pub havok_data: LandTextureHavokData,
    #[record("SNAM")]
    pub texture_sepcular_exponent: u8,
    #[record("GNAM")]
    pub grasses: Vec<FormId>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LandTextureHavokData {
    pub friction: u8,
    pub restitution: u8,
}

#[derive(Clone, Debug, Record)]
pub struct ENCH {
    #[record("EDID")]
    pub edid: ZString,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("ENIT")]
    pub enit: EnchantedItem,
    #[record("EFID")]
    pub efid: Vec<FormId>,
    #[record("EFIT")]
    pub efit: Vec<EffectItem>,
    /// TODO
    #[record("CTDA")]
    pub ctda: Vec<()>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct EnchantedItem {
    pub enchantment_cost: u32,
    pub flags: u32,
    pub cast_type: u32,
    pub ench_amount: u32,
    pub delivery: u32,
    pub enchant_type: u32,
    pub charge_time: f32,
    pub base_echantment: FormId,
    pub worn_restrictions: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct EffectItem {
    pub magnitude: f32,
    pub area_of_effect: u32,
    pub duration: u32,
}

#[derive(Clone, Debug, Record)]
pub struct SPEL {
    #[record("EDID")]
    pub edid: ZString,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("MDOB")]
    pub mdob: Option<FormId>,
    #[record("ETYP")]
    pub etyp: FormId,
    #[record("DESC")]
    pub desc: LString,
    #[record("SPIT")]
    pub spit: SpellItem,
    // TODO
    // EFID
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct SpellItem {
    pub spell_cost: u32,
    pub flags: u32,
    pub typ: u32,
    pub charge_time: f32,
    pub cast_type: u32,
    pub delivery: u32,
    pub cast_duration: f32,
    pub range: f32,
    pub perk: FormId,
}

#[derive(Clone, Debug, Record)]
pub struct Activator {
    #[record("EDID")]
    pub id: ZString,
    // TODO
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("STCP")]
    pub animation_sound: Option<FormId>,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("PRPS")]
    pub prps: Option<()>,
    #[record("NTRM")]
    pub ntrm: Option<()>,
    #[record("FTYP")]
    pub ftyp: Option<()>,
    #[record("PNAM")]
    pub marker_color: Option<RgbPadded<u8>>,
    #[record("SNAM")]
    pub sound_looping: Option<FormId>,
    #[record("VNAM")]
    pub sound_activation: Option<FormId>,
    #[record("WNAM")]
    pub water_type: Option<FormId>,
    // ATTX
    #[record("FNAM")]
    pub flags: Option<ActivatorFlags>,
    #[record("KNAM")]
    pub interaction_keyword: Option<FormId>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ActivatorFlags: u16 {
    }
}

impl_decode_bitflags!(ActivatorFlags: u16);

#[derive(Clone, Debug, Record)]
pub struct TACT {
    #[record("EDID")]
    pub edid: ZString,
    // TODO
    #[record("VMAD")]
    pub vmad: Option<()>,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: Option<LString>,
    // TODO
    #[record("MODL")]
    pub modl: Option<()>,
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("KSIZ")]
    pub ksiz: Option<u32>,
    // TODO
    #[record("KWDA")]
    pub kwda: Option<()>,
    #[record("SNAM")]
    pub snam: Option<FormId>,
    #[record("VNAM")]
    pub vnam: Option<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct Armor {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<()>,
    #[record("OBND")]
    pub object_bounds: Option<Bounds>,
    #[record("PTRN")]
    pub prview_transform: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("EITM")]
    pub object_effect: Option<FormId>,
    #[record("MOD2")]
    pub male_world_model: ArmorMaleModel,
    #[record("ICON")]
    pub male_inventory_image: Option<ZString>,
    #[record("MICO")]
    pub male_message_icon: Option<ZString>,
    #[record("MOD4")]
    pub female_world_model: Option<ZString>,
    #[record("ICO2")]
    pub female_inventory_image: Option<ZString>,
    #[record("MIC2")]
    pub female_message_icon: Option<ZString>,
    #[record("BOD2")]
    pub biped_body_template: ARMO_BOD2,
    #[record("DEST")]
    pub destruction_data: DEST,
    #[record("YNAM")]
    pub sound_pick_up: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("ETYP")]
    pub equipment_type: Option<FormId>,
    #[record("BIDS")]
    pub block_bash_impact_data: Option<FormId>,
    #[record("BAMT")]
    pub block_material: Option<FormId>,
    #[record("RNAM")]
    pub race: Option<FormId>,
    #[record("KWDA")]
    pub keywords: Option<Keywords>,
    #[record("DESC")]
    pub description: Option<LString>,
    #[record("INRD")]
    pub instance_naming: Option<FormId>,
    #[record("MODL")]
    pub models: SubRecordArray<ArmorModel>,
    #[record("DATA")]
    pub data: ArmorData,
    #[record("FNAM")]
    pub data2: ArmorData2,
    #[record("DAMA")]
    pub resistances: Option<Array<Resistance>>,
    #[record("TNAM")]
    pub template_armor: Option<FormId>,
    #[record("OBTS")]
    pub object_template: SubRecordArray<ObjectTemplateCombination>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorMaleModel {
    #[record("MOD2")]
    pub model: Option<ZString>,
    #[record("MO2T")]
    pub model_info: Option<ModelInfo>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorFemaleModel {
    #[record("MOD4")]
    pub model: Option<ZString>,
    #[record("MO4T")]
    pub model_info: Option<ModelInfo>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorModel {
    #[record("INDX")]
    pub addon_index: u16,
    #[record("MODL")]
    pub armor_addon: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ARMO_BOD2 {
    pub body_part_flags: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ArmorData {
    pub value: i32,
    pub weight: f32,
    pub health: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ArmorData2 {
    pub armor_rating: u16,
    pub base_addon_index: u16,
    pub stagger_rating: Stagger,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u8)]
pub enum Stagger {
    None = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
    ExtraLarge = 4,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Resistance {
    pub damage_type: FormId,
    pub value: u32,
}

#[derive(Clone, Debug, Record)]
pub struct ObjectTemplateCombination {
    #[record("OBTF")]
    pub editor_only: Option<()>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("OBTS")]
    pub item: ObjectModTemplateItem,
}

#[derive(Clone, Debug)]
pub struct ObjectModTemplateItem {
    pub level_min: u8,
    pub level_max: u8,
    pub parent_combination_index: i16,
    pub default: bool,
    pub keywords: Vec<FormId>,
    pub min_level_for_ranks: u8,
    pub alt_level_per_tier: u8,
    pub includes: Vec<ObjectModTemplateItemInclude>,
}

impl Decode for ObjectModTemplateItem {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let include_count = u32::decode(&mut buf)?;
        let property_count = u32::decode(&mut buf)?;
        let level_min = u8::decode(&mut buf)?;
        u8::decode(&mut buf)?;
        let level_max = u8::decode(&mut buf)?;
        u8::decode(&mut buf)?;
        let parent_combination_index = i16::decode(&mut buf)?;
        let default = bool::decode(&mut buf)?;

        let keyword_count = u8::decode(&mut buf)?;
        let keywords = (0..keyword_count)
            .map(|_| FormId::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        let min_level_for_ranks = u8::decode(&mut buf)?;
        let alt_level_per_tier = u8::decode(&mut buf)?;

        let includes = (0..include_count)
            .map(|_| ObjectModTemplateItemInclude::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        // TODO: Properties

        Ok(Self {
            level_min,
            level_max,
            parent_combination_index,
            default,
            keywords,
            min_level_for_ranks,
            alt_level_per_tier,
            includes,
        })
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ObjectModTemplateItemInclude {
    pub mod_: FormId,
    pub attach_point_index: u8,
    pub optional: bool,
    pub dont_use_all: bool,
}

#[derive(Clone, Debug, Record)]
pub struct Book {
    #[record("EDID")]
    pub edid: ZString,
    // TODO
    #[record("VMAD")]
    pub vmad: Option<()>,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: Option<LString>,
    // TODO
    #[record("MODL")]
    pub modl: Option<()>,
    #[record("ICON")]
    pub icon: Option<ZString>,
    #[record("MICO")]
    pub mico: Option<ZString>,
    #[record("DESC")]
    pub desc: LString,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("YNAM")]
    pub ynam: Option<FormId>,
    #[record("ZNAM")]
    pub znam: Option<FormId>,
    #[record("KSIZ")]
    pub ksiz: Option<u32>,
    // #[record("KWDA")]
    // pub kwda: Vec<FormId>,
    #[record("DATA")]
    pub data: Option<BOOK_DATA>,
    #[record("INAM")]
    pub inam: Option<FormId>,
    #[record("CNAM")]
    pub cnam: LString,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct BOOK_DATA {
    pub flags: u8,
    pub typ: u8,
    _pad0: [u8; 2],
    pub teaches: u32,
    // Only 8 bytes in record?
    // pub value: u32,
    // pub weight: f32,
}

/// A container represents an entity that holds items in its inventory.
#[derive(Clone, Debug, Record)]
pub struct Container {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("FULL")]
    pub full: Option<LString>,
    #[record("MODL")]
    pub modl: Model,
    #[record("COCT")]
    items_count: Option<u32>,
    #[record("CNTO")]
    pub items: Vec<ContainerItem>,
    // TODO: Add this
    // #[record("COED")]
    // pub coed: Vec<COED>,
    #[record("DATA")]
    pub data: ContainerData,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("FTYP")]
    pub ftyp: Option<()>,
    #[record("PRPS")]
    pub prps: Option<()>,
    #[record("NTRM")]
    pub ntrm: Option<()>,
    #[record("SNAM")]
    pub sound_open: Option<FormId>,
    #[record("QNAM")]
    pub sound_close: Option<FormId>,
    #[record("TNAM")]
    pub sound_take_all: Option<FormId>,
    #[record("ONAM")]
    pub filter_list: Option<FormId>,
}

/// An item reference contained in a [`Container`].
#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ContainerItem {
    pub item: FormId,
    pub count: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct COED {
    pub owner: FormId,
    pub value: i32,
    pub item_condition: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ContainerData {
    pub flags: ContainerDataFlags,
    pub weight: f32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ContainerDataFlags: u8 {
        const ALLOW_SOUNDS_WHEN_ANIMATION = 0x01;
        const RESPAWNS = 0x02;
        const SHOW_OWNER = 0x04;
    }
}

impl_decode_bitflags!(ContainerDataFlags: u8);

#[derive(Clone, Debug, Record)]
pub struct DOOR {
    #[record("EDID")]
    pub edid: ZString,
    #[record("VMAD")]
    pub vmad: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: Option<LString>,
    // TODOO
    #[record("MODL")]
    pub modl: Model,
    #[record("SNAM")]
    pub snam: Option<FormId>,
    #[record("ANAM")]
    pub anam: Option<FormId>,
    #[record("BNAM")]
    pub bnam: Option<FormId>,
    #[record("FNAM")]
    pub fnam: u8,
    #[record("TNAM")]
    pub tnam: Vec<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct INGR {
    #[record("EDID")]
    pub edid: ZString,
    #[record("VMAD")]
    pub vmad: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: LString,
    #[record("KSIZ")]
    pub ksiz: Option<u32>,
    #[record("KWDA")]
    pub kwda: Option<Keywords>,
    #[record("MODL")]
    pub modl: (),
    #[record("ICON")]
    pub icon: Option<ZString>,
    #[record("YNAM")]
    pub ynam: Option<FormId>,
    #[record("ZNAM")]
    pub znam: Option<FormId>,
    #[record("DATA")]
    pub data: INGR_DATA,
    #[record("ENIT")]
    pub enit: INGR_ENIT,
    #[record("EFID")]
    pub efid: Vec<FormId>,
    #[record("EFIT")]
    pub efit: Vec<INGR_EFIT>,
    // TODOO
    #[record("CTDA")]
    pub ctda: Vec<()>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Array<T>(pub Vec<T>);

impl<T> Decode for Array<T>
where
    T: Decode,
{
    type Error = <T as Decode>::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut arr = Vec::new();
        while buf.has_remaining() {
            arr.push(T::decode(&mut buf)?);
        }

        Ok(Self(arr))
    }
}

#[deprecated(note = "use Array<FormId> instead")]
#[derive(Clone, Debug)]
pub struct Keywords(Vec<FormId>);

impl Decode for Keywords {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut keywords = Vec::new();
        while buf.has_remaining() {
            let kw = FormId::decode(&mut buf)?;
            keywords.push(kw);
        }

        Ok(Self(keywords))
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct INGR_DATA {
    pub value: u32,
    pub weight: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct INGR_ENIT {
    pub ingredient_value: u32,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct INGR_EFIT {
    pub magnitude: f32,
    pub area_of_effect: u32,
    pub duration: u32,
}

/// A light-emitting entity.
#[derive(Clone, Debug, Record)]
pub struct Light {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("MODL")]
    pub model: Model,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,

    // PRPS
    /// The localized name of the record.
    #[record("FULL")]
    pub name: Option<LString>,
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    #[record("DATA")]
    pub data: LightData,
    #[record("FNAM")]
    pub fade_value: f32,
    // NAM0
    // LNAM
    // WGDR
    #[record("SNAM")]
    pub sound: Option<FormId>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LightData {
    pub time: i32,
    /// Light emission radius in game units.
    pub radius: u32,
    /// Color of the emitted light.
    pub color: RgbPadded<u8>,
    pub flags: LightDataFlags,
    pub falloff_exponent: f32,
    pub fov: f32,
    pub near_clip: f32,
    pub flicker_period: f32,
    pub flicker_intensity_amplitude: f32,
    pub flicker_movement_amplitude: f32,
    pub constant: f32,
    pub scalar: f32,
    #[default_value]
    pub exponent: f32,
    #[default_value]
    pub god_rays_near_clip: f32,
    #[default_value]
    pub value: u32,
    #[default_value]
    pub weight: f32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct LightDataFlags: u32 {
        const CAN_BE_CARRIED = 0x00000002;
        const FLICKER = 0x00000008;
        const OFF_BY_DEFAULT = 0x00000020;
        const PULSE = 0x00000080;
        /// The light is a shadow casting spotlight.
        const SHADOW_SPOTLIGHT = 0x00000400;
        const SHADOW_HEMISPHERE = 0x00000800;
        /// The light is a shadow casting point light.
        const SHADOW_OMNIDIRECTIONAL = 0x00001000;
        /// The light is a spotlight without casting shadows.
        const NON_SHADOW_SPOTLIGHT = 0x00004000;
        const NON_SPECULAR = 0x00008000;
        const ATTENUATION_ONLY = 0x00010000;
        const NON_SHADOW_BOX = 0x00020000;
        const IGNORE_ROUGHNESS = 0x00040000;
        const NO_RIM_LIGHTING = 0x00080000;
        const AMBIENT_ONLY = 0x00100000;
    }
}

impl_decode_bitflags!(LightDataFlags: u32);

/// A miscellaneous item.
#[derive(Clone, Debug, Record)]
pub struct MiscObject {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("PTRN")]
    pub preview_tranform: Option<FormId>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    /// The localized name of the record.
    #[record("FULL")]
    pub name: Option<LString>,
    /// The worldspace model of the record.
    #[record("MODL")]
    pub model: Model,
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("YNAM")]
    pub sound_pickup: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("DATA")]
    pub data: MiscObjectData,
    #[record("CVPA")]
    pub components: Option<Array<MiscObjectComponents>>,
    #[record("CDIX")]
    pub component_display_indices: Option<Array<u8>>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MiscObjectData {
    pub value: u32,
    pub weight: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MiscObjectComponents {
    pub component: FormId,
    pub count: u32,
}

#[derive(Clone, Debug, Record)]
pub struct STAT {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("MODL")]
    pub modl: Model,
    #[record("DNAM")]
    pub dnam: STAT_DNAM,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct STAT_DNAM {
    pub max_angle: f32,
    pub directional_material: FormId,
}

#[derive(Clone, Debug, Record)]
pub struct StaticCollection {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("MODL")]
    pub model: Model,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("FLTR")]
    pub filter: Option<ZString>,
    #[record("ONAM")]
    pub parts: SubRecordArray<StaticCollectionPart>,
}

#[derive(Clone, Debug, Record)]
pub struct StaticCollectionPart {
    #[record("ONAM")]
    pub static_item: FormId,
    #[record("DATA")]
    pub placements: Array<StaticCollectionPartPlacement>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct StaticCollectionPartPlacement {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub scale: f32,
}

#[derive(Clone, Debug, Record)]
pub struct MSTT {
    #[record("EDID")]
    pub edid: ZString,
    #[record("OBND")]
    pub obnd: Bounds,
    // TODO
    #[record("MODL")]
    pub model: Model,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("DATA")]
    pub data: u8,
    #[record("SNAM")]
    pub snam: Option<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct GRAS {
    #[record("EDID")]
    pub edid: ZString,
    #[record("OBND")]
    pub obnd: Bounds,
    // TODO
    #[record("MODL")]
    pub modl: Option<()>,
    #[record("DATA")]
    pub data: GRAS_DATA,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct GRAS_DATA {
    pub density: u8,
    pub min_slope: u8,
    pub max_slope: u8,
    _pad0: u8,
    pub distance_from_water: u16,
    _pad1: u16,
    pub water_distance_op: u32,
    pub position_range: f32,
    pub height_range: f32,
    pub color_range: f32,
    pub wave_period: f32,
    pub flags: u8,
    _pad2: u8,
    _pad3: u16,
}

#[derive(Clone, Debug, Record)]
pub struct FLOR {
    #[record("EDID")]
    pub edid: ZString,
    #[record("VMAD")]
    pub vmad: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub obnd: Bounds,
    #[record("FULL")]
    pub full: LString,
    // TODO
    #[record("MODL")]
    pub modl: Option<()>,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("KSIZ")]
    pub ksiz: Option<u32>,
    #[record("KWDA")]
    pub kwda: Option<Keywords>,
    #[record("PNAM")]
    pub pnam: u32,
    #[record("RNAM")]
    pub rnam: Option<LString>,
    #[record("FNAM")]
    pub fnam: u16,
    #[record("PFIG")]
    pub pfig: Option<FormId>,
    #[record("SNAM")]
    pub snam: Option<FormId>,
    #[record("PFPC")]
    pub pfpc: u32,
}

#[derive(Clone, Debug, Record)]
pub struct Furniture {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    // TODO
    #[record("DEST")]
    pub dest: Option<()>,
    #[record("KSIZ")]
    keyword_size: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Keywords>,
    #[record("PNAM")]
    pub pnam: u32,
    #[record("FNAM")]
    pub fnam: u16,
    #[record("KNAM")]
    pub knam: Option<FormId>,
    #[record("MNAM")]
    pub mnam: u32,
    // Size of 1?
    // #[record("WBDT")]
    // pub wbdt: FURN_WBDT,
    // TODO
    #[record("XMRK")]
    pub xmrk: Option<()>,
    #[record("ENAM")]
    pub enam: Vec<u32>,
    #[record("NAM0")]
    pub nam0: Vec<FURN_NAM0>,
    #[record("FNMK")]
    pub fnmk: Vec<FormId>,
    #[record("FNPR")]
    pub fnpr: Vec<FURN_FNPR>,
}

// #[derive(Copy, Clone, Debug, BinaryStruct)]
// pub struct FURN_WBDT {
//     pub workbench: u8,
//     pub actor_value_skill: u8,
// }

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FURN_NAM0 {
    _unknown0: u16,
    pub flags: u16,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FURN_FNPR {
    pub flags_marker_type: u16,
    pub flags_marker_entry_point: u16,
}

#[derive(Clone, Debug, Record)]
pub struct Weapon {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Option<Bounds>,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("STCP")]
    pub animation_sound: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    #[record("ICON")]
    pub inventory_icon: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    #[record("EITM")]
    pub object_effect: Option<FormId>,
    #[record("EAMT")]
    pub enchament_amount: Option<u16>,
    #[record("ETYP")]
    pub equipment_type: Option<FormId>,
    #[record("BIDS")]
    pub block_impact_data: Option<FormId>,
    // BAMT
    #[record("YNAM")]
    pub sound_pickup: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("DESC")]
    pub description: Option<LString>,
    #[record("INRD")]
    pub instance_naming: Option<FormId>,
    #[record("APPR")]
    pub attach_parent_slots: Option<Array<FormId>>,
    // OBTE
    #[record("DNAM")]
    pub data: WeaponData,
    // FNAM
    #[record("CRDT")]
    pub critical_data: Option<WeaponCriticalData>,
    #[record("INAM")]
    pub impact_data_set: Option<FormId>,
    #[record("LNAM")]
    pub npc_add_ammo_list: Option<FormId>,
    #[record("WAMD")]
    pub aim_model: Option<FormId>,
    #[record("WZMD")]
    pub zoom: Option<FormId>,
    #[record("CNAM")]
    pub template: Option<FormId>,
    #[record("DAMA")]
    pub damage_types: Option<Array<WeaponDamageType>>,
    // FLTR
    #[record("MASE")]
    pub melee_speed: Option<MeleeSpeed>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct WeaponData {
    pub ammo: FormId,
    pub speed: f32,
    pub reload_speed: f32,
    pub reach: f32,
    pub min_range: f32,
    pub max_range: f32,
    pub attack_delay: f32,
    _unknown0: u32,
    pub damage_out_of_range_mult: f32,
    pub on_hit: u32,
    pub skill: FormId,
    pub resist: FormId,
    pub flags: WeaponDataFlags,
    pub capacity: u16,
    pub animation_type: WeaponAnimationType,
    pub secondary_damage: f32,
    pub weight: f32,
    pub value: u32,
    pub base_damage: u16,
    pub sound_level: SoundLevel,
    pub sound_attack: FormId,
    pub sound_attack_2d: FormId,
    pub sound_attack_loop: FormId,
    pub sound_attack_fail: FormId,
    pub sound_idle: FormId,
    pub sound_equip: FormId,
    pub sound_unequip: FormId,
    pub sound_fast_equip: FormId,
    pub accuracy_bonus: u8,
    pub animation_attack_seconds: f32,
    _unknown1: u16,
    pub action_point_cost: f32,
    pub full_power_seconds: f32,
    pub min_power_per_shot: f32,
    pub stagger: Stagger,
    _unknown2: u32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct WeaponDataFlags: u32 {
        const PLAYER_ONLY = 0x00000001;
        const NPCS_USE_AMMO = 0x00000002;
        const NO_JAM_AFTER_RELOAD = 0x00000004;
        const CHARGING_RELOAD = 0x00000008;
        const MINOR_CRIME = 0x00000010;
        const FIXED_RANGE = 0x00000020;
        const NOT_USED_IN_NORMAL_COMBAT = 0x00000040;
        const CRIT_EFFECT_ON_DEATH = 0x00000100;
        const CHARGING_ATTACK = 0x00000200;
        const HOLD_INPUT_TO_POWER = 0x00000800;
        const NON_HOSTILE = 0x00001000;
        const BOUND_WEAPON = 0x00002000;
        const IGNORES_NORMAL_WEAPON_RESISTANCE = 0x00004000;
        const AUTOMATIC = 0x00008000;
        const REPEATABLE_SINGLE_FIRE = 0x00010000;
        const CANT_DROP = 0x00020000;
        const HIDE_BACKPACK = 0x00040000;
        const EMBEDDED_WEAPON = 0x00080000;
        const NOT_PLAYABLE = 0x00100000;
        const HAS_SCOPE = 0x00200000;
        const BOLT_ACTION = 0x00400000;
        const SECONDARY_WEAPON = 0x00800000;
        const DISABLE_SHELLS = 0x01000000;
    }
}

impl_decode_bitflags!(WeaponDataFlags: u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum WeaponAnimationType {
    HandToHandMelee = 0,
    OneHandSword = 1,
    OneHandDagger = 2,
    OneHandAxe = 3,
    OneHandMace = 4,
    TwoHandSword = 5,
    TwoHandAxe = 6,
    Bow = 7,
    Staff = 8,
    Gun = 9,
    Grenade = 10,
    Mine = 11,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeaponCriticalData {
    pub damage_mult: f32,
    pub damage_bonus: f32,
    pub effect: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeaponDamageType {
    pub typ: FormId,
    pub amount: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u32)]
pub enum MeleeSpeed {
    VerySlow = 0,
    Slow = 1,
    Medium = 2,
    Fast = 3,
    VeryFast = 4,
}

/// A Weapon Ammunition record.
#[derive(Clone, Debug, Record)]
#[non_exhaustive]
pub struct Ammo {
    /// The editor name of the record.
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    /// The localized name of the record.
    #[record("FULL")]
    pub full_name: Option<LString>,
    /// The worldspace model of the item.
    #[record("MODL")]
    pub model: Model,
    #[record("DEST")]
    pub destruction: DEST,
    /// The sound played when the item is picked up.
    ///
    /// Reference to a [`SoundDescriptor`].
    #[record("YNAM")]
    pub sound_pickup: Option<FormId>,
    /// The sound played when the item is dropped.
    ///
    /// Reference to a [`SoundDescriptor`].
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("DESC")]
    pub desc: Option<LString>,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    /// A list of keywords attached to the item.
    ///
    /// Reference to a [`Keyword`].
    #[record("KWDA")]
    pub keywords: Array<FormId>,
    #[record("DATA")]
    pub data: AmmoItemData,
    #[record("DNAM")]
    pub projectile_data: AmmoProjectileData,
    #[record("ONAM")]
    pub short_name: Option<LString>,
    #[record("NAM1")]
    pub casing_model: Option<ZString>,
    #[record("NAM2")]
    pub casing_model_info: Option<ModelInfo>,
}

/// Item related data about an [`Ammo`] record.
#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AmmoItemData {
    /// The value of the item.
    pub value: u32,
    /// The weight of the item inside an inventory.
    pub weight: f32,
}

/// Combat related data about an [`Ammo`] record.
#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AmmoProjectileData {
    /// The object spawned when the ammo is fired.
    pub projectile: FormId,
    pub flags: u32,
    /// The damage that the projectile deals on impact.
    pub damage: f32,
    pub health: u32,
}

#[derive(Clone, Debug, Record)]
pub struct DEST {}

#[derive(Clone, Debug, Record)]
pub struct Npc {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("ACBS")]
    pub base_stats: NpcBaseStats,
    #[record("SNAM")]
    pub factions: Vec<NpcFactionMember>,
    #[record("INAM")]
    pub death_item: Option<FormId>,
    #[record("VTCK")]
    pub voice_type: Option<FormId>,
    #[record("TPLT")]
    pub default_template: Option<FormId>,
    #[record("LTPT")]
    pub legendary_template: Option<FormId>,
    #[record("LTPC")]
    pub legendary_chance: Option<FormId>,
    #[record("TPTA")]
    pub template_actors: Option<NpcTemplateActors>,
    #[record("RNAM")]
    pub race: FormId,
    #[record("WNAM")]
    pub skin: Option<FormId>,
    #[record("ANAM")]
    pub far_away_model: Option<FormId>,
    #[record("ATKR")]
    pub attack_race: Option<FormId>,
    // attacks?
    #[record("SPOR")]
    pub spectator_override_package_list: Option<FormId>,
    #[record("OCOR")]
    pub observe_dead_body_override_package_list: Option<FormId>,
    #[record("GWOR")]
    pub guard_warn_override_package_list: Option<FormId>,
    #[record("ECOR")]
    pub combat_override_package_list: Option<FormId>,
    #[record("FCPL")]
    pub follower_command_package_list: Option<FormId>,
    #[record("RCLR")]
    pub follower_elevator_package_list: Option<FormId>,
    #[record("PRKZ")]
    perk_count: Option<u32>,
    #[record("PRKR")]
    pub perks: Vec<NpcPerk>,
    #[record("COCT")]
    count_of_container: Option<u32>,
    #[record("CNTO")]
    pub items: Vec<NpcContainerObject>,
    #[record("COED")]
    pub owner: Option<NpcOwner>,
    #[record("AIDT")]
    pub ai_data: Option<NpcAiData>,
    #[record("PKID")]
    pub ai_packages: Vec<FormId>,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Keywords>,
    // attach_parent_slot
    // object template
    #[record("CNAM")]
    pub class: Option<FormId>,
    // TODO: Need to figure out ordered/hested fields
    // This is also present in the object template section
    // #[record("FULL")]
    // pub full_name: Option<LString>,
    #[record("SHRT")]
    pub short_name: Option<LString>,
    #[record("DATA")]
    data: (),
    #[record("DNAM")]
    pub stats: NpcStats,
    #[record("PNAM")]
    pub head_parts: Vec<FormId>,
    #[record("HCLF")]
    pub hair_color: Option<FormId>,
    #[record("BCLF")]
    pub facial_hair_color: Option<FormId>,
    #[record("ZNAM")]
    pub combat_style: Option<FormId>,
    #[record("GNAM")]
    pub gift_filter: Option<FormId>,
    #[record("NAM5")]
    unknown0: u16,
    #[record("NAM6")]
    pub height_min: f32,
    #[record("NAM4")]
    pub height_max: f32,
    #[record("MWGT")]
    pub weight: NpcWeight,
    #[record("NAM8")]
    pub sound_level: SoundLevel,
    // actor sounds
    #[record("CSCR")]
    pub audio_template: Option<FormId>,
    #[record("DOFT")]
    pub default_outfit: Option<FormId>,
    #[record("SOFT")]
    pub sleep_outfit: Option<FormId>,
    #[record("DPLT")]
    pub default_package_list: Option<FormId>,
    #[record("CRIF")]
    pub crime_faction: Option<FormId>,
    #[record("FTST")]
    pub face_texture: Option<FormId>,
    #[record("QNAM")]
    pub skin_tone: Option<RgbPadded<f32>>,
    // face morphs
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NpcBaseStats {
    pub flags: NpcBaseFlags,
    pub xp_value_offset: i16,
    pub level: i16,
    pub calc_min_level: u16,
    pub calc_max_level: u16,
    pub disposition_base: i16,
    pub template_flags: u16,
    pub bleedout_override: u16,
    _unknown0: [u8; 2],
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NpcBaseFlags: u32 {
        const FEMALE = 0x01;
        const ESSENTIAL = 0x02;
        const IS_CHAR_GEN_FACE_PRESET = 0x04;
        const RESPAWN = 0x08;
        const AUTO_CALC_STATS = 0x10;
        const UNIQUE = 0x20;
        const DOESNT_AFFECT_STEALTH_METER = 0x40;
        const PC_LEVEL_MULT = 0x80;
        const AUDIO_TEMPLATE = 0x100;
        const PROTECTED = 0x800;
        const GHOST = 0x20000000;
        const INVULNERABLE = 0x80000000;
    }
}

impl_decode_bitflags! {
    NpcBaseFlags: u32
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct NpcFactionMember {
    pub faction: FormId,
    pub rank: i8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct NpcTemplateActors {
    pub traits: FormId,
    pub stats: FormId,
    pub factions: FormId,
    pub spell_list: FormId,
    pub ai_data: FormId,
    pub ai_packages: FormId,
    pub model: FormId,
    pub base_data: FormId,
    pub inventory: FormId,
    pub script: FormId,
    pub def_pack_list: FormId,
    pub attack_data: FormId,
    pub keywords: FormId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct NpcPerk {
    pub perk: FormId,
    pub rank: i8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct NpcContainerObject {
    pub item: FormId,
    pub count: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, BinaryStruct)]
pub struct NpcOwner {
    pub owner: FormId,
    pub value: u32,
    pub item_condition: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NpcAiData {
    pub aggression: NpcAiAggression,
    pub confidence: NpcAiConfidence,
    pub energy: u8,
    pub morality: NpcAiMorality,
    pub mood: NpcAiMood,
    pub flags: NpcAiDataFlags,
    _unknown0: u8,
    pub warn: u32,
    pub warn_attack: u32,
    pub attack: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum NpcAiAggression {
    Unaggressive = 0x00,
    Aggressive = 0x01,
    VeryAggressive = 0x02,
    Frenzied = 0x03,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum NpcAiConfidence {
    Cowardly = 0x00,
    Cautious = 0x01,
    Average = 0x02,
    Brave = 0x03,
    Foolhardy = 0x04,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum NpcAiMorality {
    AnyCrime = 0x00,
    ViolenceAgainstEnemies = 0x01,
    PropertyCrime = 0x02,
    NoCrime = 0x03,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum NpcAiMood {
    Neutral = 0x00,
    Angry = 0x01,
    Fear = 0x02,
    Happy = 0x03,
    Sad = 0x04,
    Surprised = 0x05,
    Puzzled = 0x06,
    Disgusted = 0x07,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum NpcAiAssistance {
    HelpsNobody = 0x00,
    HelpsAllies = 0x01,
    HelpsFriendsAndAllies = 0x02,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NpcAiDataFlags: u8 {
        const AGGRO_RADIUS_BEHAVIOR = 0x01;
    }
}

impl_decode_bitflags! { NpcAiDataFlags: u8 }

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NpcStats {
    pub calculated_health: u16,
    pub calculated_action_points: u16,
    pub far_away_model_distance: u16,
    pub geared_up_weapons: u8,
    _pad0: [u8; 1],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum SoundLevel {
    Loud = 0,
    Normal = 1,
    Silent = 2,
    VeryLoud = 3,
    Quiet = 4,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NpcWeight {
    pub thin: f32,
    pub muscular: f32,
    pub fat: f32,
}

#[derive(Clone, Debug, Record)]
pub struct LeveledActor {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("LVLD")]
    pub chance_none: u8,
    #[record("LVLM")]
    pub max_count: u8,
    #[record("LVLF")]
    pub flags: LeveledActorFlags,
    #[record("LVLG")]
    pub use_global: Option<FormId>,
    #[record("LLCT")]
    list_count: u8,
    #[record("LVLO")]
    pub entries: Vec<LeveledListEntry>,
    #[record("COED")]
    pub extra_data: Vec<NpcOwner>,
    #[record("LLKC")]
    pub filter_keyword_chances: Option<FilterKeywordChances>,
    // TODO
    // model
}

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct LeveledActorFlags: u8 {
        const ALL_LEVELS = 0x01;
        const EACH = 0x02;
    }
}

impl_decode_bitflags!(LeveledActorFlags: u8);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LeveledListEntry {
    pub level: u32,
    pub reference: FormId,
    pub count: u32,
    // pub chance_none: u8,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FilterKeywordChances {
    pub keyword: FormId,
    pub chance: u32,
}

#[derive(Clone, Debug, Record)]
pub struct Key {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("FULL")]
    pub full: Option<LString>,
    //model
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    //DEST
    #[record("YNAM")]
    pub sound_pick_up: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Keywords>,
    #[record("DATA")]
    pub data: KeyData,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct KeyData {
    pub value: u32,
    pub weight: f32,
}

#[derive(Clone, Debug, Record)]
pub struct Ingestible {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("FULL")]
    pub name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Keywords>,
    //model
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    #[record("YNAM")]
    pub sound_pick_up: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("ETYP")]
    pub equipment_type: Option<FormId>,
    #[record("CUSD")]
    pub sound_crafting: Option<FormId>,
    // DEST
    #[record("DESC")]
    pub description: Option<LString>,
    #[record("DATA")]
    pub weight: f32,
    #[record("ENIT")]
    pub effect_data: EffectData,
    #[record("DNAM")]
    pub addiction_name: Option<LString>,
    //effects
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct EffectData {
    pub value: i32,
    pub flags: EffectDataFlags,
    pub addiction: FormId,
    pub addiction_chance: f32,
    pub sound_consume: FormId,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct EffectDataFlags: u32 {
        const NO_AUTO_CALC = 0x00001;
        const FOOD = 0x00002;
        const MEDICINE = 0x10000;
        const POISON = 0x20000;
    }
}

impl_decode_bitflags!(EffectDataFlags: u32);

#[derive(Clone, Debug, Record)]
pub struct IdleMarker {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("KSIZ")]
    keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Vec<FormId>,
    #[record("IDLF")]
    pub flags: Option<IdleMarkerFlags>,
    #[record("IDLC")]
    idle_animation_count: Option<u8>,
    #[record("IDLT")]
    pub idle_timer_setting: Option<f32>,
    #[record("IDLA")]
    pub idle_animations: Option<Array<FormId>>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct IdleMarkerFlags: u8 {
        const RUN_IN_SEQUENCE = 0x01;
        const DO_ONCE = 0x04;
        const IGNORED_BY_SANDBOX = 0x10;
    }
}

impl_decode_bitflags!(IdleMarkerFlags: u8);

#[derive(Clone, Debug, Record)]
pub struct Note {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    // model
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("YNAM")]
    pub sound_pick_up: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("DNAM")]
    pub typ: NoteType,
    #[record("DATA")]
    pub data: NoteData,
    /// Sound,Scene or Terminal based on NoteType
    #[record("SNAM")]
    pub sound_scene_terminal: Option<FormId>,
    // program file
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum NoteType {
    Sound = 0,
    Voice = 1,
    Program = 2,
    Terminal = 3,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NoteData {
    pub value: u32,
    pub weight: f32,
}

#[derive(Clone, Debug, Record)]
pub struct Projectile {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("FULL")]
    pub full_name: Option<LString>,
    //model
    #[record("DNAM")]
    pub data: ProjectileData,
    // muzzle flash data
    #[record("VNAM")]
    pub sound_detection_level: SoundLevel,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ProjectileData {
    pub flags: ProjectileFlags,
    pub ty: ProjectileType,
    pub gravity: f32,
    pub speed: f32,
    pub range: f32,
    pub light: FormId,
    pub muzzle_flash_light: FormId,
    pub explosion_alt_trigger_proximity: f32,
    pub explosion_alt_trigger_timer: f32,
    pub explosion: FormId,
    pub sound: FormId,
    pub muzzle_flash_duration: f32,
    pub fade_duration: f32,
    pub impact_force: f32,
    pub sound_countdown: FormId,
    pub sound_disable: FormId,
    pub default_weapon_source: FormId,
    pub cone_spread: f32,
    pub collision_radius: f32,
    pub lifetime: f32,
    pub relaunch_interval: f32,
    pub decal_data: FormId,
    pub collision_layer: FormId,
    pub tracer_frequency: u8,
    pub vats_projectile: FormId,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ProjectileFlags: u16 {
        const HITSCAN = 0x0001;
        const EXPLOSION = 0x0002;
        const ALT_TRIGGER = 0x0004;
        const MUZZLE_FLASH = 0x0008;
        const CAN_BE_DISABLED = 0x0020;
        const CAN_BE_PICKED_UP = 0x0040;
        const SUPERSONIC = 0x0080;
        const PINS_LIMBS = 0x0100;
        const PASS_THROUGH_SMALL_TRANSPARENT = 0x0200;
        const DISABLE_COMBAT_AIN_CORRECTION = 0x0400;
        const PENETRATES_GEOMETRY = 0x0800;
        const CONTINOUS_UPDATE = 0x1000;
        const SEEKS_TARGET = 0x2000;
    }
}

impl_decode_bitflags!(ProjectileFlags: u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u16)]
pub enum ProjectileType {
    Missle = 0x01,
    Lobber = 0x02,
    Beam = 0x04,
    Flame = 0x08,
    Cone = 0x10,
    Barrier = 0x20,
    Arrow = 0x40,
}

#[derive(Clone, Debug, Record)]
pub struct Hazard {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("FULL")]
    pub full_name: Option<LString>,
    //modl
    #[record("MNAM")]
    pub image_space_modifier: Option<FormId>,
    #[record("DNAM")]
    pub data: HazardData,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct HazardData {
    pub limit: u32,
    pub radius: f32,
    pub lifetime: f32,
    pub image_space_radius: f32,
    pub target_interval: f32,
    pub flags: HazardFlags,
    pub effect: FormId,
    pub light: FormId,
    pub impact_data_set: FormId,
    pub sound: FormId,
    pub taper_effectiveness: HazardTaperEffectiveness,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct HazardFlags: u32 {
        const AFFECTS_PLAYER_ONLY = 0x01;
        const INHERIT_DURATION_FROM_SPAWN_SPELL = 0x02;
        const ALIGN_TO_IMPACT_NORMAL = 0x04;
        const INHERIT_RADIUS_FROM_SPAWN_SPELL = 0x08;
        const DROP_TO_GROUND = 0x10;
        const TAPER_EFFECTIVENESS_BY_PROXIMITY = 0x20;
    }
}

impl_decode_bitflags!(HazardFlags: u32);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct HazardTaperEffectiveness {
    pub full_effect_radius: f32,
    pub taper_weight: f32,
    pub taper_curse: f32,
}

#[derive(Clone, Debug, Record)]
pub struct BlendableSpline {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("OBND")]
    pub object_bounds: Option<Bounds>,
    #[record("DNAM")]
    pub data: BlendableSplineData,
    #[record("TNAM")]
    pub texture: Option<FormId>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct BlendableSplineData {
    pub default_number_of_tiles: f32,
    pub default_number_of_slices: u16,
    pub default_number_of_tiles_relative_to_length: u16,
    pub default_color: Rgb<f32>,
    pub wind_settings_sensitivity: f32,
    pub wind_settings_flexibility: f32,
}

#[derive(Clone, Debug, Record)]
pub struct Terminal {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Option<Bounds>,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("NAM0")]
    pub header_text: Option<LString>,
    #[record("WNAM")]
    pub welcome_text: Option<LString>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    // PRPS
    #[record("SNAM")]
    pub sound_looping: Option<FormId>,
    #[record("COCT")]
    _holotape_count: Option<u32>,
    #[record("CNTO")]
    pub holotapes: Option<Array<TerminalHolotape>>,
    // BTXT
    // ITXT
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct TerminalHolotape {
    pub item: FormId,
    pub count: i32,
}

#[derive(Clone, Debug, Record)]
pub struct Tree {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Option<Bounds>,
    #[record("MODL")]
    pub model: Model,
    #[record("PFIG")]
    pub ingredient: Option<FormId>,
    #[record("SNAM")]
    pub harvest_sound: Option<FormId>,
    #[record("PFPC")]
    pub ingredient_production: Option<TreeIngredientProduction>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("CNAM")]
    pub data: TreeData,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct TreeIngredientProduction {
    pub spring: u8,
    pub summer: u8,
    pub fall: u8,
    pub winter: u8,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct TreeData {
    pub trunk_flexibility: f32,
    pub branch_flexibility: f32,
    pub trunk_amplitude: f32,
    pub front_amplitude: f32,
    pub back_amplitude: f32,
    pub side_amplitude: f32,
    pub front_frequency: f32,
    pub back_frequency: f32,
    pub side_frequency: f32,
    pub leaf_flexibility: f32,
    pub leaf_amplitude: f32,
    pub leaf_frequency: f32,
}

#[derive(Clone, Debug, Record)]
pub struct LeveledItem {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("LVLD")]
    pub chance_none: u8,
    #[record("LVLM")]
    pub max_count: Option<u8>,
    #[record("LVLF")]
    pub flags: LeveledActorFlags,
    #[record("LVLG")]
    pub use_global: Option<FormId>,
    #[record("LLCT")]
    leveled_list_entries_count: Option<u8>,
    #[record("LVLO")]
    pub entries: Vec<LeveledListEntry>,
    //COED
    #[record("LLKC")]
    pub filter_keyword_chances: Option<FilterKeywordChances>,
    #[record("LVSG")]
    pub epic_loot_chance: Option<FormId>,
    #[record("ONAM")]
    pub override_name: Option<LString>,
}

#[derive(Clone, Debug, Record)]
pub struct Weather {
    #[record("EDID")]
    pub id: ZString,
    #[record("00TX")]
    pub cloud_texture_layer00: Option<ZString>,
    #[record("10TX")]
    pub cloud_texture_layer01: Option<ZString>,
    #[record("20TX")]
    pub cloud_texture_layer02: Option<ZString>,
    #[record("30TX")]
    pub cloud_texture_layer03: Option<ZString>,
    #[record("40TX")]
    pub cloud_texture_layer04: Option<ZString>,
    #[record("50TX")]
    pub cloud_texture_layer05: Option<ZString>,
    #[record("60TX")]
    pub cloud_texture_layer06: Option<ZString>,
    #[record("70TX")]
    pub cloud_texture_layer07: Option<ZString>,
    #[record("80TX")]
    pub cloud_texture_layer08: Option<ZString>,
    #[record("90TX")]
    pub cloud_texture_layer09: Option<ZString>,
    #[record(":0TX")]
    pub cloud_texture_layer10: Option<ZString>,
    #[record(";0TX")]
    pub cloud_texture_layer11: Option<ZString>,
    #[record("<0TX")]
    pub cloud_texture_layer12: Option<ZString>,
    #[record("=0TX")]
    pub cloud_texture_layer13: Option<ZString>,
    #[record(">0TX")]
    pub cloud_texture_layer14: Option<ZString>,
    #[record("?0TX")]
    pub cloud_texture_layer15: Option<ZString>,
    #[record("@0TX")]
    pub cloud_texture_layer16: Option<ZString>,
    #[record("A0TX")]
    pub cloud_texture_layer17: Option<ZString>,
    #[record("B0TX")]
    pub cloud_texture_layer18: Option<ZString>,
    #[record("C0TX")]
    pub cloud_texture_layer19: Option<ZString>,
    #[record("D0TX")]
    pub cloud_texture_layer20: Option<ZString>,
    #[record("E0TX")]
    pub cloud_texture_layer21: Option<ZString>,
    #[record("F0TX")]
    pub cloud_texture_layer22: Option<ZString>,
    #[record("G0TX")]
    pub cloud_texture_layer23: Option<ZString>,
    #[record("H0TX")]
    pub cloud_texture_layer24: Option<ZString>,
    #[record("I0TX")]
    pub cloud_texture_layer25: Option<ZString>,
    #[record("J0TX")]
    pub cloud_texture_layer26: Option<ZString>,
    #[record("K0TX")]
    pub cloud_texture_layer27: Option<ZString>,
    #[record("L0TX")]
    pub cloud_texture_layer28: Option<ZString>,
    #[record("M0TX")]
    pub cloud_texture_layer29: Option<ZString>,
    #[record("N0TX")]
    pub cloud_texture_layer30: Option<ZString>,
    #[record("O0TX")]
    pub cloud_texture_layer31: Option<ZString>,
    #[record("MNAM")]
    pub precipitation_type: FormId,
    #[record("NNAM")]
    pub visual_effect: FormId,
    #[record("RNAM")]
    pub cloud_speed_y: CloudSpeed,
    #[record("QNAM")]
    pub cloud_speed_x: CloudSpeed,
    #[record("PNAM")]
    pub cloud_texture_color: [TimeOfDaySet<Rgba<u8>>; 32],
    #[record("JNAM")]
    pub cloud_texture_alpha: [TimeOfDaySet<f32>; 32],
    #[record("NAM0")]
    pub colors: WeatherColors,
    #[record("FNAM")]
    pub fog_distance: WeatherFogDistance,
    #[record("DATA")]
    pub data: WeatherData,
    #[record("NAM1")]
    pub disabled_cloud_layers: Option<u32>,
    #[record("SNAM")]
    pub ambient_sounds: Vec<AmbientSound>,
    #[record("TNAM")]
    pub sky_statics: Vec<FormId>,
    #[record("IMSP")]
    pub image_spaces: TimeOfDaySet<FormId>,
    #[record("WGDR")]
    pub god_rays: Option<TimeOfDaySetExtended<FormId>>,
    /// Order:
    /// - Sunrise
    /// - Day
    /// - Sunset
    /// - Night
    /// - Early Sunrise
    /// - Late Sunrise
    /// - Early Sunset
    /// - Late Sunset
    #[record("DALC")]
    pub lighting_data: Vec<WeatherLightingData>,
    // model (aurora/radstorm)
    #[record("UNAM")]
    pub magic: Option<WeatherMagic>,
    #[record("VNAM")]
    pub volatility_mult: Option<f32>,
    #[record("WNAM")]
    pub visibility_mult: Option<f32>,
}

#[derive(Copy, Clone, Debug)]
pub struct CloudSpeed(pub [f32; 32]);

impl Decode for CloudSpeed {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        <[u8; 32]>::decode(buf).map(|arr| {
            Self(arr.map(|byte| {
                // Lerp between 0x00 => -0.1 and 0xfe => 0.1
                glam::FloatExt::lerp(-0.1, 0.1, byte as f32 / 0xfe as f32)
            }))
        })
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct TimeOfDaySet<T> {
    pub sunrise: T,
    pub day: T,
    pub sunset: T,
    pub night: T,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct TimeOfDaySetExtended<T> {
    pub sunrise: T,
    pub day: T,
    pub sunset: T,
    pub night: T,
    pub early_sunrise: T,
    pub late_sunrise: T,
    pub early_sunset: T,
    pub late_sunset: T,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeatherColors {
    pub sky_upper: TimeOfDaySet<Rgba<u8>>,
    pub fog_near: TimeOfDaySet<Rgba<u8>>,
    _unknown0: TimeOfDaySet<Rgba<u8>>,
    pub ambient: TimeOfDaySet<Rgba<u8>>,
    pub sunlight: TimeOfDaySet<Rgba<u8>>,
    pub sun: TimeOfDaySet<Rgba<u8>>,
    pub stars: TimeOfDaySet<Rgba<u8>>,
    pub sky_lower: TimeOfDaySet<Rgba<u8>>,
    pub horizon: TimeOfDaySet<Rgba<u8>>,
    pub effect_lightning: TimeOfDaySet<Rgba<u8>>,
    pub cloud_lod_diffuse: TimeOfDaySet<Rgba<u8>>,
    pub cloud_lod_ambient: TimeOfDaySet<Rgba<u8>>,
    pub fog_far: TimeOfDaySet<Rgba<u8>>,
    pub sky_statics: TimeOfDaySet<Rgba<u8>>,
    pub water_multiplier: TimeOfDaySet<Rgba<u8>>,
    pub sun_glare: TimeOfDaySet<Rgba<u8>>,
    pub moon_glare: TimeOfDaySet<Rgba<u8>>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeatherFogDistance {
    pub day_near: f32,
    pub day_far: f32,
    pub night_near: f32,
    pub night_far: f32,
    pub day_power: f32,
    pub night_power: f32,
    pub day_max: f32,
    pub night_max: f32,
}

/// FIXME: Most of the u8s are quantized floats
#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeatherData {
    pub wind_speed: u8,
    _unknown0: u8,
    pub trans_delta: u8,
    pub sun_glare: u8,
    pub sun_damage: u8,
    pub precipitation_begin_fade_in: u8,
    pub precipitation_end_fade_out: u8,
    pub thunder_lightning_begin_fade_in: u8,
    pub thunder_lightning_end_fade_out: u8,
    pub thunder_lightning_frequency: u8,
    pub flags: WeatherFlags,
    pub lightning_color: RgbPadded<u8>,
    pub visual_effect_begin: u8,
    pub visual_effect_end: u8,
    pub wind_direction: u8,
    pub wind_dir_range: u8,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct WeatherFlags: u8 {
        const PLEASANT = 0x01;
        const CLOUDY = 0x02;
        const RAINY = 0x04;
        const SNOW = 0x08;
        const ALWAYS_VISIBLE = 0x10;
        const FOLLOWS_SUN_POSITION = 0x20;
    }
}

impl_decode_bitflags!(WeatherFlags: u8);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AmbientSound {
    pub sound: FormId,
    pub ty: AmbientSoundType,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u16)]
pub enum AmbientSoundType {
    Default = 0,
    Precip = 1,
    Wine = 2,
    Thunder = 3,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeatherLightingData {
    pub directional_ambient_x_up: Rgba<u8>,
    pub directional_ambient_x_down: Rgba<u8>,
    pub directional_ambient_y_up: Rgba<u8>,
    pub directional_ambient_y_down: Rgba<u8>,
    pub directional_ambient_z_up: Rgba<u8>,
    pub directional_ambient_z_down: Rgba<u8>,
    pub specular_color: Rgba<u8>,
    pub fresnel_power: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeatherMagic {
    pub on_lightning_strike_spell: FormId,
    pub on_lightning_strike_threshold: f32,
    pub on_weather_activate_spell: FormId,
    // Only present on some weathers
    // pub on_weather_activate_threshold: f32,
}

#[derive(Clone, Debug, Record)]
pub struct Climate {
    #[record("EDID")]
    pub id: ZString,
    #[record("WLST")]
    pub weathers: Array<ClimateWeather>,
    #[record("FNAM")]
    pub sun_texture: Option<ZString>,
    #[record("GNAM")]
    pub sun_glare_texture: Option<ZString>,
    //model
    #[record("TNAM")]
    pub timing: ClimateTiming,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ClimateWeather {
    pub weather: FormId,
    pub chance: u32,
    pub global: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ClimateTiming {
    pub sunrise_begin: u8,
    pub sunrise_end: u8,
    pub sunset_begin: u8,
    pub sunset_end: u8,
    pub volatility: u8,
    pub moons: u8,
}

#[derive(Clone, Debug, Record)]
pub struct ShaderParticleGeometry {
    #[record("EDID")]
    pub id: ZString,
    #[record("DATA")]
    pub data: ShaderParticleGeometryData,
    #[record("MNAM")]
    pub particle_texture: ZString,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ShaderParticleGeometryData {
    pub gravity_velocity: f32,
    pub rotation_velocity: f32,
    pub particle_size_x: f32,
    pub particle_size_y: f32,
    pub center_offset_min: f32,
    pub center_offset_max: f32,
    pub initial_rotation_range: f32,
    pub num_of_subtextures_x: u32,
    pub num_of_subtextures_y: u32,
    pub shader_type: u32,
    pub box_size: u32,
    pub particle_density: f32,
}

#[derive(Clone, Debug, Record)]
pub struct VisualEffect {
    #[record("EDID")]
    pub id: ZString,
    #[record("DATA")]
    pub data: VisualEffectData,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct VisualEffectData {
    pub effect_art: FormId,
    pub shader: FormId,
    pub flags: VisualEffectFlags,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct VisualEffectFlags: u32 {
        const ROTATE_TO_FACE_TARGET = 0x01;
        const ATTACH_TO_CAMERA = 0x02;
        const INHERIT_ROTATION = 0x04;
    }
}

impl_decode_bitflags!(VisualEffectFlags: u32);

#[derive(Clone, Debug, Record)]
pub struct Region {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("RCLR")]
    pub map_color: Option<Rgb<u8>>,
    #[record("WNAM")]
    pub worldspace: Option<FormId>,
    // RPLI
    // RDAT
}

#[derive(Clone, Debug, Record)]
pub struct NAVI {}

#[derive(Clone, Debug, Record)]
pub struct Cell {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("DATA")]
    pub flags: CellFlags,
    #[record("XCLC")]
    pub grid: Option<Grid>,
    #[record("XCLL")]
    pub lighting: Option<CellLighting>,
    // max_height_data
    #[record("LTMP")]
    pub lighting_template: FormId,
    #[record("XCLW")]
    pub water_height: f32,
    #[record("XCLR")]
    pub regions: Option<Array<FormId>>,
    #[record("XLCN")]
    pub location: Option<FormId>,
    #[record("XWCU")]
    pub water_velocity: Option<Array<Vec3Padded<f32>>>,
    #[record("XCWT")]
    pub water: Option<FormId>,
    #[record("XOWN")]
    pub owner: Option<FormId>,
    //owner faction rank
    #[record("XILL")]
    pub lock_list: Option<FormId>,
    // exterior lod
    // water environment map
    #[record("XCCM")]
    pub weather_from_region: Option<FormId>,
    #[record("XCAS")]
    pub acoustic_space: Option<FormId>,
    #[record("XEZN")]
    pub encounter_zone: Option<FormId>,
    #[record("XCMO")]
    pub music_type: Option<FormId>,
    #[record("XCIM")]
    pub image_space: Option<FormId>,
    // god rays
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct CellFlags: u16 {
        const INTERIOR = 0x0001;
        const HAS_WATER = 0x0002;
        const CAN_TRAVEL_FROM_HERE = 0x0004;
        const NO_LOD_WATER = 0x0008;
        const PUBLIC_AREA = 0x0020;
        const HAND_CHANGED = 0x0040;
        const SHOW_SKY = 0x0080;
        const USE_SKY_LIGHTING = 0x0100;
        const HIDDEN_FROM_INTERIOR_CELL_LIST = 0x0400;
        const SUNLIGHT_SHADOWS = 0x0800;
        const DISTANT_LOD_ONLY = 0x1000;
        const PLAYER_FOLLOWERS_CANT_TRAVEL_HERE = 0x2000;
    }
}

impl_decode_bitflags!(CellFlags: u16);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Grid {
    pub x: i32,
    pub y: i32,
    pub flags: GridFlags,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct GridFlags: u32 {
        const FORCE_HIDE_LAND_QUAD1 = 0x1;
        const FORCE_HIDE_LAND_QUAD2 = 0x2;
        const FORCE_HIDE_LAND_QUAD3 = 0x4;
        const FORCE_HIDE_LAND_QUAD4 = 0x8;
    }
}

impl_decode_bitflags!(GridFlags: u32);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct CellLighting {
    pub ambient: RgbPadded<u8>,
    pub directional: RgbPadded<u8>,
    pub fog_color_near: RgbPadded<u8>,
    pub fog_near: f32,
    pub fog_far: f32,
    pub directional_rotation_xy: i32,
    pub directional_rotation_z: i32,
    pub directional_fade: f32,
    pub fog_clip_distance: f32,
    pub fog_power: f32,
    pub ambient_x_up: RgbPadded<u8>,
    pub ambient_x_down: RgbPadded<u8>,
    pub ambient_y_up: RgbPadded<u8>,
    pub ambient_y_down: RgbPadded<u8>,
    pub ambient_z_up: RgbPadded<u8>,
    pub ambient_z_down: RgbPadded<u8>,
    pub specular_color: RgbPadded<u8>,
    pub scale: f32,
    pub fog_color_far: RgbPadded<u8>,
    pub fog_max: f32,
    pub light_fade_begin: f32,
    pub light_fade_end: f32,
    pub inherit_flags: CellLightingInheritFlags,
    // Following fields are missing sometimes?
    // pub near_height_mid: f32,
    // pub near_height_range: f32,
    // pub fog_color_high_near: Rgb<u8>,
    // pub fog_color_high_far: Rgb<u8>,
    // pub high_density_scale: f32,
    // pub fog_near_scale: f32,
    // pub fog_far_scale: f32,
    // pub fog_high_near_scale: f32,
    // pub fog_high_far_scale: f32,
    // pub far_height_mid: f32,
    // pub far_height_range: f32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct CellLightingInheritFlags: u32 {
        const AMBIENT_COLOR = 0x0001;
        const DIRECTIONAL_COLOR = 0x0002;
        const FOG_COLOR = 0x0004;
        const FOG_NEAR = 0x0008;
        const FOG_FAR = 0x0010;
        const DIRECTIONAL_ROT = 0x0020;
        const DIRECTIONAL_FADE = 0x0040;
        const CLIP_DISTANCE = 0x0080;
        const FOG_POWER = 0x0100;
        const FOG_MAX = 0x0200;
        const LIGHT_FADE_DISTANCES = 0x0400;
    }
}

impl_decode_bitflags!(CellLightingInheritFlags: u32);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Vec3Padded<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    _pad: T,
}

#[derive(Clone, Debug, Record)]
pub struct PlacedNpc {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("NAME")]
    pub base_npc: FormId,
    #[record("XEZN")]
    pub encounter_zone: Option<FormId>,
    // ragdoll
    #[record("XLCM")]
    pub level_modifier: Option<LevelModifier>,
    #[record("XCNT")]
    pub count: Option<i32>,
    #[record("XRDS")]
    pub radius: Option<f32>,
    #[record("XHLT")]
    pub health: Option<u32>,
    #[record("XLKR")]
    pub linked_references: Vec<LinkedReference>,
    #[record("XAPD")]
    pub activation_parent_flags: Option<ActivationParentFlags>,
    /// Is this optional or vec?
    #[record("XAPR")]
    pub activate_parent: Vec<ActivateParent>,
    // some things missing
    #[record("DATA")]
    pub position: PositionRotation,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u32)]
pub enum LevelModifier {
    Easy = 0,
    Medium = 1,
    Hard = 2,
    VeryHard = 3,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LinkedReference {
    pub keyword: FormId,
    pub reference: FormId,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ActivationParentFlags: u8 {
        const PARENT_ACTIVATE_ONLY = 0x01;
    }
}

impl_decode_bitflags!(ActivationParentFlags: u8);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ActivateParent {
    pub form: FormId,
    pub delay: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PositionRotation {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
}

#[derive(Clone, Debug, Record)]
pub struct PlacedObject {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("NAME")]
    pub base_object: FormId,
    #[record("XMBO")]
    pub bound_half_extents: Option<[f32; 3]>,
    #[record("XPRM")]
    pub primitive: Option<PlacedObjectPrimitive>,
    #[record("XPOD")]
    pub portal_destination: Option<PortalDestination>,
    // XRMR
    #[record("XEMI")]
    pub emitted_light: Option<FormId>,
    #[record("XRDS")]
    pub radius: Option<f32>,
    #[record("XLIG")]
    pub light_data: Option<PlacedObjectLightData>,
    #[record("XALP")]
    pub alpha_cutoff: Option<AlphaCutoff>,
    #[record("XTEL")]
    pub door_teleport: Option<DoorTeleport>,
    #[record("XTNM")]
    pub teleport_location_name: Option<FormId>,
    #[record("XMBR")]
    pub multibound_reference: Option<FormId>,
    #[record("XWCU")]
    pub water_velocity: Option<WaterVelocity>,
    #[record("XASP")]
    pub acoustic_restriction: Option<FormId>,
    #[record("XATP")]
    pub activation_point: Option<()>,
    #[record("XAMC")]
    pub ammo_count: Option<u32>,
    #[record("XLKT")]
    pub linked_ref_transient: Option<()>,
    #[record("XLYR")]
    pub layer: Option<FormId>,
    #[record("XMSP")]
    pub material_swap: Option<FormId>,
    #[record("XRFG")]
    pub reference_group: Option<FormId>,
    #[record("XRDO")]
    pub radio: Option<Radio>,
    #[record("XBSD")]
    pub spline: Option<Spline>,
    #[record("XPDD")]
    pub projected_decal: Option<ProjectedDecal>,
    #[record("XSPC")]
    pub spawn_container: Option<FormId>,
    #[record("XAPD")]
    pub activate_parents: Option<ActivateParentsFlags>,
    #[record("XAPR")]
    pub activate_parent_refs: Option<Array<ActivateParentRefs>>,
    #[record("XLIB")]
    pub leveled_base_item_object: Option<FormId>,
    // XLCM
    #[record("XLCN")]
    pub persist_location: Option<FormId>,
    #[record("XTRI")]
    pub collision_layer: Option<u32>,
    #[record("XLOC")]
    pub lock_data: Option<LockData>,
    #[record("XEZN")]
    pub encounter_zone: Option<FormId>,
    // XNDP
    #[record("XLRL")]
    pub location_reference: Option<FormId>,
    #[record("XLRT")]
    pub location_ref_type: Vec<FormId>,
    #[record("XIS2")]
    pub ignored_by_sandbox: Option<()>,
    // OWNERSHIP
    #[record("XCNT")]
    pub item_count: Option<i32>,
    #[record("XHLT")]
    pub health_percentage: Option<u32>,
    #[record("XESP")]
    pub enable_parent: Option<EnableParent>,
    // XLKR
    // things missing
    #[record("XSCL")]
    pub scale: Option<f32>,
    #[record("DATA")]
    pub position: PositionRotation,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PlacedObjectPrimitive {
    pub bound_half_extents: [f32; 3],
    pub color: [f32; 3],
    _unknown0: f32,
    _unknown1: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct DoorTeleport {
    pub destination: FormId,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PortalDestination {
    pub origin: FormId,
    pub destination: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PlacedObjectLightData {
    pub fov_offset_from_base_object: f32,
    pub fade_offset_from_base_ojbect: f32,
    pub end_distance_cap: f32,
    pub shadow_depth_bias: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AlphaCutoff {
    pub current_alpha_cutoff: u8,
    pub default_alpha_cutoff: u8,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WaterVelocity {
    pub x_offset: f32,
    pub y_offset: f32,
    pub z_offset: f32,
    _pad0: u32,
    pub x_angle: f32,
    pub y_angle: f32,
    pub z_angle: f32,
    _pad1: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Radio {
    pub frequency: f32,
    pub min_weak_distance: f32,
    pub max_weak_distance: f32,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Spline {
    pub slack: f32,
    pub thickness: f32,
    pub half_extents: [f32; 3],
    // FIXME: Sometimes missing?
    // pub wind_detached_end: bool,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ProjectedDecal {
    pub width_scale: f32,
    pub height_scale: f32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ActivateParentsFlags: u8 {
        const PARENT_ACTIVATE_ONLY = 1;
    }
}

impl_decode_bitflags!(ActivateParentsFlags: u8);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ActivateParentRefs {
    pub reference: FormId,
    pub delay: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LockData {
    pub level: LockLevel,
    _pad0: [u8; 3],
    pub key: FormId,
    pub flags: u8,
    _pad1: [u8; 3],
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u8)]
pub enum LockLevel {
    None = 0,
    Novice1 = 1,
    Novice2 = 25,
    Advanced = 50,
    Expert = 75,
    Master = 100,
    RequiresTerminal = 253,
    Barred = 251,
    Chained = 252,
    Inaccessible = 254,
    RequiresKey = 255,
}

#[derive(Clone, Debug, Record)]
pub struct PlacedGrenade {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("NAME")]
    pub base_hazard: FormId,
    #[record("XOWN")]
    pub owner: Option<FormId>,
    #[record("DATA")]
    pub position: PositionRotation,
}

#[derive(Clone, Debug, Record)]
pub struct PlacedHazard {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("DATA")]
    pub position: PositionRotation,
}

#[derive(Clone, Debug, Record)]
pub struct Worldspace {
    #[record("EDID")]
    pub id: ZString,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("WCTR")]
    pub fixed_dimension_center_cell: Option<[i16; 2]>,
    #[record("LTMP")]
    pub interior_lighting: Option<FormId>,
    #[record("XEZN")]
    pub encounter_zone: Option<FormId>,
    #[record("XLCN")]
    pub location: Option<FormId>,
    #[record("WNAM")]
    pub parent_worldspace: Option<FormId>,
    #[record("PNAM")]
    pub parent_worldspace_flags: Option<ParentWorldspaceFlags>,
    #[record("CNAM")]
    pub climate: Option<FormId>,
    #[record("NAM2")]
    pub water: Option<FormId>,
    #[record("NAM3")]
    pub lod_water_type: Option<FormId>,
    #[record("NAM4")]
    pub lod_water_height: Option<f32>,
    #[record("DNAM")]
    pub land_data: Option<WorldspaceLandData>,
    #[record("ICON")]
    pub map_image: Option<ZString>,
    #[record("MNAM")]
    pub map_data: Option<WorldspaceMapData>,
    #[record("ONAM")]
    pub world_map_offset_data: Option<WorldMapOffsetData>,
    #[record("NAMA")]
    pub distant_lod_multiplier: Option<f32>,
    #[record("NAM0")]
    pub object_bounds_min: Option<[f32; 2]>,
    #[record("NAM9")]
    pub object_bounds_max: Option<[f32; 2]>,
    #[record("ZNAM")]
    pub music: Option<FormId>,
    #[record("XWEM")]
    pub water_environment_map: Option<ZString>,
    #[record("TNAM")]
    pub hd_lod_diffuse_texture: Option<ZString>,
    #[record("UNAM")]
    pub hd_lod_normal_texture: Option<ZString>,
    // #[record("WLEV")]
    // pub dimensions: Option<WorldspaceDimensions>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ParentWorldspaceFlags: u16 {
        const USE_LAND_DATA = 0x01;
        const USE_LOD_DATA = 0x02;
        const USE_MAP_DATA = 0x04;
        const USE_WATER_DATA = 0x08;
        const USE_CLIMATE_DATA = 0x20;
        const USE_SKY_CELL = 0x40;
    }
}

impl_decode_bitflags!(ParentWorldspaceFlags: u16);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WorldspaceLandData {
    pub default_land_height: f32,
    pub default_water_height: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WorldspaceMapData {
    pub width: u32,
    pub height: u32,
    pub nw_cell_x: i16,
    pub nw_cell_y: i16,
    pub se_cell_x: i16,
    pub se_cell_y: i16,
    // pub min_height: f32,
    // pub max_height: f32,
    // pub initial_pitch: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WorldMapOffsetData {
    pub world_map_scale: f32,
    pub cell_x_offset: f32,
    pub cell_y_offset: f32,
    pub cell_z_offset: f32,
}

#[derive(Clone, Debug, Record)]
pub struct Land {
    #[record("BTXT")]
    pub base_layer: Vec<LandLandTexture>,
    #[record("ATXT")]
    pub alpha_layer: SubRecordArray<LandTextureAlphaLayer>,
    #[record("VCLR")]
    pub vertex_colors: Option<Array<Rgb<u8>>>,
    #[record("VHGT")]
    pub vertex_height: Option<LandVertexHeight>,
    #[record("VNML")]
    pub vertex_normals: Option<Array<[i8; 3]>>,
    #[record("MPCD")]
    pub high_res_heighfield: SubRecordArray<Vec<u8>>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LandLandTexture {
    pub land_texture: FormId,
    pub quadrant: Quadrant,
    _unknown0: u8,
    pub texture_layer: i16,
}

#[derive(Clone, Debug, Record)]
pub struct LandTextureAlphaLayer {
    #[record("ATXT")]
    pub texture: LandLandTexture,
    #[record("VTXT")]
    pub alpha_layer_data: Array<LandTextureAlphaLayerPoint>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LandTextureAlphaLayerPoint {
    pub position: u16,
    _unknown: u16,
    pub alpha: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum Quadrant {
    BottomLeft = 0,
    BottomRight = 1,
    TopLeft = 2,
    TopRight = 3,
}

#[derive(Clone, Debug)]
pub struct LandVertexHeight {
    pub heightmap: [[f32; 33]; 33],
}

impl Decode for LandVertexHeight {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut offset = f32::decode(&mut buf)? * 8.0;

        let mut heightmap = [[0.0; 33]; 33];
        let mut row_offset = 0.0;

        for index in 0..1089 {
            let value = i8::decode(&mut buf)? as f32 * 8.0;

            let row = index / 33;
            let col = index % 33;

            if col == 0 {
                row_offset = 0.0;
                offset += value;
            } else {
                row_offset += value;
            }

            heightmap[col][row] = offset + row_offset;
        }

        Ok(Self { heightmap })
    }
}

// #[derive(Clone, Debug)]
// #[cfg_attr(feature = "serde", derive(Serialize))]
// pub struct LandVertexNormals(pub Vec<Vec3>);

// impl Decode for LandVertexNormals {
//     type Error = Error;

//     fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
//     where
//         B: Buf,
//     {
//         // The normal field has 3267 for 1089 vertices,
//         // which means we have 3 bytes per normal.
//         // This means they are probably quantized between
//         // -1.0 and 1.0.

//         let mut normals = Vec::new();
//         while buf.has_remaining() {
//             let x = u8::decode(&mut buf)?;
//             let y = u8::decode(&mut buf)?;
//             let z = u8::decode(&mut buf)?;

//             let x = f32::lerp(-1.0, 1.0, x as f32 / 255.0);
//             let y = f32::lerp(-1.0, 1.0, y as f32 / 255.0);
//             let z = f32::lerp(-1.0, 1.0, z as f32 / 255.0);

//             normals.push(Vec3::new(x, y, z));
//         }

//         Ok(Self(normals))
//     }
// }

#[derive(Clone, Debug, Record)]
pub struct PlacedMissile {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("NAME")]
    pub base_projectile: FormId,
    #[record("XEZN")]
    pub encounter_zone: Option<FormId>,
    #[record("XHTW")]
    pub head_tracking_weight: Option<f32>,
    #[record("XFVC")]
    pub favor_cost: Option<f32>,
    // XPWR
    #[record("XLKR")]
    pub linked_references: Option<Array<LinkedReference>>,
    #[record("XAPD")]
    pub activate_parents_flags: Option<ActivationParentFlags>,
    #[record("XAPR")]
    pub activate_parents: Vec<ActivateParent>,
    #[record("XAMC")]
    pub ammo_count: Option<u32>,
    #[record("XLYR")]
    pub layer: Option<FormId>,
    #[record("XMSP")]
    pub material_swap: Option<FormId>,
    #[record("XRFG")]
    pub reference_group: Option<FormId>,
    #[record("XESP")]
    pub enable_parent: Option<EnableParent>,
    #[record("XHLT")]
    pub health_percentage: Option<u32>,
    #[record("XEMI")]
    pub emittance: Option<FormId>,
    #[record("XMBR")]
    pub multibound_reference: Option<FormId>,
    #[record("XLRT")]
    pub location_ref_type: Option<Array<FormId>>,
    #[record("XLRL")]
    pub location_reference: Option<FormId>,
    #[record("XSCL")]
    pub scale: Option<f32>,
    #[record("DATA")]
    pub position: PositionRotation,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct EnableParent {
    pub reference: FormId,
    pub flags: EnableParentFlags,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct EnableParentFlags: u8 {}
}

impl_decode_bitflags!(EnableParentFlags: u8);

#[derive(Clone, Debug, Record)]
pub struct Quest {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter<QuestFragments>>,
}

#[derive(Clone, Debug, Record)]
pub struct DialogueBranch {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct DialogueTopic {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Scene {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct DialogueInfo {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Animation {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("CTDA")]
    pub conditions: SubRecordArray<Condition>,
    #[record("DNAM")]
    pub behavior_graph: Option<ZString>,
    #[record("ENAM")]
    pub animation_event: Option<ZString>,
    #[record("ANAM")]
    pub relations: AnimationRelations,
    #[record("DATA")]
    pub data: AnimationData,
    #[record("GNAM")]
    pub animation_file: Option<ZString>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AnimationRelations {
    pub parent: FormId,
    pub previous: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AnimationData {
    pub looping_seconds_min: u8,
    pub looping_seconds_max: u8,
    pub flags: AnimationFlags,
    pub animation_group_section: u8,
    pub replay_delay: u16,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct AnimationFlags: u8 {
        const PARENT = 0x01;
        const SEQUENCE = 0x02;
        const NO_ATTACKING = 0x04;
        const BLOCKING = 0x04;
    }
}

impl_decode_bitflags!(AnimationFlags: u8);

#[derive(Clone, Debug, Record)]
pub struct Package {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct CombatStyle {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct LoadScreen {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AnimatedObject {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("MODL")]
    pub model: Model,
    #[record("BNAM")]
    pub unload_event: ZString,
}

#[derive(Clone, Debug, Record)]
pub struct Water {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct EffectShader {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Explosion {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Debris {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ImageSpace {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ImageSpaceAdapter {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct FormList {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Perk {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct BodyPartData {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AddonNode {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ActorValueInfo {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct CameraShot {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct CameraPath {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct VoiceType {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct MaterialType {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Impact {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ImpactDataSet {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorAddon {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct EncounterZone {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Location {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Message {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct DefaultObjectManager {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct DefaultObject {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct LightingTemplate {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct MusicType {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Footstep {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct FootstepSet {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct StoryManagerBranchNode {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct StoryManagerQuestNode {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct StoryManagerEventNode {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct MusicTrack {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct DialogueView {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct EquipType {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Relationship {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AssocationType {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Outfit {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ArtObject {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct MaterialObject {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct MovementType {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct SoundDescriptor {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct SoundCategory {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct SoundOutputModel {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct CollisionLayer {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Color {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ReverbParameters {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct PackIn {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ReferenceGroup {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AimModel {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Layer {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ConstructibleObject {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ObjectModification {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct MaterialSwap {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("BNAM")]
    pub replacements: SubRecordArray<MaterialSwapReplacement>,
}

#[derive(Clone, Debug, Record)]
pub struct MaterialSwapReplacement {
    #[record("BNAM")]
    pub original_material: ZString,
    #[record("SNAM")]
    pub replacement_material: ZString,
}

#[derive(Clone, Debug, Record)]
pub struct Zoom {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("GNAM")]
    pub data: ZoomData,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct ZoomData {
    pub fov_mult: f32,
    pub overlay: u32,
    /// Reference to an [`ImageSpace`].
    pub imagespace_modifier: FormId,
    pub camera_offset: Vec3,
}

#[derive(Clone, Debug, Record)]
pub struct InstanceNamingRules {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct SoundKeywordMapping {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AudioEffectChain {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct SceneCollection {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AttractionRule {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AudioCategorySnapshot {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct AnimationSoundTagSet {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct NavigationMeshObstacleManager {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct LensFlare {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct GodRays {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct ObjectVisibilityManager {
    #[record("EDID")]
    pub id: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct Condition {
    #[record("CTDA")]
    pub data: ConditionData,
    #[record("CIS1")]
    pub parameter1: Option<ZString>,
    #[record("CIS2")]
    pub parameter2: Option<ZString>,
}

///
/// This should be read as `$object.$function($parameter1, $parameter2, $parameter3) $comparator $value`,
/// for example `Subject.HasKeyword(0x1234) == 0`.
#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ConditionData {
    pub comparator: u8,
    _pad0: [u8; 3],
    pub value: f32,
    pub function: u16,
    _pad1: [u8; 2],
    pub parameter1: u32,
    pub parameter2: u32,
    /// The object on which the function should be run on.
    pub object: ConditionDataObject,
    pub reference: FormId,
    pub parameter3: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u32)]
pub enum ConditionDataObject {
    Subject = 0,
    Target = 1,
    Reference = 2,
    CombatTarget = 3,
    LinkedReference = 4,
    QuestAlias = 5,
    PackageData = 6,
    EventData = 7,
    CommandTarget = 8,
    EventCameraRef = 9,
    MyKiller = 10,
}

// See https://github.com/TES5Edit/TES5Edit/blob/dev-4.1.6/Core/wbDefinitionsFO4.pas#L274
#[derive(Clone, Debug)]
pub enum ConditionFunction {
    GetWantBlocking,
    GetDistance(FormId),
    GetLocked,
    GetPos(i32),
    GetAngle(i32),
    GetStartingPos(u32),
    GetStartingAngle(u32),
    GetValue(FormId),
    GetCurrentTime,
    GetScale,
    IsMoving,
    IsTurning,
    GetLineOfSight(FormId),
    GetInSameCell(FormId),
    GetDisabled,
    MenuMode(i32),
    GetDisease,
    GetClothingValue,
    SameFaction(FormId),
    SameRace(FormId),
    SameSex(FormId),
    GetDetected(FormId),
    GetDead,
    GetItemCount(FormId),
    GetGold,
    GetSleeping,
    GetTalkedToPc,
    GetQuestRunning(FormId),
    GetStage(FormId),
    GetStageDone(FormId, u32),
    GetFactionRankDifference(FormId, FormId),
    GetAlarmed,
    IsRaining,
    GetAttacked,
    GetIsCreature,
    GetLockLevel,
    GetShouldAttack(FormId),
    GetInCell(FormId),
    GetIsClass(FormId),
    GetIsRace(FormId),
    GetIsSex(i32),
    GetInFaction(FormId),
    GetIsId(FormId),
    GetFactionRank(FormId),
    GetGlobalValue(FormId),
    IsSnowing,
    WouldBeStealing(FormId),
    GetLevel,
    IsRotating,
    GetDeadCount(FormId),
    GetIsAlerted,
    GetPlayerControlsDisabled(i32, i32, i32),
    GetHeadingAngle(FormId),
    IsWeaponMagicOut,
    IsTorchOut,
    IsShieldOut,
    IsFacingUp,
    GetKockedState,
    GetWeaponAnimType,
    IsWeaponSkillType(FormId),
    GetCurrentAiPackage,
    IsWaiting,
    IsIdlePlaying,
    IsIntimidatedByPlayer,
    IsPlayerInRegion(FormId),
    GetActorAggroRadiusViolated,
    GetCrime(FormId, i32),
    IsGreetingPlayer,
    IsGuard,
    HasBeenEaten,
    GetStaminaPercentage,
    HasBeenRead,
    GetDying,
    GetSceneActionPercent(FormId, i32),
    WouldRefuseCommand(FormId),
    SameFactionAsPc,
    SameRaceAsPc,
    SameSexAsPc,
    GetIsReference(FormId),
    IsTalking,
    GetComponentCount(FormId),
    GetCurrentAiProdecure,
    GetTrespassWarningLevel,
    IsTrespassing,
    IsInMyOwnedCell,
    GetWindSpeed,
    GetCurrentWeatherPercent,
    GetIsCurrentWeather(FormId),
    IsContinuingPackagePcNear,
    GetIsCrimeFaction(FormId),
    CanHaveFlames,
    HasFlames,
    GetOpenState,
    GetSitting,
    GetIsCurrentPackage(FormId),
    IsCurrentFurnitureRef(FormId),
    IsCurrentFurnitureObj(FormId),
    GetDayOfWeek,
    GetTalkedToPcParam(FormId),
    IsPcSleeping,
    IsPcAMurderer,
    HasSameEditorLocationAsRef(FormId, FormId),
    HasSameEditorLocationAsRefAlias(FormId, FormId),
    GetEquipped,
    IsSwimming,
    GetAmountSoldStolen,
    GetIgnoreCrime,
    GetPcExpelled(FormId),
    GetPcFactionMurder(FormId),
    GetPcEnemyOfFaction(FormId),
    GetPcFactionAttack(FormId),
    GetDestroyed,
    HasMagicEffect(FormId),
    GetDefaultOpen,
    IsSpellTarget(FormId),
    GetVatsMode,
    GetPersuasionNumber,
    GetVampireFeed,
    GetCannibal,
    GetIsClassDefault(FormId),
    GetClassDefaultMatch,
    GetInCellParam(FormId, FormId),
    GetPlayerDialogueInput,
    GetVatsTargetHeight,
    GetIsGhost,
    GetUnconscious,
    GetRestrained,
    GetIsUsedItem(FormId),
    GetIsUsedItemType(FormId),
    IsScenePlaying(FormId),
    IsInDialogueWithPlayer,
    GetLocationCleared(FormId),
    GetIsPlayableRace,
    GetOffersServiceNow,
    HasAssociationType(FormId, FormId),
    HasFamilyRelationship(FormId),
    HasParentRelationship(FormId),
    IsWarningAbout(FormId),
    IsWeaponOut,
    HasSpell(FormId),
    IsTimePassing,
    IsPleasant,
    IsCloudy,
    IsSmallBump,
    GetBaseValue(FormId),
    IsOwner(FormId),
    IsCellOwner(FormId, FormId),
    IsHorseStolen,
    IsLeftUp,
    IsSneaking,
    IsRunning,
    GetFriendHit,
    IsInCombat(i32),
    IsInInterior,
    IsWaterObject,
    GetPlayerAction,
    IsActorUsingTorch,
    IsXBox,
    GetInWorldspace(FormId),
    GetPcMiscStat(FormId),
    GetPairedAnimation,
    IsActorVictim,
    GetTotalPersuasionNumber,
    GetIdleDoneOnce,
    GetNoRumors,
    GetCombatState,
    GetWithinPackageLocation(FormId),
    IsRidingMount,
    IsFleeing,
    IsInDangerousWater,
    GetIgnoreFriendlyHits,
    IsPlayersLastRiddenMount,
    IsActor,
    IsEssential,
    IsPlayerMovingIntoNewSpace,
    GetInCurrentLocation(FormId),
    GetInCurrentLocationAlias(FormId),
    GetTimeDead,
    HasLinkedRef(FormId),
    IsChild,
    GetStolenItemValueNoCrime(FormId),
    GetLastPlayerAction,
    IsPlayerActionActive(i32),
    IsTalkingActivatorActor(FormId),
    IsInList(FormId),
    GetStolenItemValue(FormId),
    GetCrimeGoldViolent(FormId),
    GetCrimeGoldNonViolent(FormId),
    IsOwnedBy(FormId),
    GetCommandDistance,
    GetCommandLocationDistance,
    GetHitLocation,
    IsPc1stPerson,
    GetCauseOfDeath,
    IsLimbGone(i32),
    IsWeaponInList(FormId),
    IsBribedByPlayer,
    GetRelationshipRank(FormId),
    GetVatsValue(i32, i32),
    IsKiller(FormId),
    IsKillerObject(FormId),
    GetFactionCombatReaction(FormId, FormId),
    Exists(FormId),
    GetGroupMemberCount,
    GetGroupTargetCount,
    GetIsVoiceType(FormId),
    GetPlantedExplosive,
    IsScenePackageRunning,
    GetHealthPercentage,
    GetIsObjectType(FormId),
    PlayerVisualDetection,
    PlayerAudioDetection,
    GetIsCreatureType(i32),
    HasKey(FormId),
    IsFurnitureEntryType(FormId),
    GetInCurrentLocationFormList(FormId),
    GetInZone(FormId),
    GetVelocity(i32),
    GetGraphVariableFloat(String),
    HasPerk(FormId),
    GetFactionRelation(FormId),
    IsLastIdlePlayed(FormId),
    GetPlayerTeammate,
    GetPlayerTemmateCount,
    GetActorCrimePlayerEnemey,
    GetCrimeGold(FormId),
    IsPlayerGrabbedRef(FormId),
    GetkeywordItemCount(FormId),
    GetDestructionStage,
    GetIsAlignment(FormId),
    IsProtected,
    GetThreatRatio(FormId),
    GetIsUsedItemEquipType(FormId),
    GetPlayedActivated,
    GetFullyEnabledActorsInHigh,
    IsCarryable,
    GetConcuseed,
    GetMapMarkerVisible,
    PlayerKnows(FormId),
    GetPermanentValue(FormId),
    GetKillingBlowLimb,
    CanPayCrimeGold(FormId),
    GetDaysInJail,
    EpAlchemyGetMakingPoison,
    EpAlchemyEffectHasKeyword(FormId),
    GetAllowedWorldInteractions,
    DialogueGetActorValue(FormId),
    DialogueHasPerk(FormId),
    GetLastHitCritical,
    DialogueGetItemCount(FormId),
    LastCrippledCondition(FormId),
    HasSharedPowerGrid(FormId),
    IsCombatTarget(FormId),
    GetVatsRightAreaFree(FormId),
    GetVatsLeftAreaFree(FormId),
    GetVatsBackAreaFree(FormId),
    GetVatsFrontAreaFree(FormId),
    GetIsLockBroken,
    IsPS3,
    IsWindowsPc,
    GetVatsRightTargetVisible(FormId),
    GetVatsLefttargetVisible(FormId),
    GetVatsBackTargetVisible(FormId),
    GetVatsFrontTargetVisible(FormId),
    IsInCriticalStage(i32),
    GetXpForNextLevel,
    GetInfamy(FormId),
    GetInfamyViolent(FormId),
    GetInfamyNonViolent(FormId),
    GetTypeCommandPerforming,
    GetQuestCompleted(FormId),
    GetSpeechChallengeSuccessLevel,
    IsGoreDisabled,
    IsSceneActionComplte(FormId, i32),
    GetSpellUsageNum(FormId),
    GetActorsInHigh,
    HasLoaded3D,
    HasKeyword(FormId),
    HasRefType(i32),
    LocationHasKeyword(FormId),
    LocationHasRefType(i32),
    GetIsEditorLocation(FormId),
    GetIsAliasRef(FormId),
    GetIsEditorLocationAlias(FormId),
    IsSprinting,
    IsBlocking,
    HasEquippedSpell(FormId),
    GetCurrentCastingType(i32),
    GetCurrentDeliveryType(i32),
    GetAttackState,
    GetEventData(i32, i32),
    IsCloserToAThanB(FormId, FormId),
    LevelMinusPcLevel,
    IsBleedingOut,
    GetRelativeAngle(FormId, i32),
    GetMovementDirection,
    IsInScene,
    GetRefTypeDeadCount(FormId, i32),
    GetRefTypeAliveCount(FormId, i32),
    GetIsFlying,
    IsCurrentSpell(FormId, i32),
    SpellHasKeyword(FormId, FormId),
    GetEquippedItemType(i32),
    GetLocationAliasCleared(FormId),
    GetLocationAliasRefTypeDeadCount(FormId, i32),
    GetLocationAliasRefTypeAliveCount(FormId, i32),
    IsWardState(i32),
    IsInSameCurrentLocationAsRef(FormId, FormId),
    IsInSameCurrentLocationAsRefAlias(FormId, FormId),
    LocationAliasIsLocation(FormId, FormId),
    GetKeywordDataForLocation(FormId, FormId),
    GetKeywordDataForAlias(FormId, FormId),
    LocationAliasHasKeyword(FormId, FormId),
    IsNullPackageData(FormId),
    IsPlayerRadioOn,
    GetPlayerRadioFrequency,
    GetHighestRelationshipRank,
    GetLowestRelationshipRank,
    HasAssociationTypeAny(FormId),
    HasFamilyRelationshipAny,
    GetPathingTargetOffset(i32),
    GetPathingTargetAngleOffset(i32),
    GetPathingTargetSpeed,
    GetPathingTargetSpeedAngle(i32),
    GetMovementSpeed,
    GetInContainer(FormId),
    IsLocationLoaded(FormId),
    IsLocationAliasLoaded(FormId),
    IsDualCasting,
    GetVmQuestVariable(FormId, String),
    GetCombatAudioDetection,
    GetCombatVisualDetection,
    IsCasting,
    GetFlyingState,
    IsInFavorState,
    HasTwoHandedWeaponEquipped,
    IsFurnitureExittype(FormId),
    IsInFrientStateWithPlayer,
    GetWithinDistance(FormId, f32),
    GetValuePercent(FormId),
    IsUnique,
    GetLastBumpDirection,
    GetInfoChallengeSuccess,
    GetIsInjured,
    GetIsCrashLandRequest,
    GetIsHastyLandRequest,
    IsLinkedTo(FormId, FormId),
    GetKeywordDataForCurrentLocation(FormId),
    GetInSharedCrimeFaction(FormId),
    GetBribeSuccess,
    GetIntimidateSuccess,
    GetArrestedState,
    GetArrestingActor,
    HasVmScript(String),
    GetVmScriptVariable(String, String),
    GetWorkshopResourceDamage(FormId),
    IsAttacking,
    IsPowerAttacking,
    IsLastHostileActor,
    GetGraphVariableInt(String),
    ShouldAttackKill(FormId),
    GetActivationHeight,
    WornHasKeyword(FormId),
    GetPathingCurrentSpeed,
    GetPathingCurrentSpeedAngle(i32),
    GetWorkshopObjectCount(FormId),
    EpMagicSpellHashKeyword(FormId),
    GetNoBleedoutRecovery,
    EpMagicSpellHashSkill(FormId),
    IsAttackType(FormId),
    IsAllowedToFly,
    HasMagicEffectKeyword(FormId),
    IsCommandedActor,
    IsStaggered,
    IsRecoiling,
    HasScopeWeaponEquipped,
    IsPathing,
    GetShouldHelp(FormId),
    HasBoundWeaponEquipped(i32),
    GetCombatTargetHasKeyword(FormId),
    GetCombatGroupMemberCount,
    IsIgnoringCombat,
    GetLightLevel,
    SpellHasCastingPerk(FormId),
    IsBeingRidden,
    IsUndead,
    GetRealHoursPassed,
    IsUnlockedDoor,
    IsHostileToActor(FormId),
    GetTargetHeight(FormId),
    IsPoison,
    WornApparelHasKeywordCount(FormId),
    GetItemHealthPercent,
    EffectWasDualCast,
    GetKnockStateEnum,
    DoesNotExist,
    GetPlayerWalkAwayFromDialogueScene,
    GetActorStance,
    CanProduceForWorkshop,
    CanFlyHere,
    EpIsDamage(FormId),
    GetActorGunState,
    GetVoiceLineLength,
    ObjectTemplateItemHasKeyword(FormId),
    ObjectTemplateItemhasUniqueKeyword(FormId),
    ObjectTemplateItemGetLevel,
    MovementIdleMatches(i32, i32),
    GetActionData,
    GetActionDataShort(i32),
    GetActionDataByte(i32),
    GetActionDataFlag(i32),
    ModdedItemHasKeyword(FormId),
    GetAngryWithPlayer,
    IsCameraUnderWater,
    IsActorRefOwner(FormId),
    HasActorRefOwner(FormId),
    GetLoadedAmmoCount,
    IsTimespanSunrise,
    IsTimespanMorning,
    IsTimespanAfternoon,
    IsTimespanEvening,
    IsTimespanSunset,
    IsTimespanNight,
    IsTimespanMidnight,
    IsTimespanAnyDay,
    IsTimespanAnyNight,
    CurrentFurnitureHasKeyword(FormId),
    GetWeaponEquipIndex,
    IsOverEncumbered,
    IsPackageRequestingBlockedIdles,
    GetActionDataInt,
    GetVatsRightMinusLeftAreaFree(FormId),
    GetInIronSights(FormId),
    GetActorStaggerDirection,
    GetActorStaggerMagnitude,
    WornCoversBipedSlot(i32),
    GetInventoryValue,
    IsPlayerInConversation,
    IsInDialogueCamera,
    IsMyDialogueTargetPlayer,
    IsMyDialogueTargetActor,
    GetMyDialogueTargetDistance,
    IsSeatOccupied(FormId),
    IsPlayerRiding,
    IsTryingEventCamera,
    UseLeftSideCamera,
    GetNoteType,
    LocationHasPlayerOwnedWorkshop,
    IsStartinAction,
    IsMidAction,
    IsWeaponChargeAttack,
    IsInWorkshopMode,
    IsWeaponCharginhgoldAttack,
    IsEncounterAbovePlayerLevel,
    IsMeleeAttacking,
    GetVatsQueuedTargetsUnique,
    GetCurrentLocationCleared,
    IsPowered,
    GetTransmitterDistance,
    GetCameraPlaybackTime,
    IsInWater,
    GetWithinActivateDistance(FormId),
    IsUnderWater,
    IsInSameSpace(FormId),
    LocationAllowsReset,
    GetVatsBackRightAreaFree(FormId),
    GetVatsBackLeftAreaFree(FormId),
    GetVatsBackRightTargetVisible(FormId),
    GetVatsBackLeftTargetVisible(FormId),
    GetVatsTargetLimbVisible(FormId),
    IsPlayerListening(f32),
    GetPathingRequestedQuickTurn,
    EpIsCalculatingBaseDamage,
    GetReanimating,
    IsInRobotWorkbench,
}
