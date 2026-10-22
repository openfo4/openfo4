use openfo4_esp_derive::{BinaryStruct, Record};

use crate::fields::{SubField, SubRecordArray};
use crate::records::{Array, Bounds, Destruction, Effect, Model};
use crate::vmad::VirtualMachineAdapter;
use crate::{FormId, LString, ZString};

#[derive(Clone, Debug, Record)]
#[record(ordered)]
pub struct Ingredient {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("VMAD")]
    pub virtual_machine_adapter: Option<VirtualMachineAdapter>,
    #[record("OBND")]
    pub object_bounds: Bounds,
    #[record("PTRN")]
    pub preview_transform: Option<FormId>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("KSIZ")]
    _keywords_count: Option<u32>,
    #[record("KWDA")]
    pub keywords: Option<Array<FormId>>,
    #[record("MODL")]
    pub model: Model,
    #[record("ICON")]
    pub inventory_image: Option<ZString>,
    #[record("MICO")]
    pub message_icon: Option<ZString>,
    #[record("DEST")]
    pub destruction: Option<SubField<Destruction>>,
    #[record("ETYP")]
    pub equipment_type: Option<FormId>,
    #[record("YNAM")]
    pub sound_pickup: Option<FormId>,
    #[record("ZNAM")]
    pub sound_drop: Option<FormId>,
    #[record("DATA")]
    pub data: IngredientData,
    #[record("ENIT")]
    pub effect_data: IngredientEffectData,
    #[record("EFID")]
    pub effects: SubRecordArray<Effect>,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct IngredientData {
    pub value: u32,
    pub weight: f32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
pub struct IngredientEffectData {
    pub value: u32,
    pub flags: u32,
}
