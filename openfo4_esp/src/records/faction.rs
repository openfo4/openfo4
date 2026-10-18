use bitflags::bitflags;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::conditions::Conditions;
use crate::fields::SubRecordArray;
use crate::records::impl_decode_bitflags;
use crate::{FormId, LString, ZString};

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct Faction {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("XNAM")]
    pub relations: Vec<FactionRelation>,
    #[record("DATA")]
    pub data: FactionFlags,
    #[record("JAIL")]
    pub jail_marker: Option<FormId>,
    #[record("WAIT")]
    pub follower_wait_marker: Option<FormId>,
    #[record("STOL")]
    pub stolen_goods_container: Option<FormId>,
    #[record("PLCN")]
    pub player_inventory_container: Option<FormId>,
    /// The list of [`Faction`]s that this [`Faction`] shares crime values with.
    ///
    /// Reference to a [`FormList`].
    ///
    /// [`FormList`]: crate::FormList
    #[record("CRGR")]
    pub shared_crime_faction_list: Option<FormId>,
    /// Reference to an [`Outfit`].
    ///
    /// [`Outfit`]: crate::Outfit
    #[record("JOUT")]
    pub jail_outfit: Option<FormId>,
    #[record("CRVA")]
    pub crime_values: Option<FactionCrimeValues>,
    #[record("RNAM")]
    pub ranks: SubRecordArray<FactionRank>,
    /// Reference to a [`FormList`].
    ///
    /// [`FormList`]: crate::FormList
    #[record("VEND")]
    pub vendor_buy_sell_list: Option<FormId>,
    /// Reference to a [`PlacedObject`].
    ///
    /// [`PlacedObject`]: crate::PlacedObject
    #[record("VENC")]
    pub vendor_container: Option<FormId>,
    #[record("VENV")]
    pub vendor_values: Option<FactionVendorValues>,
    #[record("CITC")]
    _condition_count: Option<u32>,
    #[record("CTDA")]
    pub conditions: Conditions,
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct FactionRelation {
    /// The other [`Faction`].
    ///
    /// Reference to a [`Faction`].
    pub faction: FormId,
    pub modifier: u32,
    pub reaction: FactionCombatReaction,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u32)]
pub enum FactionCombatReaction {
    Neutral = 0,
    Enemy = 1,
    Ally = 2,
    Friend = 3,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FactionFlags: u32 {
        const HIDDEN_FROM_NPC = 1 << 0;
        const SPECIAL_COMBAT = 1 << 1;
        const TRACK_CRIME = 1 << 6;
        const IGNORE_CRIMES_MURDER = 1 << 7;
        const IGNORE_CRIMES_ASSAULT = 1 << 8;
        const IGNORE_CRIMES_STEALING = 1 << 9;
        const IGNORE_CRIMES_TRESPASS =  1 << 10;
        const IGNORE_CRIMES_AGAINST_MEMBERS = 1 << 11;
        const CRIME_HOLD_USE_DEFAULTS = 1 << 12;
        const IGNORE_CRIMES_PICKPOCKET = 1 << 13;
        const VENDOR = 1 << 14;
        const CAN_BE_OWNER = 1 << 15;
        const IGNORE_CRIMES_WEREWOLF = 1 << 16;
    }
}

impl_decode_bitflags!(FactionFlags: u32);

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FactionCrimeValues {
    pub arrest: bool,
    pub attack_on_sight: bool,
    pub murder: u16,
    pub assault: u16,
    pub trespass: u16,
    pub pickpocket: u16,
    _pad0: u16,
    pub steal_factor: f32,
    pub escape: u16,
    pub werewolf: u16,
}

#[derive(Clone, Debug, Record)]
pub struct FactionRank {
    #[record("RNAM")]
    pub rank: u32,
    #[record("MNAM")]
    pub male_title: Option<LString>,
    #[record("FNAM")]
    pub female_title: Option<LString>,
    #[record("INAM")]
    pub insignia: Option<ZString>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct FactionVendorValues {
    pub start_hour: u16,
    pub end_hour: u16,
    pub radius: u16,
    _pad0: u16,
    pub buys_stolen_items: bool,
    pub buy_sell_everything_not_in_list: bool,
    pub buys_non_stolen_items: bool,
    _pad1: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct FactionLocation {
    /// TODO: Decode this properly.
    pub ty: u32,
    /// TODO: Decode this properly.
    pub value: u32,
    pub radius: i32,
    pub collection_index: u32,
}
