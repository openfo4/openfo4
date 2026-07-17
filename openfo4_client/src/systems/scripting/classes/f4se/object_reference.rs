use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ObjectReference", "ApplyMaterialSwap", apply_material_swap);
    engine.register_if_exists("ObjectReference", "AttachWire", attach_wire);
    engine.register_if_exists("ObjectReference", "GetAllMods", get_all_mods);
    engine.register_if_exists("ObjectReference", "GetConnectPoints", get_connect_points);
    engine.register_if_exists("ObjectReference", "GetConnectedObjects", get_connected_objects);
    engine.register_if_exists("ObjectReference", "GetDisplayName", get_display_name);
    engine.register_if_exists("ObjectReference", "GetInventoryItems", get_inventory_items);
    engine.register_if_exists("ObjectReference", "GetInventoryWeight", get_inventory_weight);
    engine.register_if_exists("ObjectReference", "GetMaterialSwap", get_material_swap);
    engine.register_if_exists("ObjectReference", "SetMaterialSwap", set_material_swap);
    engine.register_if_exists("ObjectReference", "TransmitConnectedPower", transmit_connected_power);
}

/// `ApplyMaterialSwap`
///
/// ```text
/// MatSwap:RemapData[] Function ApplyMaterialSwap(MatSwap mSwap, bool renameMaterial = false) Native
/// ```
fn apply_material_swap(
    caller: Caller<'_, State>,
    self_ptr: Object,
    material_swap: Object,
    rename_material: bool,
) -> Result<Object, Error> {
    todo!()
}

/// `AttachWire`
///
/// ```text
/// ObjectReference Function AttachWire(ObjectReference akRef, Form akSpline = None) Native
/// ```
fn attach_wire(
    caller: Caller<'_, State>,
    self_ptr: Object,
    object: Object,
    spline: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `GetAllMods`
///
/// ```text
/// ObjectMod[] Function GetAllMods() Native
/// ```
fn get_all_mods(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetConnectPoints`
///
/// ```text
/// ConnectPoint[] Function GetConnectPoints() Native
/// ```
fn get_connect_points(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetConnectedObjects`
///
/// ```text
/// ObjectReference[] Function GetConnectedObjects() Native
/// ```
fn get_connected_objects(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetDisplayName`
///
/// ```text
/// string Function GetDisplayName() Native
/// ```
fn get_display_name(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetInventoryItems`
///
/// ```text
/// Form[] Function GetInventoryItems() Native
/// ```
fn get_inventory_items(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetInventoryWeight`
///
/// ```text
/// float Function GetInventoryWeight() Native
/// ```
fn get_inventory_weight(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetMaterialSwap`
///
/// ```text
/// GetMaterialSwap
/// ```
fn get_material_swap(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetMaterialSwap`
///
/// ```text
/// Function SetMaterialSwap(MatSwap mSwap) Native
/// ```
fn set_material_swap(
    caller: Caller<'_, State>,
    self_ptr: Object,
    material_swap: Object,
) -> Result<(), Error> {
    todo!()
}

/// `TransmitConnectedPower`
///
/// ```text
/// bool Function TransmitConnectedPower() Native
/// ```
fn transmit_connected_power(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}
