use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_source: Object,
    _ak_ammo: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetAmmo`
///
/// ```text
/// Ammo Function GetAmmo() native
/// ```
fn get_ammo(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
