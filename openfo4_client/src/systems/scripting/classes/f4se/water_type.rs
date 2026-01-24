use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("WaterType", "GetConsumeSpell", get_consume_spell);
    engine.register_native_function("WaterType", "SetConsumeSpell", set_consume_spell);
    engine.register_native_function("WaterType", "GetContactSpell", get_contact_spell);
    engine.register_native_function("WaterType", "SetContactSpell", set_contact_spell);
}

/// `GetConsumeSpell`
///
/// ```text
/// Spell Function GetConsumeSpell() native
/// ```
fn get_consume_spell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetConsumeSpell`
///
/// ```text
/// Function SetConsumeSpell(Spell sp) native
/// ```
fn set_consume_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    sp: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetContactSpell`
///
/// ```text
/// Spell Function GetContactSpell() native
/// ```
fn get_contact_spell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetContactSpell`
///
/// ```text
/// Function SetContactSpell(Spell sp) native
/// ```
fn set_contact_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    sp: Object,
) -> Result<(), Exception> {
    todo!()
}
