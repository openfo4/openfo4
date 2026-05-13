use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception, ObjectAddr},
};

use crate::systems::scripting::classes::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Input", "GetMappedKey", get_mapped_key);
    engine.register_native_function("Input", "GetMappedControl", get_mapped_control);
}

/// `GetMappedKey`
///
/// ```text
/// int Function GetMappedKey(string control, int deviceType) native global
/// ```
fn get_mapped_key(
    caller: Caller<'_, State>,
    control: ObjectAddr,
    device_type: i32,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetMappedControl`
///
/// ```text
/// string Function GetMappedControl(int keycode) native global
/// ```
fn get_mapped_control(caller: Caller<'_, State>, keycode: i32) -> Result<ObjectAddr, Exception> {
    todo!()
}
