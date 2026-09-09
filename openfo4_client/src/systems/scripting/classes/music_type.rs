use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("MusicType", "Add", add);
    engine.register_if_exists("MusicType", "Remove", remove);
}

/// `Add`
///
/// ```text
/// Function() native
/// ```
fn add(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Remove`
///
/// ```text
/// Function() native
/// ```
fn remove(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}
