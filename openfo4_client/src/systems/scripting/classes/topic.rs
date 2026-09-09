use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Topic", "Add", add);
}

/// `Add`
///
/// ```text
/// Function Add() native
/// ```
fn add(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}
