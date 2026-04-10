use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("MiscObject", "GetObjectComponentCount", get_object_component_count);
}

/// `GetObjectComponentCount`
///
/// ```text
/// int Function GetObjectComponentCount(Component akComponent) native
/// ```
fn get_object_component_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_component: Object,
) -> Result<i32, Exception> {
    todo!()
}
