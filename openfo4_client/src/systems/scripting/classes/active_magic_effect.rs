use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("ActiveMagicEffect", "Dispel", dispel);
    engine.register_native_function("ActiveMagicEffect", "GetBaseObject", get_base_object);
    engine.register_native_function("ActiveMagicEffect", "GetCasterActor", get_caster_actor);
    engine.register_native_function("ActiveMagicEffect", "GetTargetActor", get_target_actor);
    engine.register_native_function("ActiveMagicEffect", "StartObjectProfiling", start_object_profiling);
    engine.register_native_function("ActiveMagicEffect", "StopObjectProfiling", stop_object_profiling);
}

/// `Dispel`
///
/// ```text
/// Function Dispel() native
/// ```
fn dispel(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `GetBaseObject`
///
/// ```text
/// MagicEffect Function GetBaseObject() native
/// ```
fn get_base_object(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCasterActor`
///
/// ```text
/// Actor Function GetCasterActor() native
/// ```
fn get_caster_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetTargetActor`
///
/// ```text
/// Actor Function GetTargetActor() native
/// ```
fn get_target_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `StartObjectProfiling`
///
/// ```text
/// Function StartObjectProfiling() native
/// ```
fn start_object_profiling(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `StopObjectProfiling`
///
/// ```text
/// Function StopObjectProfiling() native
/// ```
fn stop_object_profiling(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
