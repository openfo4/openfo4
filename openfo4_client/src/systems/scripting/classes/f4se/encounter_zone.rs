use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("EncounterZone", "GetLocation", get_location);
    engine.register_if_exists("EncounterZone", "SetLocation", set_location);
    engine.register_if_exists("EncounterZone", "GetRank", get_rank);
    engine.register_if_exists("EncounterZone", "SetRank", set_rank);
    engine.register_if_exists("EncounterZone", "GetMinLevel", get_min_level);
    engine.register_if_exists("EncounterZone", "SetMinLevel", set_min_level);
    engine.register_if_exists("EncounterZone", "GetMaxLevel", get_max_level);
    engine.register_if_exists("EncounterZone", "SetMaxLevel", set_max_level);
    engine.register_if_exists("EncounterZone", "IsNeverResetable", is_never_resetable);
    engine.register_if_exists("EncounterZone", "SetNeverResetable", set_never_resetable);
    engine.register_if_exists("EncounterZone", "IsWorkshop", is_workshop);
    engine.register_if_exists("EncounterZone", "SetWorkshop", set_workshop);
}

/// `GetLocation`
///
/// ```text
/// Location Function GetLocation() native
/// ```
fn get_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetLocation`
///
/// ```text
/// Function SetLocation(Location akLoc) native
/// ```
fn set_location(caller: Caller<'_, State>, self_ptr: Object, ak_loc: Object) -> Result<(), Error> {
    todo!()
}

/// `GetRank`
///
/// ```text
/// int Function GetRank() native
/// ```
fn get_rank(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetRank`
///
/// ```text
/// Function SetRank(int rank) native
/// ```
fn set_rank(caller: Caller<'_, State>, self_ptr: Object, rank: i32) -> Result<(), Error> {
    todo!()
}

/// `GetMinLevel`
///
/// ```text
/// int Function GetMinLevel() native
/// ```
fn get_min_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetMinLevel`
///
/// ```text
/// Function SetMinLevel(int level) native
/// ```
fn set_min_level(caller: Caller<'_, State>, self_ptr: Object, level: i32) -> Result<(), Error> {
    todo!()
}

/// `GetMaxLevel`
///
/// ```text
/// int Function GetMaxLevel() native
/// ```
fn get_max_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetMaxLevel`
///
/// ```text
/// Function SetMaxLevel(int level) native
/// ```
fn set_max_level(caller: Caller<'_, State>, self_ptr: Object, level: i32) -> Result<(), Error> {
    todo!()
}

/// `IsNeverResetable`
///
/// ```text
/// bool Function IsNeverResetable() native
/// ```
fn is_never_resetable(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `IsWorkshop`
///
/// ```text
/// bool Function IsWorkshop() native
/// ```
fn is_workshop(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `SetWorkshop`
///
/// ```text
/// Function SetWorkshop(bool ws) native
/// ```
fn set_workshop(caller: Caller<'_, State>, self_ptr: Object, ws: bool) -> Result<(), Error> {
    todo!()
}
