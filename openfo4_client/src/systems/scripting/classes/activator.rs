use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Activator", "IsRadio", is_radio);
}

/// `IsRadio`
///
/// ```text
/// bool Function IsRadio() native
/// ```
fn is_radio(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}
