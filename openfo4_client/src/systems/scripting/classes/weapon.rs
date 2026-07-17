use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Weapon", "Fire", fire);
    engine.register_if_exists("Weapon", "GetAmmo", get_ammo);
}

/// `Fire`
///
/// ```text
/// Function Fire(ObjectReference akSource, Ammo akAmmo) native
/// ```
fn fire(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_source: Object,
    ak_ammo: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetAmmo`
///
/// ```text
/// Ammo Function GetAmmo() native
/// ```
fn get_ammo(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
