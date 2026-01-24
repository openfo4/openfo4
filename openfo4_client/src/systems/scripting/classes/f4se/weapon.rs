use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Weapon", "GetEmbeddedMod", get_embedded_mod);
    engine.register_native_function("Weapon", "SetEmbeddedMod", set_embedded_mod);
}

/// `GetEmbeddedMod`
///
/// ```text
/// ObjectMod Function GetEmbeddedMod() native
/// ```
fn get_embedded_mod(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetEmbeddedMod`
///
/// ```text
/// Function SetEmbeddedMod(ObjectMod mod) native
/// ```
fn set_embedded_mod(
    caller: Caller<'_, State>,
    self_ptr: Object,
    r#mod: Object,
) -> Result<(), Exception> {
    todo!()
}
