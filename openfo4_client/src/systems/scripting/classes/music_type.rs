use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("MusicType", "Add", add);
    engine.register_native_function("MusicType", "Remove", remove);
}

/// `Add`
///
/// ```text
/// Function() native
/// ```
fn add(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Remove`
///
/// ```text
/// Function() native
/// ```
fn remove(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
