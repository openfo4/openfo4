use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("EncounterZone", "GetLocation", get_location);
    engine.register_native_function("EncounterZone", "SetLocation", set_location);
    engine.register_native_function("EncounterZone", "GetRank", get_rank);
    engine.register_native_function("EncounterZone", "SetRank", set_rank);
    engine.register_native_function("EncounterZone", "GetMinLevel", get_min_level);
    engine.register_native_function("EncounterZone", "SetMinLevel", set_min_level);
    engine.register_native_function("EncounterZone", "GetMaxLevel", get_max_level);
    engine.register_native_function("EncounterZone", "SetMaxLevel", set_max_level);
    engine.register_native_function("EncounterZone", "IsNeverResetable", is_never_resetable);
    engine.register_native_function("EncounterZone", "SetNeverResetable", set_never_resetable);
    engine.register_native_function("EncounterZone", "IsWorkshop", is_workshop);
    engine.register_native_function("EncounterZone", "SetWorkshop", set_workshop);
}

/// `GetLocation`
///
/// ```text
/// Location Function GetLocation() native
/// ```
fn get_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetLocation`
///
/// ```text
/// Function SetLocation(Location akLoc) native
/// ```
fn set_location(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_loc: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetRank`
///
/// ```text
/// int Function GetRank() native
/// ```
fn get_rank(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetRank`
///
/// ```text
/// Function SetRank(int rank) native
/// ```
fn set_rank(caller: Caller<'_, State>, self_ptr: Object, rank: i32) -> Result<(), Exception> {
    todo!()
}

/// `GetMinLevel`
///
/// ```text
/// int Function GetMinLevel() native
/// ```
fn get_min_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetMinLevel`
///
/// ```text
/// Function SetMinLevel(int level) native
/// ```
fn set_min_level(caller: Caller<'_, State>, self_ptr: Object, level: i32) -> Result<(), Exception> {
    todo!()
}

/// `GetMaxLevel`
///
/// ```text
/// int Function GetMaxLevel() native
/// ```
fn get_max_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetMaxLevel`
///
/// ```text
/// Function SetMaxLevel(int level) native
/// ```
fn set_max_level(caller: Caller<'_, State>, self_ptr: Object, level: i32) -> Result<(), Exception> {
    todo!()
}

/// `IsNeverResetable`
///
/// ```text
/// bool Function IsNeverResetable() native
/// ```
fn is_never_resetable(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `SetNeverResetable`
///
/// ```text
/// Function SetNeverResetable(bool resetable) native
/// ```
fn set_never_resetable(
    caller: Caller<'_, State>,
    self_ptr: Object,
    resetable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `IsWorkshop`
///
/// ```text
/// bool Function IsWorkshop() native
/// ```
fn is_workshop(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `SetWorkshop`
///
/// ```text
/// Function SetWorkshop(bool ws) native
/// ```
fn set_workshop(caller: Caller<'_, State>, self_ptr: Object, ws: bool) -> Result<(), Exception> {
    todo!()
}
