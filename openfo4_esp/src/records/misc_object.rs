use bstr::BString;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::records::{Array, Bounds, Model};
use crate::vmad::VirtualMachineAdapter;
use crate::{FormId, LString};

/// A miscellaneous item.
#[derive(Clone, Debug, Record, Default)]
#[non_exhaustive]
pub struct MiscObject {
    #[record("EDID")]
    pub id: Option<BString>,
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
    pub inventory_image: Option<BString>,
    #[record("MICO")]
    pub message_icon: Option<BString>,
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

#[derive(Copy, Clone, Debug, Default, BinaryStruct)]
#[non_exhaustive]
pub struct MiscObjectData {
    pub value: u32,
    pub weight: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[non_exhaustive]
pub struct MiscObjectComponents {
    pub component: FormId,
    pub count: u32,
}
