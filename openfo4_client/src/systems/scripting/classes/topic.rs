use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("Topic", "Add", add);
}

/// `Add`
///
/// ```text
/// Function Add() native
/// ```
fn add(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
