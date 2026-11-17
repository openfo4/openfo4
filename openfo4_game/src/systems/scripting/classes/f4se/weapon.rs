use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
fn get_embedded_mod(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetEmbeddedMod`
///
/// ```text
/// Function SetEmbeddedMod(ObjectMod mod) native
/// ```
fn set_embedded_mod(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    r#_mod: Object,
) -> Result<(), Error> {
    todo!()
}
