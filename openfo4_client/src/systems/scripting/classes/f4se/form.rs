use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Form", "GetName", get_name);
    engine.register_native_function("Form", "SetName", set_name);
    engine.register_native_function("Form", "GetDescription", get_description);
    engine.register_native_function("Form", "GetWeight", get_weight);
    engine.register_native_function("Form", "SetWeight", set_weight);
    engine.register_native_function("Form", "SetGoldValue", set_gold_value);
    engine.register_native_function("Form", "GetKeywords", get_keywords);
    engine.register_native_function("Form", "HasWorldModel", has_world_model);
    engine.register_native_function("Form", "GetWorldModelPath", get_world_model_path);
    engine.register_native_function("Form", "SetWorldModelPath", set_world_model_path);
    engine.register_native_function("Form", "GetIconPath", get_icon_path);
    engine.register_native_function("Form", "SetIconPath", set_icon_path);
    engine.register_native_function("Form", "GetMessageIconPath", get_message_icon_path);
    engine.register_native_function("Form", "SetMessageIconPath", set_message_icon_path);
    engine.register_native_function("Form", "GetEnchantment", get_enchantment);
    engine.register_native_function("Form", "SetEnchantment", set_enchantment);
    engine.register_native_function("Form", "GetEnchantmentValue", get_enchantment_value);
    engine.register_native_function("Form", "SetEnchantmentValue", set_enchantment_value);
    engine.register_native_function("Form", "GetEquipType", get_equip_type);
    engine.register_native_function("Form", "SetEquipType", set_equip_type);
    engine.register_native_function("Form", "GetRaceForm", get_race_form);
    engine.register_native_function("Form", "SetRaceForm", set_race_form);
    engine.register_native_function("Form", "GetSlotMask", get_slot_mask);
    engine.register_native_function("Form", "SetSlotMask", set_slot_mask);
    engine.register_native_function("Form", "AddSlotToMask", add_slot_to_mask);
    engine.register_native_function("Form", "RemoveSlotFromMask", remove_slot_from_mask);
    engine.register_native_function("Form", "GetMaskForSlot", get_mask_for_slot);
}

/// `GetName`
///
/// ```text
/// string Function() native
/// ```
fn get_name(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `SetName`
///
/// ```text
/// Function(string name) native
/// ```
fn set_name(caller: Caller<'_, State>, self_ptr: Object, name: isize) -> Result<(), Exception> {
    todo!()
}

/// `GetDescription`
///
/// ```text
/// string Function() native
/// ```
fn get_description(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `GetWeight`
///
/// ```text
/// float Function() native
/// ```
fn get_weight(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `SetWeight`
///
/// ```text
/// Function(float weight) native
/// ```
fn set_weight(caller: Caller<'_, State>, self_ptr: Object, weight: f32) -> Result<(), Exception> {
    todo!()
}

/// `SetGoldValue`
///
/// ```text
/// Function(int value) native
/// ```
fn set_gold_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    value: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `GetKeywords`
///
/// ```text
/// Keyword[] Function() native
/// ```
fn get_keywords(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `HasWorldModel`
///
/// ```text
/// bool Function() native
/// ```
fn has_world_model(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetWorldModelPath`
///
/// ```text
/// string Function() native
/// ```
fn get_world_model_path(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `SetWorldModelPath`
///
/// ```text
/// Function(string path) native
/// ```
fn set_world_model_path(
    caller: Caller<'_, State>,
    self_ptr: Object,
    path: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `GetIconPath`
///
/// ```text
/// string Function() native
/// ```
fn get_icon_path(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `SetIconPath`
///
/// ```text
/// Function(string path) native
/// ```
fn set_icon_path(
    caller: Caller<'_, State>,
    self_ptr: Object,
    path: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `GetMessageIconPath`
///
/// ```text
/// string Function() native
/// ```
fn get_message_icon_path(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `SetMessageIconPath`
///
/// ```text
/// Function(string path) native
/// ```
fn set_message_icon_path(
    caller: Caller<'_, State>,
    self_ptr: Object,
    path: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `GetEnchantment`
///
/// ```text
/// Enchantment Function() native
/// ```
fn get_enchantment(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetEnchantment`
///
/// ```text
/// Function(Enchantment e) native
/// ```
fn set_enchantment(
    caller: Caller<'_, State>,
    self_ptr: Object,
    e: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetEnchantmentValue`
///
/// ```text
/// int Function() native
/// ```
fn get_enchantment_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetEnchantmentValue`
///
/// ```text
/// Function(int value) native
/// ```
fn set_enchantment_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    value: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `GetEquipType`
///
/// ```text
/// EquipSlot Function() native
/// ```
fn get_equip_type(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetEquipType`
///
/// ```text
/// Function(EquipSlot type) native
/// ```
fn set_equip_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    r#type: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetRaceForm`
///
/// ```text
/// Race Function() native
/// ```
fn get_race_form(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetRaceForm`
///
/// ```text
/// Function(Race newRace) native
/// ```
fn set_race_form(
    caller: Caller<'_, State>,
    self_ptr: Object,
    new_race: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetSlotMask`
///
/// ```text
/// int Function() native
/// ```
fn get_slot_mask(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetSlotMask`
///
/// ```text
/// Function(int slotMask) native
/// ```
fn set_slot_mask(
    caller: Caller<'_, State>,
    self_ptr: Object,
    slot_mask: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `AddSlotToMask`
///
/// ```text
/// int Function(int slotMask) native
/// ```
fn add_slot_to_mask(
    caller: Caller<'_, State>,
    self_ptr: Object,
    slot_mask: i32,
) -> Result<i32, Exception> {
    todo!()
}

/// `RemoveSlotFromMask`
///
/// ```text
/// int Function(int slotMask) native
/// ```
fn remove_slot_from_mask(
    caller: Caller<'_, State>,
    self_ptr: Object,
    slot_mask: i32,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetMaskForSlot`
///
/// ```text
/// int Function(int slot) native global
/// ```
fn get_mask_for_slot(caller: Caller<'_, State>, slot: i32) -> Result<i32, Exception> {
    todo!()
}
