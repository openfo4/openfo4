use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ObjectMod", "GetPropertyModifiers", get_property_modifiers);
    engine.register_if_exists("ObjectMod", "GetMaxRank", get_max_rank);
    engine.register_if_exists("ObjectMod", "SetMaxRank", set_max_rank);
    engine.register_if_exists("ObjectMod", "GetPriority", get_priority);
    engine.register_if_exists("ObjectMod", "SetPriority", set_priority);
    engine.register_if_exists("ObjectMod", "GetLooseMod", get_loose_mod);
}

/// `GetPropertyModifiers`
///
/// ```text
/// PropertyModifier[] Function GetPropertyModifiers() native
/// ```
fn get_property_modifiers(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetMaxRank`
///
/// ```text
/// int Function GetMaxRank() native
/// ```
fn get_max_rank(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetMaxRank`
///
/// ```text
/// Function SetMaxRank(int rank) native
/// ```
fn set_max_rank(_caller: Caller<'_, State>, _self_ptr: Object, _rank: i32) -> Result<(), Error> {
    todo!()
}

/// `GetPriority`
///
/// ```text
/// int Function GetPriority() native
/// ```
fn get_priority(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `SetPriority`
///
/// ```text
/// Function SetPriority(int priority) native
/// ```
fn set_priority(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _priority: i32,
) -> Result<(), Error> {
    todo!()
}

/// `GetLooseMod`
///
/// ```text
/// MiscObject Function GetLooseMod() native
/// ```
fn get_loose_mod(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
