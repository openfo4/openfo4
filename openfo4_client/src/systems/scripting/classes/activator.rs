use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("Activator", "IsRadio", is_radio);
}

/// `IsRadio`
///
/// ```text
/// bool Function IsRadio() native
/// ```
fn is_radio(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}
