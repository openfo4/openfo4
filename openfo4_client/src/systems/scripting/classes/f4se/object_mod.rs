use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

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
fn get_property_modifiers(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetMaxRank`
///
/// ```text
/// int Function GetMaxRank() native
/// ```
fn get_max_rank(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetMaxRank`
///
/// ```text
/// Function SetMaxRank(int rank) native
/// ```
fn set_max_rank(caller: Caller<'_, State>, self_ptr: Object, rank: i32) -> Result<(), Exception> {
    todo!()
}

/// `GetPriority`
///
/// ```text
/// int Function GetPriority() native
/// ```
fn get_priority(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `SetPriority`
///
/// ```text
/// Function SetPriority(int priority) native
/// ```
fn set_priority(
    caller: Caller<'_, State>,
    self_ptr: Object,
    priority: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `GetLooseMod`
///
/// ```text
/// MiscObject Function GetLooseMod() native
/// ```
fn get_loose_mod(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}
