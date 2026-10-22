use bitflags::bitflags;
use glam::Vec3;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::conditions::Conditions;
use crate::fields::{SubField, SubRecordArray};
use crate::records::{
    Array, Bounds, ContainerItem, Destruction, Model, NavMeshGeometry, ObjectTemplates, Property,
    Rgba, impl_decode_bitflags,
};
use crate::vmad::VirtualMachineAdapter;
use crate::{FormId, LString, ZString};

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct Furniture {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    /// Reference to a [`Transform`] record.
    ///
    /// [`Transform`]: crate::TRNS
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("MODL")]
    pub model: Model,
    #[record("DEST")]
    pub destruction: Option<SubField<Destruction>>,
    #[record("KSIZ")]
    _keywords_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("PRPS")]
    pub properties: Option<Array<Property>>,
    /// Reference to a [`Terminal`] record.
    ///
    /// [`Terminal`]: crate::Terminal
    #[record("NTRM")]
    pub native_terminal: Option<FormId>,
    /// Reference to a [`LocationReferenceType`].
    ///
    /// [`LocationReferenceType`]: crate::LocationReferenceType
    #[record("FTYP")]
    pub forced_loc_ref_type: Option<FormId>,
    #[record("PNAM")]
    pub marker_color: Rgba<u8>,
    /// Reference to a [`Water`] record.
    ///
    /// [`Water`]: crate::Water
    #[record("WNAM")]
    pub drinking_water_type: Option<FormId>,
    #[record("ATTX")]
    pub activate_text_override: Option<LString>,
    #[record("FNAM")]
    pub flags: FurnitureFlags,
    #[record("CITC")]
    _conditions_count: Option<u32>,
    #[record("CTDA")]
    pub conditions: Conditions,
    #[record("COCT")]
    _items_count: Option<u32>,
    #[record("CNTO")]
    pub items: Vec<ContainerItem>,
    #[record("MNAM")]
    pub marker_flags: FurnitureMarkerFlags,
    #[record("WBDT")]
    pub workbench: Option<WorkbenchType>,
    #[record("NAM1")]
    pub associated_form: Option<FormId>,
    #[record("ENAM")]
    pub markers: SubRecordArray<FurnitureMarker>,
    #[record("FNPR")]
    pub marker_entry_points: Vec<FurnitureMarkerEntryPoint>,
    #[record("XMRK")]
    pub marker_model: Option<ZString>,
    #[record("SNAM")]
    pub marker_parameters: Vec<MarkerParameters>,
    #[record("APPR")]
    pub attach_parent_slots: Vec<FormId>,
    #[record("OBTE")]
    pub object_templates: ObjectTemplates,
    #[record("NVNM")]
    pub navmesh: Option<NavMeshGeometry>,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FurnitureFlags: u16 {
        const IGNORED_BY_SANDBOX = 1 << 1;
    }
}

impl_decode_bitflags!(FurnitureFlags: u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
pub enum WorkbenchType {
    None = 0,
    CreateObject = 1,
    Weapons = 2,
    Enchanting = 3,
    EnchantingExperiment = 4,
    /// Chemistry/Cooking
    Alchemy = 5,
    AlchemyExperiment = 6,
    Armor = 7,
    PowerArmor = 8,
    RobotMod = 9,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FurnitureMarkerFlags: u32 {
        const ALLOW_AWAKE_SOUND = 1 << 22;
        const ENTER_WITH_WEAPON_DRAWN = 1 << 23;
        const PLAY_ANIM_WHEN_FULL = 1 << 24;
        const DISABLES_ACTIVATION = 1 << 25;
        const IS_PERCH = 1 << 26;
        const MUST_EXIT_TO_TALK = 1 << 27;
        const USE_STATIC_AVOID_NODE = 1 << 28;
        const HAS_MODEL = 1 << 30;
        const IS_SLEEP_FURNITURE = 1 << 31;
    }
}

impl_decode_bitflags!(FurnitureMarkerFlags: u32);

#[derive(Copy, Clone, Debug, Record)]
pub struct FurnitureMarker {
    #[record("ENAM")]
    pub index: u32,
    #[record("NAM0")]
    pub disabled_entry_points: FurnitureMarkerEntryPoint,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FurnitureMarkerEntryPoint {
    pub type_: FurnitureAnimType,
    pub entry_points: FurnitureEntryTypeFlags,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u16)]
pub enum FurnitureAnimType {
    _0 = 0,
    Sit = 1,
    Lay = 2,
    _3 = 3,
    Lean = 4,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FurnitureEntryTypeFlags: u16 {
        const FRONT = 1 << 0;
        const BACK = 1 << 1;
        const RIGHT = 1 << 2;
        const LEFT = 1 << 3;
        const TOP = 1 << 4;
    }
}

impl_decode_bitflags!(FurnitureEntryTypeFlags: u16);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct MarkerParameters {
    pub offset: Vec3,
    pub rotation_z: f32,
    /// Reference to a [`Keyword`] record.
    ///
    /// [`Keyword`]: crate::Keyword
    pub keyword: FormId,
    #[default_value]
    pub entry_types: FurnitureEntryTypeFlags2,
    #[default_value]
    _pad0: [u8; 3],
}

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FurnitureEntryTypeFlags2: u8 {
        const FRONT = 1 << 0;
        const BACK = 1 << 1;
        const RIGHT = 1 << 2;
        const LEFT = 1 << 3;
        const OTHER = 1 << 4;
    }
}

impl_decode_bitflags!(FurnitureEntryTypeFlags2: u8);
