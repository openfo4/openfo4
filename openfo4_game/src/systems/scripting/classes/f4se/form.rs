use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Form", "GetName", get_name);
    engine.register_if_exists("Form", "SetName", set_name);
    engine.register_if_exists("Form", "GetDescription", get_description);
    engine.register_if_exists("Form", "GetWeight", get_weight);
    engine.register_if_exists("Form", "SetWeight", set_weight);
    engine.register_if_exists("Form", "SetGoldValue", set_gold_value);
    engine.register_if_exists("Form", "GetKeywords", get_keywords);
    engine.register_if_exists("Form", "HasWorldModel", has_world_model);
    engine.register_if_exists("Form", "GetWorldModelPath", get_world_model_path);
    engine.register_if_exists("Form", "SetWorldModelPath", set_world_model_path);
    engine.register_if_exists("Form", "GetIconPath", get_icon_path);
    engine.register_if_exists("Form", "SetIconPath", set_icon_path);
    engine.register_if_exists("Form", "GetMessageIconPath", get_message_icon_path);
    engine.register_if_exists("Form", "SetMessageIconPath", set_message_icon_path);
    engine.register_if_exists("Form", "GetEnchantment", get_enchantment);
    engine.register_if_exists("Form", "SetEnchantment", set_enchantment);
    engine.register_if_exists("Form", "GetEnchantmentValue", get_enchantment_value);
    engine.register_if_exists("Form", "SetEnchantmentValue", set_enchantment_value);
    engine.register_if_exists("Form", "GetEquipType", get_equip_type);
    engine.register_if_exists("Form", "SetEquipType", set_equip_type);
    engine.register_if_exists("Form", "GetRaceForm", get_race_form);
    engine.register_if_exists("Form", "SetRaceForm", set_race_form);
    engine.register_if_exists("Form", "GetSlotMask", get_slot_mask);
    engine.register_if_exists("Form", "SetSlotMask", set_slot_mask);
    engine.register_if_exists("Form", "AddSlotToMask", add_slot_to_mask);
    engine.register_if_exists("Form", "RemoveSlotFromMask", remove_slot_from_mask);
    engine.register_if_exists("Form", "GetMaskForSlot", get_mask_for_slot);
}

/// `GetName`
///
/// ```text
/// string Function() native
/// ```
fn get_name(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetName`
///
/// ```text
/// Function(string name) native
/// ```
fn set_name(_caller: Caller<'_, State>, _self_ptr: Object, _name: Object) -> Result<(), Error> {
    todo!()
}

/// `GetDescription`
///
/// ```text
/// string Function() native
/// ```
fn get_description(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetWeight`
///
/// ```text
/// float Function() native
/// ```
fn get_weight(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `SetWeight`
///
/// ```text
/// Function(float weight) native
/// ```
fn set_weight(_caller: Caller<'_, State>, _self_ptr: Object, _weight: f32) -> Result<(), Error> {
    todo!()
}

/// `SetGoldValue`
///
/// ```text
/// Function(int value) native
/// ```
fn set_gold_value(_caller: Caller<'_, State>, _self_ptr: Object, _value: i32) -> Result<(), Error> {
    todo!()
}

/// `GetKeywords`
///
/// ```text
/// Keyword[] Function() native
/// ```
fn get_keywords(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `HasWorldModel`
///
/// ```text
/// bool Function() native
/// ```
fn has_world_model(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetWorldModelPath`
///
/// ```text
/// string Function() native
/// ```
fn get_world_model_path(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetWorldModelPath`
///
/// ```text
/// Function(string path) native
/// ```
fn set_world_model_path(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _path: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetIconPath`
///
/// ```text
/// string Function() native
/// ```
fn get_icon_path(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetIconPath`
///
/// ```text
/// Function(string path) native
/// ```
fn set_icon_path(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _path: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetMessageIconPath`
///
/// ```text
/// string Function() native
/// ```
fn get_message_icon_path(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetMessageIconPath`
///
/// ```text
/// Function(string path) native
/// ```
fn set_message_icon_path(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _path: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetEnchantment`
///
/// ```text
/// Enchantment Function() native
/// ```
fn get_enchantment(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetEnchantment`
///
/// ```text
/// Function(Enchantment e) native
/// ```
fn set_enchantment(_caller: Caller<'_, State>, _self_ptr: Object, _e: Object) -> Result<(), Error> {
    todo!()
}

/// `GetEnchantmentValue`
///
/// ```text
/// int Function() native
/// ```
fn get_enchantment_value(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetEnchantmentValue`
///
/// ```text
/// Function(int value) native
/// ```
fn set_enchantment_value(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _value: i32,
) -> Result<(), Error> {
    todo!()
}

/// `GetEquipType`
///
/// ```text
/// EquipSlot Function() native
/// ```
fn get_equip_type(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetEquipType`
///
/// ```text
/// Function(EquipSlot type) native
/// ```
fn set_equip_type(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    r#_type: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetRaceForm`
///
/// ```text
/// Race Function() native
/// ```
fn get_race_form(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetRaceForm`
///
/// ```text
/// Function(Race newRace) native
/// ```
fn set_race_form(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _new_race: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetSlotMask`
///
/// ```text
/// int Function() native
/// ```
fn get_slot_mask(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetSlotMask`
///
/// ```text
/// Function(int slotMask) native
/// ```
fn set_slot_mask(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _slot_mask: i32,
) -> Result<(), Error> {
    todo!()
}

/// `AddSlotToMask`
///
/// ```text
/// int Function(int slotMask) native
/// ```
fn add_slot_to_mask(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _slot_mask: i32,
) -> Result<i32, Error> {
    todo!()
}

/// `RemoveSlotFromMask`
///
/// ```text
/// int Function(int slotMask) native
/// ```
fn remove_slot_from_mask(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _slot_mask: i32,
) -> Result<i32, Error> {
    todo!()
}

/// `GetMaskForSlot`
///
/// ```text
/// int Function(int slot) native global
/// ```
fn get_mask_for_slot(_caller: Caller<'_, State>, _slot: i32) -> Result<i32, Error> {
    todo!()
}
