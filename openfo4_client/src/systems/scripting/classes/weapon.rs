use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Weapon", "Fire", fire);
    engine.register_native_function("Weapon", "GetAmmo", get_ammo);
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
) -> Result<(), Exception> {
    todo!()
}

/// `GetAmmo`
///
/// ```text
/// Ammo Function GetAmmo() native
/// ```
fn get_ammo(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}
