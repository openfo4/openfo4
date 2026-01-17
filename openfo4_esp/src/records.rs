use std::ops::{Deref, DerefMut};

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
    pub conditions: SubRecordArray<Condition>,
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
    pub conditions: SubRecordArray<Condition>,
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
    pub object_template: Option<SubRecordSection<ObjectModificationTemplates>>,
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
pub struct ObjectModificationTemplates {
    #[record("OBTF")]
    pub templates: SubRecordArray<ObjectTemplateCombination>,
    #[record("STOP")]
    _stop: Option<()>,
}

#[derive(Clone, Debug, Record)]
pub struct ObjectTemplateCombination {
    #[record("OBTF")]
    pub editor_only: (),
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
    pub parent_combination_index: i16,
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
        let parent_combination_index = i16::decode(&mut buf)?;
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
    pub quest_dialogue_conditions: SubRecordArray<Condition>,
    #[record("NEXT")]
    _marker: Option<()>,
    #[record("CTDA")]
    pub story_manager_conditions: SubRecordArray<Condition>,
    #[record("INDX")]
    pub stages: SubRecordArray<QuestStage>,
    // ...
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
    pub conditions: SubRecordArray<Condition>,
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
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
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
pub struct ObjectModification {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("DESC")]
    pub description: Option<ZString>,
    #[record("MODL")]
    pub model: Model,
    #[record("DATA")]
    pub data: ObjectModificationData,
    #[record("MNAM")]
    pub target_keywords: Option<Array<FormId>>,
    #[record("FNAM")]
    pub filter_keywords: Option<Array<FormId>>,
    #[record("LNAM")]
    pub loose_mod: Option<FormId>,
    #[record("NAM1")]
    pub priority: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct ObjectModificationData {
    pub form_type: ObjectModificationFormType,
    pub max_rank: u8,
    pub level_tier_scaled_offset: u8,
    pub attach_point: FormId,
    pub attach_parent_slots: Vec<FormId>,
    pub includes: Vec<ObjectModificationInclude>,
    pub properties: Vec<ObjectModificationProperty>,
}

impl Decode for ObjectModificationData {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let include_count = u32::decode(&mut buf)?;
        let property_count = u32::decode(&mut buf)?;
        u16::decode(&mut buf)?;
        let form_type = ObjectModificationFormType::decode(&mut buf)?;

        let max_rank = u8::decode(&mut buf)?;
        let level_tier_scaled_offset = u8::decode(&mut buf)?;
        let attach_point = FormId::decode(&mut buf)?;

        let attach_parent_slots_count = u32::decode(&mut buf)?;
        let attach_parent_slots = (0..attach_parent_slots_count)
            .map(|_| FormId::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        for _ in 0..u32::decode(&mut buf)? {
            u32::decode(&mut buf)?;
            u32::decode(&mut buf)?;
        }

        let includes = (0..include_count)
            .map(|_| ObjectModificationInclude::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        let properties = (0..property_count)
            .map(|_| ObjectModificationProperty::decode(&mut buf, form_type))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            form_type,
            max_rank,
            level_tier_scaled_offset,
            attach_point,
            attach_parent_slots,
            includes,
            properties,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectModificationFormType {
    Armor,
    Npc,
    Weapon,
    None,
}

impl Decode for ObjectModificationFormType {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let bytes = <[u8; 4]>::decode(buf)?;
        match &bytes {
            b"ARMO" => Ok(Self::Armor),
            b"NPC_" => Ok(Self::Npc),
            b"WEAP" => Ok(Self::Weapon),
            b"NONE" => Ok(Self::None),
            _ => Err(Error::BadEnumValue(
                stringify!(ObjectModificationFormType),
                u32::from_le_bytes(bytes).into(),
            )),
        }
    }
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct ObjectModificationInclude {
    pub mod_: FormId,
    pub min_level: u8,
    pub optional: bool,
    pub dont_use_all: bool,
}

#[derive(Clone, Debug)]
pub struct ObjectModificationProperty {
    pub value_type: ObjectModificationValueType,
    pub function_type: ObjectModificationFunctionType,
    pub property: ObjectModificationPropertyProperty,
    pub value1: u32,
    pub value2: u32,
    pub step: f32,
}

impl ObjectModificationProperty {
    fn decode<B>(mut buf: B, form_type: ObjectModificationFormType) -> Result<Self, Error>
    where
        B: Buf,
    {
        let value_type = ObjectModificationValueType::decode(&mut buf)?;
        <[u8; 3]>::decode(&mut buf)?;

        let function_type = u8::decode(&mut buf)?;
        <[u8; 3]>::decode(&mut buf)?;
        let function_type = match value_type {
            ObjectModificationValueType::Float => match function_type {
                0 => ObjectModificationFunctionType::Set,
                1 => ObjectModificationFunctionType::MulAdd,
                2 => ObjectModificationFunctionType::Add,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
            ObjectModificationValueType::Bool => match function_type {
                0 => ObjectModificationFunctionType::Set,
                1 => ObjectModificationFunctionType::And,
                2 => ObjectModificationFunctionType::Or,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
            ObjectModificationValueType::String => {
                return Err(Error::BadEnumValue(
                    stringify!(ObjectModificationFunctionType),
                    function_type.into(),
                ));
            }
            ObjectModificationValueType::Enum => match function_type {
                0 => ObjectModificationFunctionType::Set,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
            ObjectModificationValueType::FormIdFloat | ObjectModificationValueType::FormIdInt => {
                match function_type {
                    0 => ObjectModificationFunctionType::Set,
                    1 => ObjectModificationFunctionType::Rem,
                    2 => ObjectModificationFunctionType::Add,
                    _ => {
                        return Err(Error::BadEnumValue(
                            stringify!(ObjectModificationFunctionType),
                            function_type.into(),
                        ));
                    }
                }
            }
            ObjectModificationValueType::Int => match function_type {
                0 => ObjectModificationFunctionType::Set,
                2 => ObjectModificationFunctionType::Add,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
        };

        let property = u16::decode(&mut buf)?;
        <[u8; 2]>::decode(&mut buf)?;
        let property = match form_type {
            ObjectModificationFormType::Armor => match property {
                0 => ObjectModificationPropertyProperty::Enchantments,
                1 => ObjectModificationPropertyProperty::BashImpactDataSet,
                2 => ObjectModificationPropertyProperty::BlockMaterial,
                3 => ObjectModificationPropertyProperty::Keywords,
                4 => ObjectModificationPropertyProperty::Weight,
                5 => ObjectModificationPropertyProperty::Value,
                6 => ObjectModificationPropertyProperty::Rating,
                7 => ObjectModificationPropertyProperty::AddonIndex,
                8 => ObjectModificationPropertyProperty::BodyPart,
                9 => ObjectModificationPropertyProperty::DamageTypeValue,
                10 => ObjectModificationPropertyProperty::ActorValues,
                11 => ObjectModificationPropertyProperty::Health,
                12 => ObjectModificationPropertyProperty::ColorRemappingIndex,
                13 => ObjectModificationPropertyProperty::MaterialSwaps,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationPropertyProperty),
                        property.into(),
                    ));
                }
            },
            ObjectModificationFormType::Npc => match property {
                0 => ObjectModificationPropertyProperty::Keywords,
                1 => ObjectModificationPropertyProperty::ForcedInventory,
                2 => ObjectModificationPropertyProperty::XpOffset,
                3 => ObjectModificationPropertyProperty::Enchantments,
                4 => ObjectModificationPropertyProperty::ColorRemappingIndex,
                5 => ObjectModificationPropertyProperty::MaterialSwaps,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationPropertyProperty),
                        property.into(),
                    ));
                }
            },
            ObjectModificationFormType::Weapon => match property {
                0 => ObjectModificationPropertyProperty::Speed,
                1 => ObjectModificationPropertyProperty::Reach,
                2 => ObjectModificationPropertyProperty::MinRange,
                3 => ObjectModificationPropertyProperty::MaxRange,
                4 => ObjectModificationPropertyProperty::AttackDelaySec,
                5 => ObjectModificationPropertyProperty::Unknown,
                6 => ObjectModificationPropertyProperty::OutOfRangeDamageMult,
                7 => ObjectModificationPropertyProperty::SecondaryDamage,
                8 => ObjectModificationPropertyProperty::CriticalChargeBonus,
                9 => ObjectModificationPropertyProperty::HitBehavior,
                10 => ObjectModificationPropertyProperty::Rank,
                11 => ObjectModificationPropertyProperty::Unknown,
                12 => ObjectModificationPropertyProperty::AmmoCapacity,
                13 => ObjectModificationPropertyProperty::Unknown,
                14 => ObjectModificationPropertyProperty::Unknown,
                15 => ObjectModificationPropertyProperty::Type,
                16 => ObjectModificationPropertyProperty::IsPlayerOnly,
                17 => ObjectModificationPropertyProperty::NpcsUseAmmo,
                18 => ObjectModificationPropertyProperty::HasChargingReload,
                19 => ObjectModificationPropertyProperty::IsMinorCrime,
                20 => ObjectModificationPropertyProperty::IsFixedRange,
                21 => ObjectModificationPropertyProperty::HasEffectOnDeath,
                22 => ObjectModificationPropertyProperty::HasAlternateRumble,
                23 => ObjectModificationPropertyProperty::IsNonHostile,
                24 => ObjectModificationPropertyProperty::IgnoreResist,
                25 => ObjectModificationPropertyProperty::IsAutomatic,
                26 => ObjectModificationPropertyProperty::CantDrop,
                27 => ObjectModificationPropertyProperty::IsNonPlayable,
                28 => ObjectModificationPropertyProperty::AttackDamage,
                29 => ObjectModificationPropertyProperty::Value,
                30 => ObjectModificationPropertyProperty::Weight,
                31 => ObjectModificationPropertyProperty::Keywords,
                32 => ObjectModificationPropertyProperty::AimModel,
                33 => ObjectModificationPropertyProperty::AimModelMinConeDegrees,
                34 => ObjectModificationPropertyProperty::AimModelMaxConeDegrees,
                35 => ObjectModificationPropertyProperty::AimModelConeIncreasePerShot,
                36 => ObjectModificationPropertyProperty::AimModelConeDecreasePerSec,
                37 => ObjectModificationPropertyProperty::AimModelConeDecreaseDelayMs,
                38 => ObjectModificationPropertyProperty::AimModelConeSneakMultiplier,
                39 => ObjectModificationPropertyProperty::AimModelRecoilDiminishSpringForce,
                40 => ObjectModificationPropertyProperty::AimModelRecoilDiminishSightsMult,
                41 => ObjectModificationPropertyProperty::AimModelRecoilMaxDegPerShot,
                42 => ObjectModificationPropertyProperty::AimModelRecoilMinDegPerShot,
                43 => ObjectModificationPropertyProperty::AimModelRecoilHipMult,
                44 => ObjectModificationPropertyProperty::AimModelRecoilShotsForRunaway,
                45 => ObjectModificationPropertyProperty::AimModelRecoilArcDeg,
                46 => ObjectModificationPropertyProperty::AimModelRecoilArcRotateDeg,
                47 => ObjectModificationPropertyProperty::AimModelConeIronSightsMultiplier,
                48 => ObjectModificationPropertyProperty::HasScope,
                49 => ObjectModificationPropertyProperty::ZoomDataFovMult,
                50 => ObjectModificationPropertyProperty::FireSeconds,
                51 => ObjectModificationPropertyProperty::NumProjectiles,
                52 => ObjectModificationPropertyProperty::AttackSound,
                53 => ObjectModificationPropertyProperty::AttackSound2D,
                54 => ObjectModificationPropertyProperty::AttackLoop,
                55 => ObjectModificationPropertyProperty::AttackFailSound,
                56 => ObjectModificationPropertyProperty::IdleSound,
                57 => ObjectModificationPropertyProperty::EquipSound,
                58 => ObjectModificationPropertyProperty::UnequipSound,
                59 => ObjectModificationPropertyProperty::SoundLevel,
                60 => ObjectModificationPropertyProperty::ImpactDataSet,
                61 => ObjectModificationPropertyProperty::Ammo,
                62 => ObjectModificationPropertyProperty::CritEffect,
                63 => ObjectModificationPropertyProperty::BashImpactDataSet,
                64 => ObjectModificationPropertyProperty::BlockMaterial,
                65 => ObjectModificationPropertyProperty::Enchantments,
                66 => ObjectModificationPropertyProperty::AimModelBaseStability,
                67 => ObjectModificationPropertyProperty::ZoomData,
                68 => ObjectModificationPropertyProperty::ZoomDataOverlay,
                69 => ObjectModificationPropertyProperty::ZoomDataImageSpace,
                70 => ObjectModificationPropertyProperty::ZoomDataCameraOffsetX,
                71 => ObjectModificationPropertyProperty::ZoomDataCameraOffsetY,
                72 => ObjectModificationPropertyProperty::ZoomDataCameraOffsetZ,
                73 => ObjectModificationPropertyProperty::EquipSlot,
                74 => ObjectModificationPropertyProperty::SoundLevelMult,
                75 => ObjectModificationPropertyProperty::NpcAmmoList,
                76 => ObjectModificationPropertyProperty::ReloadSpeed,
                77 => ObjectModificationPropertyProperty::DamageTypeValues,
                78 => ObjectModificationPropertyProperty::AccuracyBonus,
                79 => ObjectModificationPropertyProperty::AttackActionPointCost,
                80 => ObjectModificationPropertyProperty::OverrideProjectile,
                81 => ObjectModificationPropertyProperty::HasBoltAction,
                82 => ObjectModificationPropertyProperty::StaggerValue,
                83 => ObjectModificationPropertyProperty::SightedTransitionSeconds,
                84 => ObjectModificationPropertyProperty::FullPowerSeconds,
                85 => ObjectModificationPropertyProperty::HoldInputToPower,
                86 => ObjectModificationPropertyProperty::HasRepeatableSingleFire,
                87 => ObjectModificationPropertyProperty::MinPowerPerShot,
                88 => ObjectModificationPropertyProperty::ColorRemappingIndex,
                89 => ObjectModificationPropertyProperty::MaterialSwaps,
                90 => ObjectModificationPropertyProperty::CriticalDamageMult,
                91 => ObjectModificationPropertyProperty::FastEquipSound,
                92 => ObjectModificationPropertyProperty::DisableShells,
                93 => ObjectModificationPropertyProperty::HasChargingAttack,
                94 => ObjectModificationPropertyProperty::ActorValues,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationPropertyProperty),
                        property.into(),
                    ));
                }
            },
            ObjectModificationFormType::None => {
                return Err(Error::BadEnumValue(
                    stringify!(ObjectModificationPropertyProperty),
                    property.into(),
                ));
            }
        };

        let value1 = u32::decode(&mut buf)?;
        let value2 = u32::decode(&mut buf)?;
        let step = f32::decode(&mut buf)?;

        Ok(Self {
            value_type,
            function_type,
            property,
            value1,
            value2,
            step,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum ObjectModificationValueType {
    Int = 0,
    Float = 1,
    Bool = 2,
    String = 3,
    FormIdInt = 4,
    Enum = 5,
    FormIdFloat = 6,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectModificationFunctionType {
    Set,
    MulAdd,
    Add,
    And,
    Or,
    Rem,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectModificationPropertyProperty {
    Enchantments,
    BashImpactDataSet,
    BlockMaterial,
    Keywords,
    Weight,
    Value,
    Rating,
    AddonIndex,
    BodyPart,
    DamageTypeValue,
    ActorValues,
    Health,
    ColorRemappingIndex,
    MaterialSwaps,
    ForcedInventory,
    XpOffset,
    Speed,
    Reach,
    MinRange,
    MaxRange,
    AttackDelaySec,
    OutOfRangeDamageMult,
    SecondaryDamage,
    CriticalChargeBonus,
    HitBehavior,
    Rank,
    AmmoCapacity,
    Type,
    IsPlayerOnly,
    NpcsUseAmmo,
    HasChargingReload,
    IsMinorCrime,
    IsFixedRange,
    HasEffectOnDeath,
    HasAlternateRumble,
    IsNonHostile,
    IgnoreResist,
    IsAutomatic,
    CantDrop,
    IsNonPlayable,
    AttackDamage,
    AimModel,
    AimModelMinConeDegrees,
    AimModelMaxConeDegrees,
    AimModelConeIncreasePerShot,
    AimModelConeDecreasePerSec,
    AimModelConeDecreaseDelayMs,
    AimModelConeSneakMultiplier,
    AimModelRecoilDiminishSpringForce,
    AimModelRecoilDiminishSightsMult,
    AimModelRecoilMaxDegPerShot,
    AimModelRecoilMinDegPerShot,
    AimModelRecoilHipMult,
    AimModelRecoilShotsForRunaway,
    AimModelRecoilArcDeg,
    AimModelRecoilArcRotateDeg,
    AimModelConeIronSightsMultiplier,
    HasScope,
    ZoomDataFovMult,
    FireSeconds,
    NumProjectiles,
    AttackSound,
    AttackSound2D,
    AttackLoop,
    AttackFailSound,
    IdleSound,
    EquipSound,
    UnequipSound,
    SoundLevel,
    ImpactDataSet,
    Ammo,
    CritEffect,
    AimModelBaseStability,
    ZoomData,
    ZoomDataOverlay,
    ZoomDataImageSpace,
    ZoomDataCameraOffsetX,
    ZoomDataCameraOffsetY,
    ZoomDataCameraOffsetZ,
    EquipSlot,
    SoundLevelMult,
    NpcAmmoList,
    ReloadSpeed,
    DamageTypeValues,
    AccuracyBonus,
    AttackActionPointCost,
    OverrideProjectile,
    HasBoltAction,
    StaggerValue,
    SightedTransitionSeconds,
    FullPowerSeconds,
    HoldInputToPower,
    HasRepeatableSingleFire,
    MinPowerPerShot,
    CriticalDamageMult,
    FastEquipSound,
    DisableShells,
    HasChargingAttack,
    Unknown,
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
