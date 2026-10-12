use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Terminal", "ShowOnPipboy", show_on_pipboy);
}

/// `ShowOnPipboy`
///
/// ```text
/// Function ShowOnPipboy() native
/// ```
fn show_on_pipboy(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}
