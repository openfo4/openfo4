use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("MiscObject", "GetObjectComponentCount", get_object_component_count);
}

/// `GetObjectComponentCount`
///
/// ```text
/// int Function GetObjectComponentCount(Component akComponent) native
/// ```
fn get_object_component_count(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_component: Object,
) -> Result<i32, Error> {
    todo!()
}
