use bitflags::bitflags;
use bytes::Buf;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::conditions::Conditions;
use crate::fields::{SubField, SubRecordArray};
use crate::records::{Array, impl_decode_bitflags};
use crate::vmad::VirtualMachineAdapter;
use crate::{Decode, Error, FormId, Text, ZString};

/// An AI package.
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
    #[record("UNAM")]
    pub data_input_named: SubRecordArray<PackageDataInput>,
    // TODO: OnBegin
    // TODO: OnEnd
    // TODO: OnChange
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct PackageData {
    pub flags: u32,
    pub ty: PackageType,
    pub interrupt_override: PackageInterruptOverride,
    /// The preferred movement speed mode while executing this [`Package`]
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
    pub set_interrupt_flags: u16,
    pub clear_interrupt_flags: u16,
    pub preferred_speed_override: PackagePreferredSpeed,
    _pad0: [u8; 3],
}
