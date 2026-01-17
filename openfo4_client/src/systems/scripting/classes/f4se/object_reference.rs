use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

/// `ApplyMaterialSwap`
///
/// ```text
/// MatSwap:RemapData[] Function ApplyMaterialSwap(MatSwap mSwap, bool renameMaterial = false) Native
/// ```
pub fn apply_material_swap(
    caller: Caller<'_, State>,
    object: Object,
    material_swap: Object,
    rename_material: bool,
) -> Result<isize, Exception> {
    todo!()
}

/// `AttachWire`
///
/// ```text
/// ObjectReference Function AttachWire(ObjectReference akRef, Form akSpline = None) Native
/// ```
pub fn attach_wire(
    caller: Caller<'_, State>,
    object: Object,
    spline: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetAllMods`
///
/// ```text
/// ObjectMod[] Function GetAllMods() Native
/// ```
pub fn get_all_mods(caller: Caller<'_, State>, object: Object) -> Result<isize, Exception> {
    todo!()
}

/// `GetConnectPoints`
///
/// ```text
/// ConnectPoint[] Function GetConnectPoints() Native
/// ```
pub fn get_connect_points(caller: Caller<'_, State>, object: Object) -> Result<isize, Exception> {
    todo!()
}

/// `GetConnectedObjects`
///
/// ```text
/// ObjectReference[] Function GetConnectedObjects() Native
/// ```
pub fn get_connected_objects(
    caller: Caller<'_, State>,
    object: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetDisplayName`
///
/// ```text
/// string Function GetDisplayName() Native
/// ```
pub fn get_display_name(caller: Caller<'_, State>, object: Object) -> Result<isize, Exception> {
    todo!()
}

/// `GetInventoryItems`
///
/// ```text
/// Form[] Function GetInventoryItems() Native
/// ```
pub fn get_inventory_items(caller: Caller<'_, State>, object: Object) -> Result<isize, Exception> {
    todo!()
}

/// `GetInventoryWeight`
///
/// ```text
/// float Function GetInventoryWeight() Native
/// ```
pub fn get_inventory_weight(caller: Caller<'_, State>, object: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetMaterialSwap`
///
/// ```text
/// GetMaterialSwap
/// ```
pub fn get_material_swap(caller: Caller<'_, State>, object: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetMaterialSwap`
///
/// ```text
/// Function SetMaterialSwap(MatSwap mSwap) Native
/// ```
pub fn set_material_swap(
    caller: Caller<'_, State>,
    object: Object,
    material_swap: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `TransmitConnectedPower`
///
/// ```text
/// bool Function TransmitConnectedPower() Native
/// ```
pub fn transmit_connected_power(
    caller: Caller<'_, State>,
    object: Object,
) -> Result<bool, Exception> {
    todo!()
}
