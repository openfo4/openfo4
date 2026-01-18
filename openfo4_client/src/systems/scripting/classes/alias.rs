use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Alias", "GetOwningQuest", get_owning_quest);
    engine.register_native_function("Alias", "StartObjectProfiling", start_object_profiling);
    engine.register_native_function("Alias", "StopObjectProfiling", stop_object_profiling);
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function() native
/// ```
fn get_owning_quest(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `StartObjectProfiling`
///
/// ```text
/// Function() native
/// ```
fn start_object_profiling(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `StopObjectProfiling`
///
/// ```text
/// Function() native
/// ```
fn stop_object_profiling(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
