use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Terminal", "ShowOnPipboy", show_on_pipboy);
}

/// `ShowOnPipboy`
///
/// ```text
/// Function ShowOnPipboy() native
/// ```
fn show_on_pipboy(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
