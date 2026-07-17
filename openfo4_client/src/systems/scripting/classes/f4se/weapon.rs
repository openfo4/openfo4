use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Weapon", "GetEmbeddedMod", get_embedded_mod);
    engine.register_if_exists("Weapon", "SetEmbeddedMod", set_embedded_mod);
}

/// `GetEmbeddedMod`
///
/// ```text
/// ObjectMod Function GetEmbeddedMod() native
/// ```
fn get_embedded_mod(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
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
) -> Result<(), Error> {
    todo!()
}
