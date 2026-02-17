use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("LeveledItem", "AddForm", add_form);
    engine.register_native_function("LeveledItem", "Revert", revert);
}

/// `AddForm`
///
/// ```text
/// Function AddForm(Form apForm, int aiLevel, int aiCount) native
/// ```
fn add_form(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_form: Object,
    ai_level: i32,
    ai_count: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `Revert`
///
/// ```text
/// Function Revert() native
/// ```
fn revert(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
