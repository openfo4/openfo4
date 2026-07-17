use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error, ObjectAddr};

use crate::systems::scripting::classes::{EngineExt, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Input", "GetMappedKey", get_mapped_key);
    engine.register_if_exists("Input", "GetMappedControl", get_mapped_control);
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
) -> Result<i32, Error> {
    todo!()
}

/// `GetMappedControl`
///
/// ```text
/// string Function GetMappedControl(int keycode) native global
/// ```
fn get_mapped_control(caller: Caller<'_, State>, keycode: i32) -> Result<ObjectAddr, Error> {
    todo!()
}
