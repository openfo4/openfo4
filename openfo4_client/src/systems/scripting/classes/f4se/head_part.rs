use openfo4_script::engine::Engine;
use openfo4_script::runtime::Caller;

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("HeadPart", "GetExtraParts", get_extra_parts);
    engine.register_if_exists("HeadPart", "GetType", get_type);
    engine.register_if_exists("HeadPart", "GetValidRaces", get_valid_races);
    engine.register_if_exists("HeadPart", "HasExtraPart", has_extra_part);
    engine.register_if_exists("HeadPart", "IsExtraPart", is_extra_part);
    engine.register_if_exists("HeadPart", "SetValidRaces", set_valid_races);
}

/// `GetExtraParts`
///
/// ```text
/// HeadPart[] Function GetExtraParts() native
/// ```
fn get_extra_parts(_caller: Caller<'_, State>, _self_ptr: Object) -> Object {
    todo!()
}

/// `GetType`
///
/// ```text
/// int Function GetType() native
/// ```
fn get_type(_caller: Caller<'_, State>, _self_ptr: Object) -> i32 {
    todo!()
}

/// `GetValidRaces`
///
/// ```text
/// FormList Function GetValidRaces() native
/// ```
fn get_valid_races(_caller: Caller<'_, State>, _self_ptr: Object) -> Object {
    todo!()
}

/// `HasExtraPart`
///
/// ```text
/// bool Function HasExtraPart(HeadPart p) native
/// ```
fn has_extra_part(_caller: Caller<'_, State>, _self_ptr: Object, _part: Object) -> bool {
    todo!()
}

/// `IsExtraPart`
///
/// ```text
/// bool Function IsExtraPart() native
/// ```
fn is_extra_part(_caller: Caller<'_, State>, _self_ptr: Object) -> bool {
    todo!()
}

/// `SetValidRaces`
///
/// ```text
/// Function SetValidRaces(FormList vRaces) native
/// ```
fn set_valid_races(_caller: Caller<'_, State>, _self_ptr: Object, _races: Object) {
    todo!()
}
