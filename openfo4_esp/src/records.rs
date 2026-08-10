pub mod object_modification;

use std::ops::{Deref, DerefMut};

use bitflags::bitflags;
use bytes::Buf;
use glam::Vec3;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::conditions::Conditions;
use crate::fields::{FieldMap, Read, SubField, SubRecordArray, SubRecordSection};
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

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
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
    #[record("OFST")]
    _unknown_ofst: Option<()>,
    #[record("DELE")]
    _unknown_dele: Option<()>,
    #[record("CNAM")]
    pub author: Option<ZString>,
    #[record("SNAM")]
    pub description: Option<ZString>,
    #[record("MAST")]
    pub master_files: SubRecordArray<MasterFile>,
    #[record("INTV")]
    pub intv: Option<u32>,
    #[record("INCC")]
    pub incc: Option<u32>,
}

#[derive(Clone, Debug, Record)]
pub struct MasterFile {
    #[record("MAST")]
    pub file_name: ZString,
    #[record("DATA")]
    _unknown_data: Option<()>,
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

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Self::read(map, key)
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
    /// The editor ID of this record.
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("CNAM")]
    pub color: Option<RgbPadded<u8>>,
    #[record("DNAM")]
    pub notes: Option<ZString>,
    #[record("TNAM")]
    pub type_: Option<u32>,
    #[record("DATA")]
    pub attraction_rule: Option<()>,
    #[record("FULL")]
    pub full_name: Option<LString>,
}

#[derive(Clone, Debug, Record)]
pub struct TRNS {
    #[record("EDID")]
    pub edid: ZString,
    #[record("DATA")]
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Record)]
pub struct Component {
    #[record("EDID")]
    pub id: ZString,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("CUSD")]
    pub sound_crafting: Option<FormId>,
    #[record("DATA")]
    pub auto_calc_value: Option<u32>,
    #[record("MNAM")]
    pub scrap_item: Option<FormId>,
    #[record("GNAM")]
    pub mod_scrap_scalar: Option<FormId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct Bounds {
    pub min_x: i16,
    pub min_y: i16,
    pub min_z: i16,
    pub max_x: i16,
    pub max_y: i16,
    pub max_z: i16,
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
    #[record("TX03")]
    pub emissive_texture: Option<ZString>,
    #[record("TX04")]
    pub height_texture: Option<ZString>,
    #[record("TX05")]
    pub environment_texture: Option<ZString>,
    #[record("TX02")]
    pub wrinkles_texture: Option<ZString>,
    #[record("TX06")]
    pub multilayer_texture: Option<ZString>,
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
pub struct Class {
    #[record("EDID")]
    pub id: ZString,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("DESC")]
    pub description: Option<LString>,
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("PRPS")]
    pub properties: Array<Property>,
}

#[derive(Clone, Debug, Record)]
pub struct FACT {
    /// Editor ID
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("XNAM")]
    pub relations: Vec<FactionRelation>,
    #[record("DATA")]
    pub flags: u32,
    #[record("JAIL")]
    pub jail_marker: Option<FormId>,
    #[record("WAIT")]
    pub follower_wait_marker: Option<FormId>,
    #[record("STOL")]
    pub stolen_items_container: Option<FormId>,
    #[record("PLCN")]
    pub player_inventory_container: Option<FormId>,
    #[record("CRGR")]
    pub shared_crime_faction_list: Option<FormId>,
    #[record("JOUT")]
    pub jail_outfit: Option<FormId>,
    #[record("CRVA")]
    pub crime_values: Option<FactionCrimeValues>,
    #[record("RNAM")]
    pub ranks: Vec<u32>,
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

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FactionRelation {
    pub faction: FormId,
    _unknown0: i32,
    pub combat_reaction: FactionCombatReaction,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum FactionCombatReaction {
    Neutral = 0,
    Enemy = 1,
    Ally = 2,
    Friend = 3,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FactionCrimeValues {
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
pub struct HeadPart {
    #[record("EDID")]
    pub id: ZString,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    #[record("DATA")]
    pub data: HeadPartFlags,
    #[record("PNAM")]
    pub r#type: HeadPartType,
    #[record("HNAM")]
    pub extra_parts: Vec<FormId>,
    #[record("NAM0")]
    pub assets: SubRecordArray<HeadPartAsset>,
    #[record("TNAM")]
    pub texture_set: Option<FormId>,
    #[record("CNAM")]
    pub color: Option<FormId>,
    #[record("RNAM")]
    pub valid_races: Option<FormId>,
    // TODO: Conditions
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct HeadPartFlags: u8 {
        const PLAYABLE = 0x01;
        const MALE = 0x02;
        const FEMALE = 0x04;
        const IS_EXTRA_PART = 0x10;
        const USE_SOLID_TINT = 0x20;
        const USES_BODY_TEXTURE = 0x40;
    }
}

impl_decode_bitflags!(HeadPartFlags: u8);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum HeadPartType {
    Misc = 0,
    Face = 1,
    Eyes = 2,
    Hair = 3,
    FacialHair = 4,
    Scar = 5,
    Eyebrows = 6,
    Meatcaps = 7,
    Teeth = 8,
    HeadRear = 9,
}

#[derive(Clone, Debug, Record)]
pub struct HeadPartAsset {
    #[record("NAM0")]
    pub r#type: HeadPartAssetType,
    #[record("NAM1")]
    pub file_name: ZString,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum HeadPartAssetType {
    RaceMorph = 0,
    Tri = 1,
    ChargenMorph = 2,
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
#[record(ordered)]
pub struct Race {
    /// The editor ID.
    #[record("EDID")]
    pub id: ZString,
    #[record("STCP")]
    pub animation_sound: Option<FormId>,
    /// The localized name of the record.
    #[record("FULL")]
    pub full_name: Option<LString>,
    /// The description of the record.
    #[record("DESC")]
    pub description: Option<ZString>,
    #[record("SPCT")]
    _effects_len: Option<u32>,
    #[record("SPLO")]
    pub effects: Vec<FormId>,
    /// Link to [`Armor`] that contains the skin of the race.
    #[record("WNAM")]
    pub skin: Option<FormId>,
    #[record("BOD2")]
    _bod2: Option<()>,
    #[record("KSIZ")]
    _keywords_len: Option<u32>,
    /// List of [`Keyword`] records attached to this `Race`.
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("PRPS")]
    pub properties: Option<Array<Property>>,
    #[record("APPR")]
    pub attach_parent_slots: Option<()>,
    #[record("DATA")]
    pub data: RaceData,
    #[record("MNAM")]
    pub male_skeletal_model: Option<SubRecordSection<RaceSkeletalModel>>,
    #[record("FNAM")]
    pub female_skeletal_model: Option<SubRecordSection<RaceSkeletalModel>>,
    #[record("NAM2")]
    _unknown_nam2: Option<()>,
    #[record("MTNM")]
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
    pub face_gen_main_clamp: Option<f32>,
    #[record("UNAM")]
    pub face_gen_face_clamp: Option<f32>,
    #[record("ATKR")]
    pub attack_race: Option<FormId>,
    #[record("ATKD")]
    pub attacks: SubRecordArray<RaceAttack>,
    #[record("NAM1")]
    _unknown_nam1: Option<()>,
    #[record("MNAM")]
    pub male_body_parts: Option<SubRecordSection<RaceBodyParts>>,
    #[record("FNAM")]
    pub female_body_parts: Option<SubRecordSection<RaceBodyParts>>,
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
    #[record("VNAM")]
    pub equipment_flags: Option<u32>,
    #[record("QNAM")]
    pub equip_slots: SubRecordArray<RaceEquipSlot>,
    #[record("UNWP")]
    pub unarmed_weapon: Option<FormId>,
    #[record("PHTN")]
    pub phoneme_target_names: SubRecordArray<ZString>,
    #[record("PHWT")]
    pub phonemes: SubRecordArray<()>,
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
    #[record("NAM0")]
    _head_male_marker: Option<()>,
    #[record("MNAM")]
    pub male_customization: Option<SubRecordSection<RaceMaleCustomization>>,
    #[record("NAM0")]
    _head_female_marker: Option<()>,
    #[record("FNAM")]
    pub female_customization: Option<SubRecordSection<RaceFemaleCustomization>>,
    #[record("NAM8")]
    pub morph_race: Option<FormId>,
    #[record("RNAM")]
    pub armor_race: Option<FormId>,
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
    #[record(unordered)]
    pub subgraph_data: SubRecordArray<RaceSubgraphData>,
    // TODO: Incomplete
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct Property {
    pub actor_value: FormId,
    pub value: f32,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct RaceSubgraphData {
    #[record("SAKD")]
    pub actor_keywords: Vec<FormId>,
    #[record("STKD")]
    pub target_keywords: Vec<FormId>,
    #[record("SGNM")]
    pub behavior_graph: ZString,
    #[record("SAPT")]
    pub animation_paths: Vec<ZString>,
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
    pub path: Option<ZString>,
    #[record("MODT")]
    pub info: Option<ModelInfo>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct PerGender<T> {
    pub male: T,
    pub female: T,
}

#[derive(Clone, Debug, Record)]
pub struct BodyPart {
    #[record("INDX")]
    pub index: Option<u32>,
    #[record("MODL")]
    pub model: Model,
}

#[derive(Clone, Debug, Record)]
pub struct RaceBodyParts {
    #[record("INDX")]
    pub parts: SubRecordArray<BodyPart>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceBehaviorGraph {
    #[record("MNAM")]
    pub male: Option<SubRecordSection<Model>>,
    #[record("FNAM")]
    pub female: Option<SubRecordSection<Model>>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceAttack {
    #[record("ATKD")]
    pub data: RaceAttackData,
    #[record("ATKE")]
    pub event: Option<ZString>,
    #[record("ATKW")]
    _atkw: Option<()>,
    #[record("ATKS")]
    _atks: Option<()>,
    #[record("ATKT")]
    _atkt: Option<()>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct RaceAttackData {
    pub damage_mult: f32,
    pub attack_chance: f32,
    pub attack_spell: FormId,
    pub flags: RaceAttackFlags,
    pub attack_angle: f32,
    pub strike_angle: f32,
    pub stagger: f32,
    pub knockdown: f32,
    pub recovery_time: f32,
    pub action_points_mult: f32,
    pub stagger_offsets: u32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct RaceAttackFlags: u32 {
        const POWER_ATTACK = 0x04;
    }
}

impl_decode_bitflags!(RaceAttackFlags: u32);

#[derive(Clone, Debug, Record)]
pub struct RaceEquipSlot {
    #[record("QNAM")]
    pub equip_slot: FormId,
    #[record("ZNAM")]
    _znam: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceMaleCustomization {
    #[record("NNAM")]
    pub neck_fat_adjustment: Option<()>,
    #[record("INDX")]
    pub head_parts: SubRecordArray<RaceHeadPart>,
    #[record("RPRM")]
    pub race_presets: SubRecordArray<FormId>,
    #[record("AHCM")]
    pub hair_colors: SubRecordArray<FormId>,
    #[record("FTSM")]
    pub face_details: SubRecordArray<FormId>,
    #[record("DFTM")]
    pub default_face_texture: Option<FormId>,
    #[record("TTGP")]
    pub tint_layers: SubRecordArray<RaceTintLayerGroup>,
    #[record("MPGN")]
    pub morph_groups: SubRecordArray<RaceMorphGroup>,
    #[record("FMRI")]
    pub face_morphs: SubRecordArray<RaceFaceMorph>,
    #[record("WMAP")]
    pub wrinke_map_path: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceFemaleCustomization {
    #[record("NNAM")]
    pub neck_fat_adjustment: Option<()>,
    #[record("INDX")]
    pub head_parts: SubRecordArray<RaceHeadPart>,
    #[record("RPRF")]
    pub race_presets: SubRecordArray<FormId>,
    #[record("AHCF")]
    pub hair_colors: SubRecordArray<FormId>,
    #[record("FTSF")]
    pub face_details: SubRecordArray<FormId>,
    #[record("DFTF")]
    pub default_face_texture: Option<FormId>,
    #[record("TTGP")]
    pub tint_layers: SubRecordArray<RaceTintLayerGroup>,
    #[record("MPGN")]
    pub morph_groups: SubRecordArray<RaceMorphGroup>,
    #[record("FMRI")]
    pub face_morphs: SubRecordArray<RaceFaceMorph>,
    #[record("WMAP")]
    pub wrinke_map_path: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceHeadPart {
    #[record("INDX")]
    pub index: u32,
    #[record("HEAD")]
    pub part: FormId,
}

#[derive(Clone, Debug, Record)]
pub struct RaceTintLayerGroup {
    #[record("TTGP")]
    pub group_name: LString,
    #[record("TETI")]
    pub options: SubRecordArray<RaceTintLayerGroupOption>,
    #[record("TTGE")]
    pub category_index: Option<u32>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceTintLayerGroupOption {
    #[record("TETI")]
    pub index: RaceTintLayerGroupOptionIndex,
    #[record("TTGP")]
    pub name: Option<LString>,
    #[record("TTEF")]
    pub flags: Option<u16>,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("TTET")]
    pub textures: SubRecordArray<ZString>,
    #[record("TTEB")]
    pub blend_operation: Option<()>,
    #[record("TTEC")]
    pub template_colors: Vec<RaceTintLayerTemplateColor>,
    #[record("TTED")]
    pub default: Option<f32>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct RaceTintLayerGroupOptionIndex {
    pub slot: u16,
    pub index: u16,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct RaceTintLayerTemplateColor {
    pub color: FormId,
    pub alpha: f32,
    pub template_index: u16,
    pub blend_operation: u32,
}

#[derive(Clone, Debug, Record)]
pub struct RaceMorphGroup {
    #[record("MPGN")]
    pub name: ZString,
    #[record("MPPC")]
    _presets_len: Option<u32>,
    #[record("MPPI")]
    pub presets: SubRecordArray<RaceMorphGroupPreset>,
    #[record("MPPK")]
    pub mask: Option<u16>,
    #[record("MPGS")]
    pub sliders: Option<Array<u32>>,
}

#[derive(Clone, Debug, Record)]
pub struct RaceMorphGroupPreset {
    #[record("MPPI")]
    pub index: u32,
    #[record("MPPN")]
    pub name: LString,
    #[record("MPPM")]
    pub morph: ZString,
    #[record("MPPT")]
    pub texture: FormId,
    #[record("MPPF")]
    pub playable: bool,
}

#[derive(Clone, Debug, Record)]
pub struct RaceFaceMorph {
    #[record("FMRI")]
    pub index: u32,
    #[record("FMRN")]
    pub name: LString,
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
    #[record("RDAT")]
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
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("DATA")]
    pub data: (),
    #[record("ESCE")]
    pub counter_effects: Vec<FormId>,
    #[record("SNDD")]
    pub sounds: Option<()>,
    #[record("DNAM")]
    pub magic_item_description: Option<ZString>,
    #[record("CTDA")]
    pub conditions: Conditions,
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
    pub desc: Option<ZString>,
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
    pub vmad: Option<VirtualMachineAdapter>,
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
#[record(ordered)]
pub struct Armor {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Option<Bounds>,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
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
    pub female_world_model: ArmorFemaleModel,
    #[record("ICO2")]
    pub female_inventory_image: Option<ZString>,
    #[record("MIC2")]
    pub female_message_icon: Option<ZString>,
    #[record("BOD2")]
    pub biped_body_template: BipedObjectFlags,
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
    /// Reference to a [`Race`].
    #[record("RNAM")]
    pub race: Option<FormId>,
    #[record("KSIZ")]
    _keywords_len: Option<u32>,
    /// A list of keywords attached to the `Armor`.
    ///
    /// Reference to [`Keyword`]s.
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("DESC")]
    pub description: Option<ZString>,
    #[record("INRD")]
    pub instance_naming: Option<FormId>,
    #[record("INDX")]
    pub models: SubRecordArray<ArmorModel>,
    #[record("DATA")]
    pub data: ArmorData,
    #[record("FNAM")]
    pub data2: ArmorData2,
    #[record("DAMA")]
    pub resistances: Option<Array<Resistance>>,
    #[record("TNAM")]
    pub template_armor: Option<FormId>,
    /// Reference to [`Keyword`]s.
    #[record("APPR")]
    pub attach_parent_slots: Option<Array<FormId>>,
    #[record("OBTE")]
    pub object_templates: ObjectTemplates,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorMaleModel {
    #[record("MOD2")]
    pub model: Option<ZString>,
    #[record("MO2T")]
    pub model_info: Option<ModelInfo>,
    #[record("MODC")]
    pub color_remapping_index: Option<()>,
    #[record("MO2S")]
    pub material_swap: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorFemaleModel {
    #[record("MOD4")]
    pub model: Option<ZString>,
    #[record("MO4T")]
    pub model_info: Option<ModelInfo>,
    #[record("MODC")]
    pub color_remapping_index: Option<()>,
    #[record("MO4S")]
    pub material_swap: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorModel {
    #[record("INDX")]
    pub addon_index: u16,
    #[record("MODL")]
    pub armor_addon: FormId,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct BipedObjectFlags: u32 {
        const HAIR_TOP = 0x00000001;
        const HAIR_LONG = 0x00000002;
        const FACEGEN_HEAD = 0x00000004;
        const BODY = 0x00000008;
        const LEFT_HAND = 0x00000010;
        const RIGHT_HAND = 0x00000020;
        const UNDER_TORSO = 0x00000040;
        const UNDER_LEFT_ARM = 0x00000080;
        const UNDER_RIGHT_ARM = 0x00000100;
        const UNDER_LEFT_LEG = 0x00000200;
        const UNDER_RIGHT_LEFT = 0x00000400;
        const ABOVE_TORSO = 0x00000800;
        const ABOVE_LEFT_ARM = 0x00001000;
        const ABOVE_RIGHT_ARM = 0x00002000;
        const ABOVE_LEFT_LEG = 0x00004000;
        const ABOVE_RIGHT_LEG = 0x00008000;
        const HEADBAND = 0x00010000;
        const EYES = 0x00020000;
        const BEARD = 0x00040000;
        const MOUTH = 0x00080000;
        const NECK = 0x00100000;
        const RING = 0x00200000;
        const SCALP = 0x00400000;
        const DECAPITATION = 0x00800000;
        const UNNAMED0 = 0x01000000;
        const UNNAMED1 = 0x02000000;
        const UNNAMED2 = 0x04000000;
        const UNNAMED3 = 0x08000000;
        const UNNAMED4 = 0x10000000;
        const SHIELD = 0x20000000;
        const PIPBOY = 0x40000000;
        const FX = 0x80000000;
    }
}

impl_decode_bitflags!(BipedObjectFlags: u32);

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
pub struct Book {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub objet_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    #[record("DESC")]
    pub description: Option<LString>,
    // TODO
    #[record("DEST")]
    dest: Option<()>,
    #[record("YNAM")]
    pub sound_pick_up: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("FIMD")]
    pub featured_item_message: Option<FormId>,
    #[record("DATA")]
    pub data: Option<BookData>,
    #[record("DNAM")]
    pub data2: Option<BookData2>,
    #[record("CNAM")]
    pub description2: Option<LString>,
    #[record("INAM")]
    pub inventory_art: Option<FormId>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct BookData {
    pub value: u32,
    pub weight: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct BookData2 {
    pub flags: BookFlags,
    /// [`ActorValue`], [`Spell`] or [`Perk`] depending on `flags`. Unused otherwise.
    pub form: FormId,
    pub text_offset_x: u32,
    pub text_offset_y: u32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct BookFlags: u8 {
        const ADVANCE_ACTOR_VALUE = 0x01;
        const CANT_BE_TAKEN = 0x02;
        const ADD_SPELL = 0x04;
        const ADD_PERK = 0x08;
    }
}

impl_decode_bitflags!(BookFlags: u8);

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
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
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
pub struct Door {
    #[record("EDID")]
    pub id: ZString,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("FULL")]
    pub full_name: Option<LString>,
    // TODOO
    #[record("MODL")]
    pub model: Model,
    #[record("SNAM")]
    pub sound_open: Option<FormId>,
    #[record("ANAM")]
    pub sound_close: Option<FormId>,
    #[record("BNAM")]
    pub sound_loop: Option<FormId>,
    #[record("FNAM")]
    pub flags: DoorFlags,
    #[record("TNAM")]
    pub tnam: Vec<FormId>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct DoorFlags: u8 {
        const AUTOMATIC = 0x02;
        const HIDDEN = 0x04;
        const MINIMAL_USE = 0x08;
        const SLIDING = 0x10;
        const DO_NOT_OPEN_IN_COMBAT_SEARCH = 0x20;
        const NO_TO_TEXT = 0x40;
    }
}

impl_decode_bitflags!(DoorFlags: u8);

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
    pub kwda: Option<Array<FormId>>,
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

impl<T> Deref for Array<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T> IntoIterator for &'a Array<T> {
    type Item = &'a T;
    type IntoIter = <&'a Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Array<T> {
    type Item = &'a mut T;
    type IntoIter = <&'a mut Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T> IntoIterator for Array<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
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
    pub kwda: Option<Array<FormId>>,
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
    pub keywords: Option<Array<FormId>>,
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
    #[record("BAMT")]
    pub alternate_block_material: Option<FormId>,
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
    #[record("OBTE")]
    pub object_templates: ObjectTemplates,
    #[record("DNAM")]
    pub data: WeaponData,
    #[record("FNAM")]
    pub data2: Option<WeaponData2>,
    #[record("CRDT")]
    pub critical_data: Option<WeaponCriticalData>,
    #[record("INAM")]
    pub impact_data_set: Option<FormId>,
    #[record("LNAM")]
    pub npc_add_ammo_list: Option<FormId>,
    /// Reference to [`AimModel`].
    #[record("WAMD")]
    pub aim_model: Option<FormId>,
    #[record("WZMD")]
    pub zoom: Option<FormId>,
    #[record("CNAM")]
    pub template: Option<FormId>,
    #[record("DAMA")]
    pub damage_types: Option<Array<WeaponDamageType>>,
    #[record("FLTR")]
    pub filter: Option<ZString>,
    #[record("MASE")]
    pub melee_speed: Option<MeleeSpeed>,
}

#[derive(Clone, Debug, Record)]
pub struct ObjectTemplates {
    #[record("OBTE")]
    _template_count: Option<u32>,
    #[record("OBTS")]
    #[record(unordered)]
    pub templates: SubRecordArray<ObjectTemplate>,
    #[record("STOP")]
    _stop: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ObjectTemplate {
    #[record("OBTF")]
    _editor_only: Option<()>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("OBTS")]
    pub item: ObjectTemplateItem,
}

#[derive(Clone, Debug)]
pub struct ObjectTemplateItem {
    pub includes: Vec<ObjectTemplateItemInclude>,
    pub keywords: Vec<FormId>,
    pub min_level: u8,
    pub max_level: u8,
    pub parent_combination_index: Option<u16>,
    pub default: bool,
    pub min_level_for_ranks: u8,
    pub alt_level_per_tier: u8,
}

impl Decode for ObjectTemplateItem {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let include_count = u32::decode(&mut buf)?;
        let property_count = u32::decode(&mut buf)?;
        let min_level = u8::decode(&mut buf)?;
        u8::decode(&mut buf)?;
        let max_level = u8::decode(&mut buf)?;
        u8::decode(&mut buf)?;
        let parent_combination_index = u16::decode(&mut buf)?;
        let parent_combination_index = if parent_combination_index == u16::MAX {
            None
        } else {
            Some(parent_combination_index)
        };

        let default = bool::decode(&mut buf)?;

        let keyword_count = u8::decode(&mut buf)?;
        let keywords = (0..keyword_count)
            .map(|_| FormId::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        let min_level_for_ranks = u8::decode(&mut buf)?;
        let alt_level_per_tier = u8::decode(&mut buf)?;

        let includes = (0..include_count)
            .map(|_| ObjectTemplateItemInclude::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            includes,
            min_level,
            max_level,
            parent_combination_index,
            default,
            keywords,
            min_level_for_ranks,
            alt_level_per_tier,
        })
    }
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct ObjectTemplateItemInclude {
    pub item_mod: FormId,
    pub attach_point_index: u8,
    pub optional: bool,
    pub dont_use_all: bool,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct WeaponData {
    /// Ammo that this [`Weapon`] fires.
    pub ammo: FormId,
    pub speed: f32,
    pub reload_speed: f32,
    pub reach: f32,
    pub min_range: f32,
    pub max_range: f32,
    pub attack_delay: f32,
    _unknown0: u32,
    /// Fixed factor that is applied when the target is outside the `[min_range..max_range]`
    /// bounds.
    pub damage_out_of_range_mult: f32,
    pub on_hit: u32,
    pub skill: FormId,
    pub resist: FormId,
    pub flags: WeaponDataFlags,
    /// Ammo capacity.
    pub capacity: u16,
    pub animation_type: WeaponAnimationType,
    pub secondary_damage: f32,
    /// Weight of the item in the inventory.
    pub weight: f32,
    pub value: u32,
    /// The physical damage that this weapon deals.
    ///
    /// Other damage types are defined in [`damage_types`].
    ///
    /// [`damage_types`]: Weapon::damage_types
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

#[derive(Clone, Debug, BinaryStruct)]
pub struct WeaponData2 {
    pub animation_fire_seconds: f32,
    pub rumble_left_motor_strength: f32,
    pub rumble_right_motor_strength: f32,
    pub rumble_duration: f32,
    pub animation_reload_seconds: f32,
    pub bolt_anim_seconds: f32,
    pub sighted_transition_seconds: f32,
    pub num_projectiles: u8,
    pub override_projectile: FormId,
    pub pattern: u32,
    pub rumble_period_ms: u32,
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
    pub keywords: Option<Array<FormId>>,
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
    #[record("PRPS")]
    pub properties: Array<Property>,
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
    pub keywords: Option<Array<FormId>>,
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
    pub template_flags: NpcTemplateFlags,
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

impl_decode_bitflags!(NpcBaseFlags: u32);

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NpcTemplateFlags: u16 {
        const TRAITS = 0x0001;
        const STATS = 0x0002;
        const FACTIONS = 0x0004;
        const SPELL_LIST = 0x0008;
        const AI_DATA = 0x0010;
        const AI_PACKAGES = 0x0020;
        const MODEL = 0x0040;
        const BASE_DATA = 0x0080;
        const INVENTORY = 0x0100;
        const SCRIPT = 0x0200;
        const DEF_PACK_LIST = 0x0400;
        const ATTACK_DATA = 0x0800;
        const KEYWORDS = 0x1000;
    }
}

impl_decode_bitflags!(NpcTemplateFlags: u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
pub struct NpcFactionMember {
    pub faction: FormId,
    pub rank: i8,
}

/// Indicates which properties an [`Npc`] should inherit from another [`Npc`] record.
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

impl SoundLevel {
    pub(crate) fn from_u32(v: u32) -> Result<Self, Error> {
        match v {
            0 => Ok(Self::Loud),
            1 => Ok(Self::Normal),
            2 => Ok(Self::Silent),
            3 => Ok(Self::VeryLoud),
            4 => Ok(Self::Quiet),
            _ => Err(Error::BadEnumValue(stringify!(SoundLevel), v.into())),
        }
    }
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
    /// The chance of this `LeveledActor` to evaluate to no actor.
    #[record("LVLD")]
    pub chance_none: u8,
    #[record("LVLM")]
    pub max_count: u8,
    #[record("LVLF")]
    pub flags: LeveledActorFlags,
    /// Use a [`GLOB`] variable to set the `chance_none` field.
    #[record("LVLG")]
    pub use_global: Option<FormId>,
    #[record("LLCT")]
    _list_count: u8,
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
        /// If set any actor with level set to less than or equal to the player's level may be
        /// selected. If this flag is not selected one entry with the highest level that is less
        /// than or equal to the player's level is selected.
        const ALL_LEVELS = 0x01;
        const EACH = 0x02;
        /// Always add all entries, disregarding their chances.
        const USE_ALL = 0x04;
    }
}

impl_decode_bitflags!(LeveledActorFlags: u8);

/// A possible actor selection of a [`LeveledActor`].
#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LeveledListEntry {
    /// The minimum player level at which the entry may be selected.
    pub level: u16,
    _pad0: u16,
    pub reference: FormId,
    pub count: u16,
    /// The chance that this `LeveldListEntry` is NOT selected. In other words this is the inverse
    /// chance.
    pub chance_none: u8,
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
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
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
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
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
    pub addiction_name: Option<ZString>,
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
    #[record("MODL")]
    pub model: Model,
    #[record("DNAM")]
    pub data: ProjectileData,
    #[record("NAM1")]
    pub muzzle_flash_model: ProjectileMuzzleFlashModel,
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
pub struct ProjectileMuzzleFlashModel {
    #[record("NAM1")]
    pub model: Option<ZString>,
    #[record("NAM2")]
    pub model_info: Option<ModelInfo>,
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
    pub cloud_texture_color: [TimeOfDaySetExtended<Rgba<u8>>; 32],
    #[record("JNAM")]
    pub cloud_texture_alpha: [TimeOfDaySetExtended<f32>; 32],
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
    pub image_spaces: TimeOfDaySetExtended<FormId>,
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

impl<T> IntoIterator for TimeOfDaySet<T> {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, 4>;

    fn into_iter(self) -> Self::IntoIter {
        [self.sunrise, self.day, self.sunset, self.night].into_iter()
    }
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

impl<T> IntoIterator for TimeOfDaySetExtended<T> {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, 8>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.sunrise,
            self.day,
            self.sunset,
            self.night,
            self.early_sunrise,
            self.late_sunrise,
            self.early_sunset,
            self.late_sunset,
        ]
        .into_iter()
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct WeatherColors {
    pub sky_upper: TimeOfDaySetExtended<Rgba<u8>>,
    pub fog_near: TimeOfDaySetExtended<Rgba<u8>>,
    _unknown0: TimeOfDaySetExtended<Rgba<u8>>,
    pub ambient: TimeOfDaySetExtended<Rgba<u8>>,
    pub sunlight: TimeOfDaySetExtended<Rgba<u8>>,
    pub sun: TimeOfDaySetExtended<Rgba<u8>>,
    pub stars: TimeOfDaySetExtended<Rgba<u8>>,
    pub sky_lower: TimeOfDaySetExtended<Rgba<u8>>,
    pub horizon: TimeOfDaySetExtended<Rgba<u8>>,
    pub effect_lightning: TimeOfDaySetExtended<Rgba<u8>>,
    pub cloud_lod_diffuse: TimeOfDaySetExtended<Rgba<u8>>,
    pub cloud_lod_ambient: TimeOfDaySetExtended<Rgba<u8>>,
    pub fog_far: TimeOfDaySetExtended<Rgba<u8>>,
    pub sky_statics: TimeOfDaySetExtended<Rgba<u8>>,
    pub water_multiplier: TimeOfDaySetExtended<Rgba<u8>>,
    pub sun_glare: TimeOfDaySetExtended<Rgba<u8>>,
    pub moon_glare: TimeOfDaySetExtended<Rgba<u8>>,
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
    pub sunrise_begin: ClimateTime,
    pub sunrise_end: ClimateTime,
    pub sunset_begin: ClimateTime,
    pub sunset_end: ClimateTime,
    pub volatility: u8,
    pub moons: u8,
}

/// Time for an event in [`ClimateTiming`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClimateTime {
    /// Hour between 0 and 24 (exclusive).
    pub hour: u8,
    /// Minute between 0 and 60 (exclusive).
    pub minute: u8,
}

impl ClimateTime {
    pub fn hours(&self) -> f32 {
        self.hour as f32 + self.minute as f32 / 60.0
    }
}

impl Decode for ClimateTime {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut value = u8::decode(buf)?;
        // Clamp at 23:50, which is the maximum valid value.
        value %= 23 * 6 + 5;

        let hour = value / 6;
        let minute = (value % 6) * 10;

        Ok(Self { hour, minute })
    }
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
    #[record("VISI")]
    _unknown_visi: Option<()>,
    #[record("RVIS")]
    _unknown_rvis: Option<()>,
    #[record("PCMB")]
    _unknown_pcmb: Option<()>,
    #[record("XCLC")]
    pub grid: Option<Grid>,
    #[record("XCLL")]
    pub lighting: Option<CellLighting>,
    #[record("CNAM")]
    _unknown_cnam: Option<()>,
    #[record("ZNAM")]
    _unknown_znam: Option<()>,
    #[record("TVDT")]
    _unknown_tvdt: Option<()>,
    #[record("MHDT")]
    pub max_height_data: Option<()>,
    #[record("LTMP")]
    pub lighting_template: FormId,
    #[record("XCLW")]
    pub water_height: f32,
    #[record("XCLR")]
    pub regions: Option<Array<FormId>>,
    #[record("XLCN")]
    pub location: Option<FormId>,
    #[record("XWCN")]
    _unknown_xwcn: Option<()>,
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
    #[record("XPTL")]
    pub portal_room: Option<()>,
    #[record("XORD")]
    pub linked_occlusion_reference: Option<()>,
    #[record("XOCP")]
    pub occlusion_plane_data: Option<()>,
    #[record("XMBP")]
    pub multibound_primitive: Option<()>,
    #[record("XRGD")]
    pub ragdoll_data: Option<()>,
    #[record("XRGB")]
    pub ragdoll_biped_rotation: Option<()>,
    #[record("XLIG")]
    pub light_data: Option<PlacedObjectLightData>,
    #[record("XRDS")]
    pub radius: Option<f32>,
    #[record("XEMI")]
    pub emitted_light: Option<FormId>,
    #[record("XALP")]
    pub alpha_cutoff: Option<AlphaCutoff>,
    #[record("XTEL")]
    pub door_teleport: Option<DoorTeleport>,
    #[record("XTNM")]
    pub teleport_location_name: Option<FormId>,
    #[record("XMBR")]
    pub multibound_reference: Option<FormId>,
    #[record("XWCN")]
    _unknown_xwcn: Option<()>,
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
    #[record("XLCM")]
    pub level_modifier: Option<()>,
    #[record("XLCN")]
    pub persist_location: Option<FormId>,
    #[record("XTRI")]
    pub collision_layer: Option<u32>,
    #[record("XLOC")]
    pub lock_data: Option<LockData>,
    #[record("XEZN")]
    pub encounter_zone: Option<FormId>,
    #[record("XNDP")]
    pub navigation_door_link: Option<()>,
    #[record("XLRL")]
    pub location_reference: Option<FormId>,
    #[record("XLRT")]
    pub location_ref_type: Vec<FormId>,
    #[record("XIS2")]
    pub ignored_by_sandbox: Option<()>,
    #[record("XOWN")]
    xown: Option<()>,
    #[record("XRNK")]
    xrnk: Option<()>,
    #[record("XCNT")]
    pub item_count: Option<i32>,
    #[record("XHLT")]
    pub health_percentage: Option<u32>,
    #[record("XESP")]
    pub enable_parent: Option<EnableParent>,
    #[record("XPRD")]
    pub patrol: SubRecordArray<PatrolData>,
    #[record("XLKR")]
    pub linked_references: Option<Array<LinkedReference>>,
    #[record("XACT")]
    pub action_flag: Option<()>,
    #[record("XHTW")]
    pub head_tracking_weight: Option<f32>,
    #[record("XFVC")]
    pub favor_cost: Option<f32>,
    #[record("ONAM")]
    pub open_by_default: Option<()>,
    #[record("XMRK")]
    pub map_marker: Option<SubRecordSection<MapMarker>>,
    #[record("XATR")]
    pub attach_ref: Option<()>,
    // SPLINE CONNECTION
    // POWER GRID
    #[record("XCVR")]
    _unknown_xcvr: Option<()>,
    #[record("XCVL")]
    _unknown_xcvl: Option<()>,
    #[record("XCZR")]
    pub current_zone_reference: Option<FormId>,
    #[record("XCZA")]
    _unknown_xcza: Option<()>,
    #[record("XCZC")]
    pub current_zone_cell: Option<FormId>,
    #[record("XSCL")]
    pub scale: Option<f32>,
    #[record("XLOD")]
    pub distant_lod_data: Option<()>,
    #[record("DATA")]
    pub position: PositionRotation,
}

#[derive(Clone, Debug, Record)]
pub struct MapMarker {
    #[record("FNAM")]
    pub flags: u8,
    #[record("FULL")]
    pub full_name: LString,
    #[record("TNAM")]
    pub type_: u8,
}

#[derive(Clone, Debug, Record)]
pub struct PatrolData {
    #[record("XPRD")]
    pub idle_time: f32,
    #[record("XPPA")]
    pub patrol_script_marker: Option<()>,
    #[record("INAM")]
    pub idle: FormId,
    #[record("PDTO")]
    pub topics: Vec<PatrolTopic>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct PatrolTopic {
    pub ty: u32,
    pub data: u32,
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
    /// The reference to the other door.
    ///
    /// This is a [`PlacedObject`].
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
pub struct NavMesh {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("NVNM")]
    pub geometry: NavMeshGeometry,
}

#[derive(Clone, Debug)]
pub struct NavMeshGeometry {
    pub hash: u32,
    pub parent_world: FormId,
    pub grid_x: i16,
    pub grid_y: i16,
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<NavMeshGeometryTriangle>,
    pub edge_links: Vec<NavMeshGeometryEdgeLink>,
}

impl Decode for NavMeshGeometry {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
        #[primitive(u32)]
        enum NavMeshGeometryVersion {
            V15 = 15,
        }

        NavMeshGeometryVersion::decode(&mut buf)?;

        let hash = u32::decode(&mut buf)?;
        let parent_world = FormId::decode(&mut buf)?;
        let grid_x = i16::decode(&mut buf)?;
        let grid_y = i16::decode(&mut buf)?;

        let vertex_count = u32::decode(&mut buf)?;
        let mut vertices = Vec::new();
        for _ in 0..vertex_count {
            let vertex = Vec3::decode(&mut buf)?;
            vertices.push(vertex);
        }

        let triangle_count = u32::decode(&mut buf)?;
        let mut triangles = Vec::new();
        for _ in 0..triangle_count {
            let triangle = NavMeshGeometryTriangle::decode(&mut buf)?;
            triangles.push(triangle);
        }

        let edge_link_count = u32::decode(&mut buf)?;
        let mut edge_links = Vec::new();
        for _ in 0..edge_link_count {
            let edge_link = NavMeshGeometryEdgeLink::decode(&mut buf)?;
            edge_links.push(edge_link);
        }

        // TODO: Door Triangles, Cover Array, ..

        Ok(Self {
            hash,
            parent_world,
            grid_x,
            grid_y,
            vertices,
            triangles,
            edge_links,
        })
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NavMeshGeometryTriangle {
    pub vertices: [u16; 3],
    pub edges: [u16; 3],
    pub height: f32,
    pub flags2: NavMeshTriangleFlags2,
    pub flags: NavMeshTriangleFlags,
    pub cover_flags: u16,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NavMeshTriangleFlags2: u8 {
        /// Triangle is only accessible to floating actors.
        const FLOAT = 1 << 4;
    }
}

impl_decode_bitflags!(NavMeshTriangleFlags2: u8);

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NavMeshTriangleFlags: u16 {
        const F0 = 1 << 0;
        const F1 = 1 << 1;
        const F2 = 1 << 2;
        const DELETED = 1 << 3;
        const NO_LARGE_CREATURES = 1 << 4;
        const OVERLAPPING = 1 << 5;
        const PREFERRED = 1 << 6;
        const WATER = 1 << 9;
        const DOOR = 1 << 10;
        const FOUND = 1 << 11;
    }
}

impl_decode_bitflags!(NavMeshTriangleFlags: u16);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct NavMeshGeometryEdgeLink {
    pub ty: u32,
    pub navmesh: FormId,
    pub triangle: u16,
    pub edge_index: u8,
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
    #[record("DATA")]
    pub data: u32,
    #[record("VNML")]
    pub vertex_normals: Option<Array<[i8; 3]>>,
    #[record("VHGT")]
    pub vertex_height: Option<LandVertexHeight>,
    #[record("VCLR")]
    pub vertex_colors: Option<Array<Rgb<u8>>>,
    #[record("BTXT")]
    pub base_layer: Vec<LandLandTexture>,
    #[record("ATXT")]
    pub alpha_layer: SubRecordArray<LandTextureAlphaLayer>,
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
#[record(ordered)]
pub struct Quest {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter<QuestFragments>>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("DNAM")]
    pub data: QuestData,
    #[record("ENAM")]
    pub event: Option<u32>,
    #[record("LNAM")]
    pub location: Option<FormId>,
    #[record("XNAM")]
    pub quest_completion_xp: Option<FormId>,
    #[record("QTGL")]
    pub text_display_globals: Vec<FormId>,
    #[record("FLTR")]
    pub filter: Option<ZString>,
    #[record("CTDA")]
    pub quest_dialogue_conditions: Conditions,
    #[record("NEXT")]
    _marker: Option<()>,
    #[record("CTDA")]
    pub story_manager_conditions: Conditions,
    #[record("INDX")]
    pub stages: SubRecordArray<QuestStage>,
    #[record("QOBJ")]
    pub objectives: SubRecordArray<QuestObjective>,
    #[record("ANAM")]
    pub next_alias_id: u32,
    #[record("????")] // Ignored
    pub aliases: QuestAliases,
    #[record("NNAM")]
    pub description: Option<ZString>,
    #[record("GNAM")]
    pub quest_group: Option<FormId>,
    #[record("SNAM")]
    pub swf_file: Option<ZString>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct QuestData {
    pub flags: QuestDataFlags,
    pub priority: u8,
    _pad0: u8,
    pub delay_time: f32,
    pub r#type: QuestDataType,
    _pad1: [u8; 3],
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QuestDataFlags: u16 {
        const START_GAME_ENABLED = 0x0001;
        const COMPLETED = 0x0002;
        const ADD_IDLE_TOPIC_TO_HELLOS = 0x0004;
        const ALLOW_REPEATED_STAGES = 0x0008;
        const STARTS_ENABLED = 0x0010;
        const DISPLAYED_IN_HUD = 0x0020;
        const FAILED = 0x0040;
        const STAGE_WAIT = 0x0080;
        const RUN_ONCE = 0x0100;
        const EXCLUDE_FROM_DIALOGUE_EXPORT = 0x0200;
        const WARN_ON_ALIAS_FILL_FAILURE = 0x0400;
        const ACTIVE = 0x0800;
        const REPEATS_CONDITIONS = 0x1000;
        const KEEP_INSTANCE = 0x2000;
        const WANT_DORMANT = 0x4000;
        const HAS_DIALOGUE_DATA = 0x8000;
    }
}

impl_decode_bitflags!(QuestDataFlags: u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum QuestDataType {
    None = 0,
    MainQuest = 1,
    BrotherhoodOfSteel = 2,
    Institute = 3,
    Minutemen = 4,
    Railroad = 5,
    Misc = 6,
    SideQuest = 7,
    Dlc1 = 8,
    Dlc2 = 9,
    Dlc3 = 10,
    Dlc4 = 11,
    Dlc5 = 12,
    Dlc6 = 13,
    Dlc7 = 14,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestStage {
    #[record("INDX")]
    pub index: QuestStageIndex,
    #[record("QSDT")]
    pub log_entries: SubRecordArray<QuestStageLogEntry>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct QuestStageIndex {
    pub stage_index: u16,
    pub flags: QuestStageFlags,
    _pad0: u8,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QuestStageFlags: u8 {
        const RUN_ON_START = 0x02;
        const RUN_ON_STOP = 0x04;
        const KEEP_INSTANCE_DATA_FROM_HERE_ON = 0x08;
    }
}

impl_decode_bitflags!(QuestStageFlags: u8);

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestStageLogEntry {
    #[record("QSDT")]
    pub flags: QuestStageLogEntryFlags,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("NAM2")]
    pub note: Option<ZString>,
    #[record("CNAM")]
    pub log_entry: Option<LString>,
    #[record("NAM0")]
    pub next_quest: Option<FormId>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QuestStageLogEntryFlags: u8 {
        const COMPLETE_QUEST = 0x01;
        const FAIL_QUEST = 0x02;
    }
}

impl_decode_bitflags!(QuestStageLogEntryFlags: u8);

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestObjective {
    #[record("QOBJ")]
    pub objective_index: u16,
    #[record("FNAM")]
    pub flags: QuestObjectiveFlags,
    #[record("NNAM")]
    pub display_text: Option<LString>,
    #[record("QSTA")]
    pub targets: SubRecordArray<QuestObjectiveTarget>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QuestObjectiveFlags: u32 {
        const OR_WITH_PREVIOUS = 0x01;
        const NO_STATS_TRACKING = 0x02;
    }
}

impl_decode_bitflags!(QuestObjectiveFlags: u32);

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestObjectiveTarget {
    #[record("QSTA")]
    pub data: QuestObjectiveTargetData,
    #[record("CTDA")]
    pub conditions: Conditions,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct QuestObjectiveTargetData {
    pub alias: i32,
    pub flags: QuestObjectiveTargetDataFlags,
    pub keyword: FormId,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QuestObjectiveTargetDataFlags: u32 {
        const COMPASS_MARKER_IGNORES_LOCKS = 0x01;
        const HOSTILE = 0x02;
        const USE_STRAIGHT_LINE_PATHING = 0x04;
    }
}

impl_decode_bitflags!(QuestObjectiveTargetDataFlags: u32);

#[derive(Clone, Debug)]
pub struct QuestAliases {
    pub references: Vec<QuestReferenceAlias>,
    pub locations: Vec<QuestLocationAlias>,
    pub ref_collections: Vec<QuestAliasRefCollectionAlias>,
}

impl Read for QuestAliases {
    type Error = Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        let mut references = Vec::new();
        let mut locations = Vec::new();
        let mut ref_collections = Vec::new();

        loop {
            match map.next_key() {
                Some(Text([b'A', b'L', b'S', b'T'])) => {
                    let alias = QuestReferenceAlias::read(map, key)?;
                    references.push(alias);
                }
                Some(Text([b'A', b'L', b'L', b'S'])) => {
                    let location = QuestLocationAlias::read(map, key)?;
                    locations.push(location);
                }
                Some(Text([b'A', b'L', b'C', b'S'])) => {
                    let ref_col = QuestAliasRefCollectionAlias::read(map, key)?;
                    ref_collections.push(ref_col);
                }
                _ => {
                    return Ok(Self {
                        references,
                        locations,
                        ref_collections,
                    });
                }
            }
        }
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Self::read(map, key)
    }
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestReferenceAlias {
    #[record("ALST")]
    pub id: u32,
    #[record("ALID")]
    pub name: Option<ZString>,
    #[record("FNAM")]
    pub flags: QuestAliasFlags,
    #[record("ALFI")]
    pub force_into_alias_when_filled: Option<i32>,
    /// Select a specific reference.
    ///
    /// Reference to [`PlacedObject`] or [`PlacedNpc`].
    #[record("ALFR")]
    pub forced_reference: Option<FormId>,
    /// Select the reference of an actor that has the [`UNIQUE`] flag.
    ///
    /// Reference to an [`Npc`].
    ///
    /// [`UNIQUE`]: NpcBaseFlags::UNIQUE
    #[record("ALUA")]
    pub unique_actor: Option<FormId>,
    #[record("ALFA")]
    pub location_alias_reference: Option<SubField<QuestAliasLocationAliasReference>>,
    /// Select an alias from another quest.
    #[record("ALEQ")]
    pub external_alias_reference: Option<SubField<QuestAliasExternalAlias>>,
    #[record("ALCO")]
    pub create_reference_to_object: Option<SubField<QuestAliasCreateReferenceToObject>>,
    #[record("ALNA")]
    pub find_matching_reference_near_alias:
        Option<SubField<QuestAliasFindMatchingReferenceNearAlias>>,
    #[record("ALFE")]
    pub find_matching_reference_from_event: Option<SubField<QuestAliasFindMatchingFromEvent>>,
    /// Select the closest reference to another alias.
    #[record("ALCC")]
    pub closest_to_alias: Option<i32>,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("KSIZ")]
    _keywords_len: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("COCT")]
    _item_count: Option<()>,
    #[record("CNTO")]
    pub items: Vec<ContainerItem>,
    #[record("SPOR")]
    pub spectator_override_package_list: Option<FormId>,
    #[record("OCOR")]
    pub observe_dead_body_override_package_list: Option<FormId>,
    #[record("GWOR")]
    pub guard_warn_override_package_list: Option<FormId>,
    #[record("ECOR")]
    pub combat_override_package_list: Option<FormId>,
    #[record("ALLA")]
    pub linked_aliases: Option<QuestAliasLinkedAlias>,
    #[record("ALDN")]
    pub display_name: Option<FormId>,
    #[record("ALFV")]
    pub forced_voice: Option<FormId>,
    #[record("ALDI")]
    pub death_item: Option<FormId>,
    #[record("ALSP")]
    pub spells: SubRecordArray<FormId>,
    #[record("ALFC")]
    pub factions: SubRecordArray<FormId>,
    #[record("ALPC")]
    pub packages: SubRecordArray<FormId>,
    #[record("VTCK")]
    pub voice_types: Option<FormId>,
    #[record("ALED")]
    _end_marker: Option<()>,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestAliasCreateReferenceToObject {
    #[record("ALCO")]
    pub object: FormId,
    #[record("ALCA")]
    pub alias: i16,
    #[record("ALCL")]
    pub level: Option<QuestAliasLevel>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum QuestAliasLevel {
    Easy = 0,
    Medium = 1,
    Hard = 2,
    VeryHard = 3,
    None = 4,
}

#[derive(Clone, Debug, Record)]
pub struct QuestAliasFindMatchingReferenceNearAlias {
    #[record("ALNA")]
    pub alias: i32,
    #[record("ALNT")]
    pub r#type: i32,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestLocationAlias {
    #[record("ALLS")]
    pub id: u32,
    #[record("ALID")]
    pub name: Option<ZString>,
    #[record("FNAM")]
    pub flags: QuestAliasFlags,
    #[record("ALFI")]
    pub force_into_alias_when_filled: Option<i32>,
    #[record("ALFL")]
    pub specific_location: Option<FormId>,
    #[record("ALFA")]
    pub reference_alias_location: Option<SubField<QuestAliasReferenceAliasLocation>>,
    #[record("ALEQ")]
    pub external_alias_location: Option<SubField<QuestAliasExternalAlias>>,
    #[record("ALFE")]
    pub find_matching_location_from_event: Option<SubField<QuestAliasFindMatchingFromEvent>>,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("ALCC")]
    pub closest_to_alias: Option<i32>,
    #[record("ALED")]
    _end_marker: Option<()>,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestAliasLocationAliasReference {
    #[record("ALFA")]
    pub alias: i32,
    #[record("KNAM")]
    pub keyword: Option<FormId>,
    #[record("ALRT")]
    pub ref_type: FormId,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestAliasReferenceAliasLocation {
    #[record("ALFA")]
    pub alias: i32,
    #[record("KNAM")]
    pub keyword: Option<FormId>,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestAliasExternalAlias {
    /// The [`Quest`] from which the alias should be selected.
    #[record("ALEQ")]
    pub quest: FormId,
    /// The ID of the alias in the other quest.
    #[record("ALEA")]
    pub alias: i32,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct QuestAliasFindMatchingFromEvent {
    #[record("ALFE")]
    pub from_event: u32,
    #[record("ALFD")]
    pub event_data: u32,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct QuestAliasLinkedAlias {
    pub keyword: FormId,
    pub alias: i32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct QuestAliasFlags: u32 {
        const RESERVES = 0x00000001;
        /// Allow nothing to be selected.
        ///
        /// If not `OPTIONAL` and nothing was selected quest creation should fail.
        const OPTIONAL = 0x00000002;
        const QUEST_OBJECT = 0x00000004;
        /// Allow the same entity to be selected for multiple aliases.
        const ALLOW_REUSE_IN_QUEST = 0x00000008;
        /// Allow dead references to be selected.
        const ALLOW_DEAD = 0x00000010;
        const MATCHING_REF_IN_LOADED_AREA = 0x00000020;
        /// Make the actor essential for the duration of the quest.
        const ESSENTIAL = 0x00000040;
        /// Allow disabled entities to be selected.
        const ALLOW_DISABLED = 0x00000080;
        const STORES_TEXT = 0x00000100;
        const ALLOW_RESERVED = 0x00000200;
        /// Make the actor protected for the duration of the quest.
        const PROTECTED = 0x00000400;
        const FORCED_BY_ALIASES = 0x00000800;
        const ALLOW_DESTROYED = 0x00001000;
        const MATCHING_REF_CLOSEST = 0x00002000;
        const USES_STORED_TEXT = 0x00004000;
        const INITIALLY_DISABLED = 0x00008000;
        const ALLOW_CLEARED = 0x00010000;
        const CLEAR_NAMES_WHEN_REMOVED = 0x00020000;
        const MATCHING_REF_ACTORS_ONLY = 0x00040000;
        const CREATE_REF_TEMP = 0x00080000;
        const EXTERNAL_ALIAS_LINKED = 0x00100000;
        const NO_PICKPOCKET = 0x00200000;
        const CAN_APPLY_DATA_TO_NON_ALIASED_REFS = 0x00400000;
        const IS_COMPANION = 0x00800000;
        const OPTIONAL_ALL_SCENES = 0x01000000;
    }
}

impl_decode_bitflags!(QuestAliasFlags: u32);

#[derive(Clone, Debug, Record)]
pub struct QuestAliasRefCollectionAlias {
    #[record("ALCS")]
    pub id: u32,
    #[record("ALMI")]
    pub max_initial_fill_count: Option<u8>,
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
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("PNAM")]
    pub priority: Option<f32>,
    #[record("BNAM")]
    pub branch: Option<FormId>,
    #[record("QNAM")]
    pub quest: Option<FormId>,
    #[record("KNAM")]
    pub keyword: Option<FormId>,
    #[record("DATA")]
    pub data: DialogueTopicData,
    #[record("SNAM")]
    pub subtype_name: Option<Text<4>>,
    #[record("TIFC")]
    pub info_count: Option<u32>,
    // TODO:
    // INOM
    // INOA
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct DialogueTopicData {
    pub flags: u8,
    pub category: u8,
    pub subtype: u16,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct Scene {
    #[record("EDID")]
    pub id: Option<ZString>,
    // TODO: Fragments
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("FNAM")]
    pub flags: SceneFlags,
    #[record("HNAM")]
    pub phases: SubRecordArray<ScenePhase>,
    #[record("ALID")]
    pub actors: SubRecordArray<SceneActor>,
    #[record("ANAM")]
    pub actions: SubRecordArray<SceneAction>,
    #[record("PNAM")]
    pub parent_quest: FormId,
    #[record("INAM")]
    pub last_action_index: Option<u32>,
    #[record("VNAM")]
    pub actor_behavior_settings: Option<SceneActorBehaviorSettings>,
    #[record("CNAM")]
    pub camera_distance_override: Option<f32>,
    #[record("ACTV")]
    pub dialogue_distance_override: Option<f32>,
    #[record("CRIS")]
    pub fov_override: Option<f32>,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("SCQS")]
    pub set_parent_quest_stage: Option<SceneSetStage>,
    #[record("NNAM")]
    pub notes: Option<ZString>,
    #[record("TNAM")]
    pub template_scene: Option<FormId>,
    #[record("XNAM")]
    pub index: Option<u32>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct SceneFlags: u32 {
        const BEGIN_ON_QUEST_START = 1 << 0;
        const STOP_ON_QUEST_END = 1 << 1;
        const SHOW_ALL_TEXT = 1 << 2;
        const REPEAT_CONDITIONS_WHILE_TRUE = 1 << 3;
        const INTERRUPTIBLE = 1 << 4;
        const HAS_PLAYER_DIALOGUE = 1 << 5;
        const PREVENT_PLAYER_EXIT_DIALOGUE = 1 << 6;
        const PAUSE_ACTORS_CURRENT_SCENES = 1 << 7;
        const DISABLE_DIALOGUE_CAMERA = 1 << 8;
        const NO_FOLLOWER_IDLE_CHATTER = 1 << 9;
    }
}

impl_decode_bitflags!(SceneFlags: u32);

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct ScenePhase {
    #[record("HNAM")]
    _marker_start: (),
    #[record("NAM0")]
    pub name: Option<ZString>,
    #[record("CTDA")]
    pub start_conditions: Conditions,
    #[record("NEXT")]
    _marker_start_conditions_end: (),
    #[record("CTDA")]
    pub completion_conditions: Conditions,
    #[record("NEXT")]
    _marker_completion_conditions_end: (),
    #[record("WNAM")]
    pub editor_width: Option<u32>,
    #[record("FNAM")]
    pub flags: Option<ScenePhaseFlags>,
    #[record("SCQS")]
    pub set_parent_quest_stage: Option<SceneSetStage>,
    #[record("HNAM")]
    _marker_end: (),
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ScenePhaseFlags: u16 {
        const START_WALK_AWAY_PHASE = 1 << 0;
        const DONT_RUN_END_SCRIPTS_ON_SCENE_JUMP = 1 << 1;
        const START_INHERIT_IN_TEMPLATED_SCENES = 1 << 2;
    }
}

impl_decode_bitflags!(ScenePhaseFlags: u16);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct SceneSetStage {
    pub on_start: u16,
    pub on_completion: u16,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct SceneActor {
    /// Alias ID of the actor in the owning [`Quest`].
    #[record("ALID")]
    pub alias: i32,
    #[record("LNAM")]
    pub flags: Option<SceneActorFlags>,
    #[record("DNAM")]
    pub behavior_flags: Option<SceneActorBehaviorFlags>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct SceneActorFlags: u32 {
        const NO_PLAYER_ACTIVATION = 1 << 0;
        const OPTIONAL = 1 << 1;
        const RUN_ONLY_SCENE_PACKAGES = 1 << 2;
        const NO_COMMAND_STATE = 1 << 3;
    }
}

impl_decode_bitflags!(SceneActorFlags: u32);

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct SceneActorBehaviorFlags: u32 {
        const DEATH_PAUSE = 1 << 0;
        const DEATH_END = 1 << 1;
        const COMBAT_PAUSE = 1 << 2;
        const COMBAT_END = 1 << 3;
        const DIALOGUE_PAUSE = 1 << 4;
        const DIALOGUE_END = 1 << 5;
        const OBS_COM_PAUSE = 1 << 6;
        const OBS_COM_END = 1 << 7;
    }
}

impl_decode_bitflags!(SceneActorBehaviorFlags: u32);

#[derive(Clone, Debug)]
pub struct SceneAction {
    pub name: Option<ZString>,
    pub actor: i32,
    pub index: u32,
    pub flags: Option<SceneActionFlags>,
    pub start_phase: u32,
    pub end_phase: u32,
    pub data: SceneActionData,
}

impl Read for SceneAction {
    type Error = Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Self::read_next(map, key)
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
        #[primitive(u16)]
        enum SceneActionType {
            Dialogue = 0,
            Package = 1,
            Timer = 2,
            PlayerDialogue = 3,
            StartScene = 4,
            NpcResponseDialogue = 5,
            Radio = 6,
        }

        let ty = map.consume_next(Text::new(b"ANAM"))?;
        let ty = ty.ok_or(Error::MissingField("ANAM"))?.value;

        let name = map.consume_next(Text::new(b"NAM0"))?;
        let name = name.map(|v| v.value);

        let actor = map.consume_next(Text::new(b"ALID"))?;
        let actor = actor.ok_or(Error::MissingField("ALID"))?.value;

        let index = map.consume_next(Text::new(b"INAM"))?;
        let index = index.ok_or(Error::MissingField("INAM"))?.value;

        let flags = map.consume_next(Text::new(b"FNAM"))?;
        let flags = flags.map(|v| v.value);

        let start_phase = map.consume_next(Text::new(b"SNAM"))?;
        let start_phase = start_phase.ok_or(Error::MissingField("SNAM"))?.value;

        let end_phase = map.consume_next(Text::new(b"ENAM"))?;
        let end_phase = end_phase.ok_or(Error::MissingField("ENAM"))?.value;

        let data = match ty {
            SceneActionType::Dialogue => {
                let data = SceneActionDialogueData::read_next(map, key)?;
                SceneActionData::Dialogue(data)
            }
            SceneActionType::Package => {
                let data = SceneActionPackageData::read_next(map, key)?;
                SceneActionData::Package(data)
            }
            SceneActionType::Timer => {
                let data = SceneActionTimerData::read_next(map, key)?;
                SceneActionData::Timer(data)
            }
            SceneActionType::PlayerDialogue => {
                let data = SceneActionPlayerDialogueData::read_next(map, key)?;
                SceneActionData::PlayerDialogue(data)
            }
            SceneActionType::StartScene => {
                let data = SceneActionStartSceneData::read_next(map, key)?;
                SceneActionData::StartScene(data)
            }
            SceneActionType::NpcResponseDialogue => {
                let data = SceneActionNpcResponseDialogueData::read_next(map, key)?;
                SceneActionData::NpcResponseDialogue(data)
            }
            SceneActionType::Radio => {
                let data = SceneActionRadioData::read_next(map, key)?;
                SceneActionData::Radio(data)
            }
        };

        map.consume_next::<()>(Text::new(b"ANAM"))?
            .ok_or(Error::MissingField("ANAM"))?;

        Ok(Self {
            name,
            actor,
            index,
            flags,
            start_phase,
            end_phase,
            data,
        })
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct SceneActionFlags: u32 {
        const PLAYER_POSITIVE_USE_DIALOGUE_SUBTYPE = 1 << 7;
        const PLAYER_NEGATIVE_USE_DIALOGUE_SUBTYPE = 1 << 8;
        const PLAYER_NEUTRAL_USE_DIALOGUE_SUBTYPE = 1 << 9;
        const USE_DIALOGUE_SUBTYPE = 1 << 10;
        const PLAYER_QUESTION_DIALOGUE_SUBTYPE = 1 << 11;
        const KEEP_CLEAR_TARGET_ON_ACTION_END = 1 << 12;
        const RUN_ON_END_OF_PHASE = 1 << 13;
        const FACE_TARGET = 1 << 14;
        const LOOPING = 1 << 15;
        const HEADTRACK_PLAYER = 1 << 16;
        const IGNORE_FOR_COMPLETION = 1 << 17;
        const CAMERA_SPEAKER_TARGET = 1 << 21;
        const COMPLETE_FACE_TARGET = 1 << 22;
        const NPC_POSITIVE_USE_DIALOGUE_SUBTYPE = 1 << 27;
        const NPC_NEGATIVE_USE_DIALOGUE_SUBTYPE = 1 << 28;
        const NPC_NEUTRAL_USE_DIALOGUE_SUBTYPE = 1 << 29;
        const NPC_QUESTION_USE_DIALOGUE_SUBTYPE = 1 << 30;
    }
}

impl_decode_bitflags!(SceneActionFlags: u32);

#[derive(Clone, Debug)]
pub enum SceneActionData {
    Dialogue(SceneActionDialogueData),
    Package(SceneActionPackageData),
    Timer(SceneActionTimerData),
    PlayerDialogue(SceneActionPlayerDialogueData),
    StartScene(SceneActionStartSceneData),
    NpcResponseDialogue(SceneActionNpcResponseDialogueData),
    Radio(SceneActionRadioData),
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionDialogueData {
    #[record("DATA")]
    pub topic: FormId,
    #[record("DMAX")]
    pub looping_max: f32,
    #[record("DMIN")]
    pub looping_min: f32,
    #[record("CRIS")]
    pub camera: Option<SceneActionDialogueDataCamera>,
    #[record("DEMO")]
    pub emotion_type: Option<u32>,
    #[record("DEVA")]
    pub emotion_value: Option<u32>,
    #[record("HTID")]
    pub player_headtracking: Option<Array<u32>>,
    #[record("VENC")]
    pub dialogue_subtype: Option<FormId>,
    #[record("ONAM")]
    pub audio_output_override: Option<FormId>,
    #[record("PNAM")]
    pub anim_archetype: Option<FormId>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct SceneActionDialogueDataCamera {
    pub fov_on_player_camera: f32,
    pub rate_of_camera_change: f32,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionPackageData {
    #[record("PNAM")]
    pub packages: Vec<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionTimerData {
    #[record("SNAM")]
    pub max_seconds_seconds: Option<f32>,
    #[record("SCQS")]
    pub phase_index: Option<u16>,
    #[record("TNAM")]
    pub min_time_seconds: Option<f32>,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionPlayerDialogueData {
    #[record("PTOP")]
    pub player_positive_response: Option<FormId>,
    #[record("NTOP")]
    pub player_negative_response: Option<FormId>,
    #[record("NETO")]
    pub player_neutral_response: Option<FormId>,
    #[record("QTOP")]
    pub player_question_response: Option<FormId>,
    #[record("VENC")]
    pub player_positive_dialogue_subtype: Option<FormId>,
    #[record("PLVD")]
    pub player_negative_dialogue_subtype: Option<FormId>,
    #[record("JOUT")]
    pub player_neutral_dialogue_subtype: Option<FormId>,
    #[record("DALC")]
    pub player_question_dialogue_subtype: Option<FormId>,
    #[record("DTID")]
    pub npc_headtracking: Option<Array<u32>>,
    #[record("ONAM")]
    pub audio_output_override: Option<FormId>,
    #[record("NPOT")]
    pub npc_positive_response: Option<FormId>,
    #[record("NNGT")]
    pub npc_negative_response: Option<FormId>,
    #[record("NNUT")]
    pub npc_neutral_response: Option<FormId>,
    #[record("NQUT")]
    pub npc_question_response: Option<FormId>,
    #[record("NPOS")]
    pub npc_positive_dialogue_subtype: Option<FormId>,
    #[record("NNGS")]
    pub npc_negative_dialogue_subtype: Option<FormId>,
    #[record("NNUS")]
    pub npc_neutral_dialogue_subtype: Option<FormId>,
    #[record("NQUS")]
    pub npc_question_dialogue_subtype: Option<FormId>,
    #[record("DTGT")]
    pub dialogue_actor_target: Option<u32>,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionStartSceneData {
    #[record("STSC")]
    pub topic: Option<FormId>,
    #[record("LCEP")]
    pub scenes: SubRecordArray<SceneActionStartSceneDataScene>,
    #[record("HTID")]
    pub end_scene_say_greeting: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionStartSceneDataScene {
    #[record("LCEP")]
    pub scene: FormId,
    #[record("INTT")]
    pub phase_index: u16,
    #[record("SSPN")]
    pub start_phase_for_scene: Option<ZString>,
    #[record("CITC")]
    _condition_count: Option<u32>,
    // CITC
    #[record("CTDA")]
    pub conditions: Conditions,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionNpcResponseDialogueData {
    #[record("PTOP")]
    pub player_positive_response: Option<FormId>,
    #[record("NTOP")]
    pub player_negative_response: Option<FormId>,
    #[record("NETO")]
    pub player_neutral_response: Option<FormId>,
    #[record("QTOP")]
    pub player_question_response: Option<FormId>,
    #[record("VENC")]
    pub player_positive_dialogue_subtype: Option<FormId>,
    #[record("PLVD")]
    pub player_negative_dialogue_subtype: Option<FormId>,
    #[record("JOUT")]
    pub player_neutral_dialogue_subtype: Option<FormId>,
    #[record("DALC")]
    pub player_question_dialogue_subtype: Option<FormId>,
    #[record("DTID")]
    pub npc_headtracking: Option<Array<u32>>,
    #[record("ONAM")]
    pub audio_output_override: Option<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct SceneActionRadioData {
    #[record("DATA")]
    pub topic: Option<FormId>,
    #[record("HTID")]
    pub play_sound: Option<FormId>,
    #[record("DMAX")]
    pub looping_max: Option<f32>,
    #[record("VENC")]
    pub dialogue_subtype: Option<FormId>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct SceneActorBehaviorSettings {
    pub death: SceneActorBehavior,
    pub combat: SceneActorBehavior,
    pub player_dialogue: SceneActorBehavior,
    pub observe_combat: SceneActorBehavior,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SceneActorBehavior {
    SetAllNormal,
    SetAllPause,
    SetAllEnd,
    DontSetAll,
}

impl Decode for SceneActorBehavior {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        match u32::decode(buf)? {
            0 => Ok(Self::SetAllNormal),
            1 => Ok(Self::SetAllPause),
            2 => Ok(Self::SetAllEnd),
            // Interstingly, sometimes the last SceneActorBehaviorSettings
            // field contains garbage data.
            // For most fields we read the expected value of 3 here,
            // but the garbage seems to also represent DontSetAll?
            _ => Ok(Self::DontSetAll),
        }
    }
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct DialogueInfo {
    #[record("EDID")]
    pub id: Option<ZString>,
    // TODO: Fragments
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    // TODO
    #[record("ENAM")]
    pub response_flags: Option<()>,
    #[record("TPIC")]
    pub topic: Option<FormId>,
    #[record("PNAM")]
    pub previous_info: Option<FormId>,
    #[record("DNAM")]
    pub shared_info: Option<FormId>,
    #[record("GNAM")]
    pub info_group: Option<FormId>,
    #[record("IOVR")]
    pub override_filename: Option<ZString>,
    #[record("TRDA")]
    pub responses: SubRecordArray<DialogueInfoResponse>,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("RNAM")]
    pub prompt: Option<LString>,
    #[record("ANAM")]
    pub speaker: Option<FormId>,
    #[record("TSCE")]
    pub start_scene: Option<FormId>,
    #[record("INTV")]
    _intv: Option<()>,
    #[record("ALFA")]
    pub forced_alias: Option<i32>,
    #[record("ONAM")]
    pub audio_output_override: Option<FormId>,
    #[record("GREE")]
    pub greet_distance: Option<u32>,
    #[record("TIQS")]
    pub set_parent_quest_stage: Option<SceneSetStage>,
    #[record("NAM0")]
    pub start_scene_phase: Option<ZString>,
    #[record("INCC")]
    pub challenge: Option<DialogueInfoChallenge>,
    #[record("MODQ")]
    pub reset_global: Option<FormId>,
    #[record("INAM")]
    pub subtitle_priority: Option<DialogueInfoSubtitlePriority>,
}

#[derive(Clone, Debug, Record)]
pub struct DialogueInfoResponse {
    #[record("TRDA")]
    pub response: DialogueInfoResponseData,
    #[record("NAM1")]
    pub response_text: Option<LString>,
    #[record("NAM2")]
    pub script_notes: Option<ZString>,
    #[record("NAM3")]
    pub edits: Option<ZString>,
    #[record("NAM4")]
    pub alternate_lip_text: Option<ZString>,
    #[record("SNAM")]
    pub idle_animation_speaker: Option<FormId>,
    #[record("LNAM")]
    pub idle_animation_listener: Option<FormId>,
    #[record("TNAM")]
    pub interrupt_percentage: Option<u16>,
    #[record("NAM9")]
    pub text_hash: Option<Array<u8>>,
    #[record("SRAF")]
    pub camera_path: Option<FormId>,
    #[record("WZMD")]
    pub stop_on_scene_end: Option<()>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct DialogueInfoResponseData {
    pub emotion: FormId,
    pub response_number: u8,
    pub sound_file: FormId,
    _pad0: u8,
    pub interrupt_percentage: u16,
    pub camera_target_alias: i32,
    pub camera_location_alias: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum DialogueInfoChallenge {
    None = 0,
    Easy = 1,
    Medium = 2,
    Hard = 3,
    AlwaysSucceeds = 4,
    EasyRepeatable = 5,
    MediumRepeatable = 6,
    HardRepeatable = 7,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum DialogueInfoSubtitlePriority {
    Low = 0,
    Normal = 1,
    _Unknown2 = 2,
    Force = 3,
}

#[derive(Clone, Debug, Record)]
pub struct Animation {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("CTDA")]
    pub conditions: Conditions,
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
#[record(ordered)]
pub struct Package {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("PKDT")]
    pub data: PackageData,
    #[record("PSDT")]
    pub schedule: Option<PackageSchedule>,
    #[record("CTDA")]
    pub conditions: Conditions,
    // IDLE ANIMATIONS
    #[record("CNAM")]
    pub combat_style: Option<FormId>,
    #[record("QNAM")]
    pub owner_quest: Option<FormId>,
    #[record("PKCU")]
    pub counter: Option<PackageCounter>,
    #[record("ANAM")]
    pub data_input_values: SubRecordArray<PackageDataInputValue>,
    #[record("UNAM")]
    pub data_inputs: SubRecordArray<PackageDataInput>,
    #[record("XNAM")]
    _marker: Option<()>,
    #[record("ANAM")]
    pub procedure_tree: Option<SubField<PackageProcedureTree>>,
    // TODO: DATA INPUTS
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageData {
    pub flags: u32,
    pub ty: PackageType,
    pub interrupt_override: PackageInterruptOverride,
    pub preferred_speed: PackagePreferredSpeed,
    _pad0: u8,
    pub interrupt_flags: u16,
    _pad1: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum PackageType {
    Package = 18,
    PackageTemplate = 19,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum PackageInterruptOverride {
    None = 0,
    Spectator = 1,
    ObserveDead = 2,
    GuardWarn = 3,
    Combat = 4,
    CommandTravel = 5,
    CommandActivate = 6,
    LeaveWorkstation = 7,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum PackagePreferredSpeed {
    Walk = 0,
    Jog = 1,
    Run = 2,
    FastWalk = 3,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageSchedule {
    pub month: PackageScheduleMonth,
    pub day_of_week: PackageScheduleDayOfWeek,
    pub date: u8,
    pub hour: u8,
    pub minute: u8,
    pub duration: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum PackageScheduleMonth {
    January = 0,
    February = 1,
    March = 2,
    April = 3,
    May = 4,
    June = 5,
    July = 6,
    August = 7,
    September = 8,
    October = 9,
    November = 10,
    December = 11,
    /// March, April, May
    Spring = 12,
    /// June, July, August
    Summer = 13,
    /// September, October, November
    Autumn = 14,
    /// December, January, February
    Winter = 15,
    Any = 255,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum PackageScheduleDayOfWeek {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Weekdays = 7,
    Weekends = 8,
    MondayWednesdayFriday = 9,
    TuesdayThursday = 10,
    Any = 255,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageCounter {
    pub data_input_count: u32,
    pub package_template: FormId,
    pub version_counter: u32,
}

#[derive(Clone, Debug, Record)]
pub struct PackageDataInputValue {
    #[record("ANAM")]
    pub ty: ZString,
    #[record("CNAM")]
    pub value: Option<Array<u8>>,
    #[record("BNAM")]
    _bnam: Option<()>,
    #[record("PDTO")]
    pub topics: Option<Array<TopicData>>,
    #[record("PLDT")]
    pub location: Option<PackageDataLocation>,
    #[record("PTDA")]
    pub target: Option<PackageTargetData>,
    #[record("TPIC")]
    pub dialogue_topic: Option<FormId>,
}

#[derive(Clone, Debug)]
pub enum TopicData {
    Ref(FormId),
    Subtype(Text<4>),
}

impl Decode for TopicData {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
        #[primitive(u32)]
        enum TopicDataType {
            Ref = 0,
            Subtype = 1,
        }

        let ty = TopicDataType::decode(&mut buf)?;
        match ty {
            TopicDataType::Ref => {
                let value = FormId::decode(&mut buf)?;
                Ok(Self::Ref(value))
            }
            TopicDataType::Subtype => {
                let value = Text::decode(&mut buf)?;
                Ok(Self::Subtype(value))
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PackageDataLocation {
    pub value: PackageDataLocationValue,
    pub radius: i32,
    pub collection_index: Option<u32>,
}

impl Decode for PackageDataLocation {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let value = PackageDataLocationValue::decode(&mut buf)?;
        let radius = i32::decode(&mut buf)?;
        // This is sometimes missing?
        let collection_index = (buf.remaining() >= 4)
            .then(|| u32::decode(&mut buf))
            .transpose()?;

        Ok(Self {
            value,
            radius,
            collection_index,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum PackageDataLocationValue {
    Reference(FormId),
    Cell(FormId),
    NearPackageStartLocation([u8; 4]),
    NearEditorLocation([u8; 4]),
    ObjectId(FormId),
    ObjectType(u32),
    Keyword(FormId),
    RefAlias(u32),
    LocationAlias(u32),
    InterruptData(u32),
    PackageDataTarget(u32),
    RefCollectionAlias(u32),
    Unknown,
}

impl Decode for PackageDataLocationValue {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
        #[primitive(u32)]
        enum PackageDataLocationType {
            Reference = 0,
            Cell = 1,
            NearPackageStartLocation = 2,
            NearEditorLocation = 3,
            ObjectId = 4,
            ObjectType = 5,
            Keyword = 6,
            Unknown7 = 7,
            RefAlias = 8,
            LocationAlias = 9,
            InterruptData = 10,
            PackageDataTarget = 11,
            Unknown12 = 12,
            Unknown13 = 13,
            RefCollectionAlias = 14,
        }

        let ty = PackageDataLocationType::decode(&mut buf)?;
        match ty {
            PackageDataLocationType::Reference => {
                let value = FormId::decode(buf)?;
                Ok(Self::Reference(value))
            }
            PackageDataLocationType::Cell => {
                let value = FormId::decode(buf)?;
                Ok(Self::Cell(value))
            }
            PackageDataLocationType::NearPackageStartLocation => {
                let value = <[u8; 4]>::decode(buf)?;
                Ok(Self::NearPackageStartLocation(value))
            }
            PackageDataLocationType::NearEditorLocation => {
                let value = <[u8; 4]>::decode(buf)?;
                Ok(Self::NearEditorLocation(value))
            }
            PackageDataLocationType::ObjectId => {
                let value = FormId::decode(buf)?;
                Ok(Self::ObjectId(value))
            }
            PackageDataLocationType::ObjectType => {
                let value = u32::decode(buf)?;
                Ok(Self::ObjectType(value))
            }
            PackageDataLocationType::Keyword => {
                let value = FormId::decode(buf)?;
                Ok(Self::Keyword(value))
            }
            PackageDataLocationType::Unknown7 => {
                <[u8; 4]>::decode(buf)?;
                Ok(Self::Unknown)
            }
            PackageDataLocationType::RefAlias => {
                let value = u32::decode(buf)?;
                Ok(Self::RefAlias(value))
            }
            PackageDataLocationType::LocationAlias => {
                let value = u32::decode(buf)?;
                Ok(Self::LocationAlias(value))
            }
            PackageDataLocationType::InterruptData => {
                let value = u32::decode(buf)?;
                Ok(Self::InterruptData(value))
            }
            PackageDataLocationType::PackageDataTarget => {
                let value = u32::decode(buf)?;
                Ok(Self::InterruptData(value))
            }
            PackageDataLocationType::Unknown12 | PackageDataLocationType::Unknown13 => {
                <[u8; 4]>::decode(buf)?;
                Ok(Self::Unknown)
            }
            PackageDataLocationType::RefCollectionAlias => {
                let value = u32::decode(buf)?;
                Ok(Self::RefCollectionAlias(value))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageTargetData {
    pub value: PackageTargetDataValue,
    pub count: i32,
}

#[derive(Copy, Clone, Debug)]
pub enum PackageTargetDataValue {
    SpecificReference(FormId),
    ObjectId(FormId),
    ObjectType(u32),
    /// Keyword
    LinkedReference(FormId),
    RefAlias(u32),
    InterruptData(u32),
    Self_,
    Keyword(FormId),
    Unknown,
}

impl Decode for PackageTargetDataValue {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
        #[primitive(u32)]
        enum PackageTargetDataType {
            SpecificReference = 0,
            ObjectId = 1,
            ObjectType = 2,
            LinkedReference = 3,
            RefAlias = 4,
            InterruptData = 5,
            Self_ = 6,
            Keyword = 7,
            Unknown8 = 8,
        }

        let ty = PackageTargetDataType::decode(&mut buf)?;
        match ty {
            PackageTargetDataType::SpecificReference => {
                let value = FormId::decode(buf)?;
                Ok(Self::SpecificReference(value))
            }
            PackageTargetDataType::ObjectId => {
                let value = FormId::decode(buf)?;
                Ok(Self::ObjectId(value))
            }
            PackageTargetDataType::ObjectType => {
                let value = u32::decode(buf)?;
                Ok(Self::ObjectType(value))
            }
            PackageTargetDataType::LinkedReference => {
                let value = FormId::decode(buf)?;
                Ok(Self::LinkedReference(value))
            }
            PackageTargetDataType::RefAlias => {
                let value = u32::decode(buf)?;
                Ok(Self::RefAlias(value))
            }
            PackageTargetDataType::InterruptData => {
                let value = u32::decode(buf)?;
                Ok(Self::InterruptData(value))
            }
            PackageTargetDataType::Self_ => {
                <[u8; 4]>::decode(buf)?;
                Ok(Self::Self_)
            }
            PackageTargetDataType::Keyword => {
                let value = FormId::decode(buf)?;
                Ok(Self::Keyword(value))
            }
            PackageTargetDataType::Unknown8 => {
                <[u8; 4]>::decode(buf)?;
                Ok(Self::Unknown)
            }
        }
    }
}

#[derive(Clone, Debug, Record)]
pub struct PackageDataInput {
    #[record("UNAM")]
    pub index: u8,
    #[record("BNAM")]
    pub name: Option<ZString>,
    #[record("PNAM")]
    pub flags: Option<PackageDataInputFlags>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct PackageDataInputFlags: u32 {
        const PUBLIC = 1;
    }
}

impl_decode_bitflags!(PackageDataInputFlags: u32);

#[derive(Clone, Debug, Record)]
pub struct PackageProcedureTree {
    #[record("ANAM")]
    pub branches: SubRecordArray<PackageProcedureNode>,
}

#[derive(Clone, Debug, Record)]
pub struct PackageProcedureNode {
    #[record("ANAM")]
    pub ty: ZString,
    #[record("CITC")]
    _condition_count: u32,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("PRCB")]
    pub children: Option<PackageProcedureNodeChildren>,
    #[record("PNAM")]
    pub procedure: Option<ZString>,
    #[record("FNAM")]
    pub flags: Option<PackageProcedureNodeFlags>,
    #[record("PKC2")]
    pub data_inputs: Vec<u8>,
    #[record("PFO2")]
    pub flags_override: Option<PackageProcedureNodeOverride>,
    #[record("PFOR")]
    _unknown0: Option<()>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageProcedureNodeChildren {
    /// The number of child nodes following this node.
    pub count: u32,
    pub flags: u32,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct PackageProcedureNodeFlags: u32 {
        const SUCCESS_COMPLETES_PACKAGE = 1;
    }
}

impl_decode_bitflags!(PackageProcedureNodeFlags: u32);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageProcedureNodeOverride {
    pub set_general_flags: u32,
    pub clear_general_flags: u32,
    pub set_interrupt_flags: u32,
    pub clear_interrupt_flags: u32,
    pub preferred_speed_override: u8,
    _pad0: [u8; 3],
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
    #[record("ENAM")]
    pub data: Option<ImageSpaceData>,
    #[record("HNAM")]
    pub hdr: Option<ImageSpaceHdrData>,
    #[record("CNAM")]
    pub cinematic: Option<ImageSpaceCinematicData>,
    #[record("TNAM")]
    pub tint: Option<ImageSpaceTint>,
    #[record("DNAM")]
    pub depth_of_field: Option<ImageSpaceDepthOfField>,
    #[record("TX00")]
    pub lut: Option<ZString>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ImageSpaceData {
    pub eye_adapt_speed: f32,
    pub tonemap_e: f32,
    pub bloom_threshold: f32,
    pub bloom_scale: f32,
    pub auto_exposure_min_max: f32,
    pub sunlight_scale: f32,
    pub sky_scale: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub tint_scale: f32,
    pub tint_color: Rgb<f32>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ImageSpaceHdrData {
    pub eye_adapt_speed: f32,
    pub tonemap_e: f32,
    pub bloom_threshold: f32,
    pub bloom_scale: f32,
    pub auto_exposure_max: f32,
    pub auto_exposure_min: f32,
    pub sunlight_scale: f32,
    pub sky_scale: f32,
    pub middle_gray: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ImageSpaceCinematicData {
    pub saturation: f32,
    pub brightness: f32,
    pub contrast: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ImageSpaceTint {
    pub scale: f32,
    pub color: Rgb<f32>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct ImageSpaceDepthOfField {
    pub strength: f32,
    pub distance: f32,
    pub range: f32,
    _pad0: u16,
    pub blur_radius: u16,
    #[default_value]
    pub vignette_radius: f32,
    #[default_value]
    pub vignette_strength: f32,
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
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("LNAM")]
    pub forms: Vec<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct Perk {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct BodyPartData {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("MODL")]
    pub skeleton_model: BodyPartDataSkeletonModel,
    #[record("BPNN")]
    #[record(unordered)]
    pub parts: SubRecordArray<BodyPartDataBodyPart>,
}

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct BodyPartDataBodyPart {
    #[record("BPTN")]
    pub part_name: Option<LString>,
    #[record("BPNN")]
    pub part_node: ZString,
    #[record("BPNT")]
    pub vats_target: ZString,
    #[record("BPND")]
    pub data: BodyPartNodeData,
    #[record("NAM1")]
    pub limb_replacement_model: Option<ZString>,
    #[record("NAM4")]
    pub gore_effects_target_bone: Option<ZString>,
    #[record("NAM5")]
    pub model_info: Option<()>,
    #[record("ENAM")]
    pub hit_reaction_start: Option<ZString>,
    #[record("FNAM")]
    pub hit_reaction_end: Option<ZString>,
    #[record("BNAM")]
    pub gore_effects_dismember_blood_art: Option<FormId>,
    #[record("INAM")]
    pub gore_effects_blood_impact_material_type: Option<FormId>,
    #[record("JNAM")]
    pub on_cripple_blood_impact_material_type: Option<FormId>,
    #[record("CNAM")]
    pub meat_cap_texture_set: Option<FormId>,
    #[record("NAM2")]
    pub collar_texture_set: Option<FormId>,
    #[record("DNAM")]
    pub twist_variable_prefix: Option<ZString>,
}

#[derive(Clone, Debug, Record)]
pub struct BodyPartDataSkeletonModel {
    #[record("MODL")]
    pub model: Option<ZString>,
    #[record("MODT")]
    pub info: Option<ModelInfo>,
    #[record("MODC")]
    pub color_remapping_index: Option<u32>,
    #[record("MODS")]
    pub material_swap: Option<FormId>,
    #[record("MODF")]
    pub flags: Option<u8>,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct BodyPartNodeData {
    pub damage_mult: f32,
    pub explodable_debris: FormId,
    pub explodable_explosion: FormId,
    pub explodable_debris_scale: f32,
    pub severable_debris: FormId,
    pub severable_explosion: FormId,
    pub severable_debris_scale: f32,
    pub cut_min: f32,
    pub cut_max: f32,
    pub cut_radius: f32,
    pub gore_effects_local_rotate_x: f32,
    pub gore_effects_local_rotate_y: f32,
    pub cut_tesselation: f32,
    pub severable_impact_data_set: FormId,
    pub explodable_impact_data_set: FormId,
    pub explodable_limb_replacement_scale: f32,
    pub flags: u8,
    pub part_type: u8,
    pub health_percentage: u8,
    pub actor_value: FormId,
    pub to_hit_chance: u8,
    pub explodable_explosion_chance_percentage: u8,
    pub non_lethal_dismemberment_chance: u8,
    pub severable_debris_count: u8,
    pub explodable_debris_count: u8,
    pub severable_decal_count: u8,
    pub explodable_decal_count: u8,
    pub geometry_segment_index: u8,
    pub on_cripple_art_object: FormId,
    pub on_cripple_debris: FormId,
    pub on_cripple_explosion: FormId,
    pub on_cripple_impact_data_set: FormId,
    pub on_cripple_debris_scale: f32,
    pub on_cripple_debris_count: u8,
    pub on_cripple_decal_count: u8,
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
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("DESC")]
    pub description: Option<ZString>,
    #[record("ANAM")]
    pub abbreviation: Option<LString>,
    #[record("NAM0")]
    pub default_value: Option<f32>,
    #[record("AVFL")]
    pub flags: Option<u32>,
    #[record("NAM1")]
    pub ty: Option<u32>,
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
#[record(ordered)]
pub struct ArmorAddon {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("BOD2")]
    pub biped_body_template: BipedObjectFlags,
    /// Reference to [`Race`].
    #[record("RNAM")]
    pub race: Option<FormId>,
    #[record("DNAM")]
    pub data: (),
    #[record("MOD2")]
    pub male_model: ArmorAddonMaleModel,
    #[record("MOD3")]
    pub female_model: ArmorAddonFemaleModel,
    #[record("MOD4")]
    pub male_first_person_model: ArmorAddonMaleFirstPersonModel,
    #[record("MOD5")]
    pub female_first_person_model: ArmorAddonFemaleFirstPersonModel,
    /// Reference to [`TextureSet`].
    #[record("NAM0")]
    pub male_skin_texture: Option<FormId>,
    /// Reference to [`TextureSet`].
    #[record("NAM1")]
    pub female_skin_texture: Option<FormId>,
    #[record("NAM2")]
    pub male_skin_textures_swap_list: Option<()>,
    #[record("NAM3")]
    pub female_skin_textures_swap_list: Option<()>,
    /// Reference to [`Race`]s.
    #[record("MODL")]
    pub additional_races: Vec<FormId>,
    #[record("SNDD")]
    pub footstep_sound: Option<FormId>,
    #[record("ONAM")]
    pub art_object: Option<FormId>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorAddonMaleModel {
    #[record("MOD2")]
    pub model: Option<ZString>,
    #[record("MO2T")]
    pub model_info: Option<ModelInfo>,
    #[record("MO2C")]
    pub color_remapping_index: Option<()>,
    #[record("MO2S")]
    pub material_swap: Option<()>,
    #[record("MO2F")]
    pub flags: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorAddonFemaleModel {
    #[record("MOD3")]
    pub model: Option<ZString>,
    #[record("MO3T")]
    pub model_info: Option<ModelInfo>,
    #[record("MO3C")]
    pub color_remapping_index: Option<()>,
    #[record("MO3S")]
    pub material_swap: Option<()>,
    #[record("MO3F")]
    pub flags: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorAddonMaleFirstPersonModel {
    #[record("MOD4")]
    pub model: Option<ZString>,
    #[record("MO4T")]
    pub model_info: Option<ModelInfo>,
    #[record("MO4C")]
    pub color_remapping_index: Option<()>,
    #[record("MO4S")]
    pub material_swap: Option<()>,
    #[record("MO4F")]
    pub flags: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ArmorAddonFemaleFirstPersonModel {
    #[record("MOD5")]
    pub model: Option<ZString>,
    #[record("MO5T")]
    pub model_info: Option<ModelInfo>,
    #[record("MO5C")]
    pub color_remapping_index: Option<()>,
    #[record("MO5S")]
    pub material_swap: Option<()>,
    #[record("MO5F")]
    pub flags: Option<()>,
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
    #[record("ACPR")]
    pub added_persistent_refs: Option<Array<LocationPersistentReference>>,
    #[record("LCPR")]
    pub persistent_refs: Option<Array<LocationPersistentReference>>,
    #[record("RCPR")]
    pub removed_persistent_refs: Option<Array<FormId>>,
    #[record("ACUN")]
    pub added_unique_npcs: Option<Array<LocationUniqueNpc>>,
    #[record("LCUN")]
    pub unique_npcs: Option<Array<LocationUniqueNpc>>,
    #[record("RCUN")]
    pub removed_unique_npcs: Option<Array<FormId>>,
    #[record("ACSR")]
    pub added_special_refs: Option<Array<LocationSpecialReference>>,
    #[record("LCSR")]
    pub special_refs: Option<Array<LocationSpecialReference>>,
    #[record("RCSR")]
    pub removed_special_refs: Option<Array<FormId>>,
    // TODO: SOme more fields here...
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("KSIZ")]
    _keyword_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("PNAM")]
    pub parent_location: Option<FormId>,
    #[record("NAM1")]
    pub music: Option<FormId>,
    #[record("FNAM")]
    pub unreported_crime_faction: Option<FormId>,
    #[record("RNAM")]
    pub world_location_radius: Option<f32>,
    #[record("ANAM")]
    pub actor_fade_mult: Option<f32>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LocationPersistentReference {
    pub reference: FormId,
    pub location: FormId,
    pub grid_y: i16,
    pub grid_x: i16,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LocationUniqueNpc {
    pub actor: FormId,
    pub reference: FormId,
    pub location: FormId,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct LocationSpecialReference {
    pub location_ref_type: FormId,
    pub reference: FormId,
    pub location: FormId,
    pub grid_y: i16,
    pub grid_x: i16,
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
    /// Equipment items that are part of the outfit.
    ///
    /// References to [`Armor`] or [`LeveledItem`] forms.
    #[record("INAM")]
    pub items: Array<FormId>,
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
    #[record("MNAM")]
    pub name: Option<ZString>,
    #[record("SPED")]
    pub speeds: MovementSpeeds,
    #[record("JNAM")]
    pub float_height: Option<f32>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MovementSpeeds {
    pub left: MovementSpeed,
    pub right: MovementSpeed,
    pub forward: MovementSpeed,
    pub backward: MovementSpeed,
    // TODO: Pitch Roll Yaw
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MovementSpeed {
    pub standing: f32,
    pub walk: f32,
    pub run: f32,
    pub sprint: f32,
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
    #[record("DNAM")]
    pub data: AimModelData,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct AimModelData {
    pub cone_of_fire_min_angle: f32,
    pub cone_of_fire_max_angle: f32,
    pub cone_of_fire_increase_per_shot: f32,
    pub cone_of_fire_decrease_per_second: f32,
    pub cone_of_fire_decrease_delay_ms: u32,
    pub cone_of_fire_sneak_mult: f32,
    pub recoil_diminish_sprint_force: f32,
    pub recoil_diminish_sights_mult: f32,
    pub recoil_max_per_shot: f32,
    pub recoil_min_per_shot: f32,
    pub recoil_hip_mult: f32,
    pub runaway_recoil_shots: f32,
    pub recoil_arc: f32,
    pub recoil_arc_rotate: f32,
    pub cone_of_fire_iron_sights_mult: f32,
    pub base_stability: f32,
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
