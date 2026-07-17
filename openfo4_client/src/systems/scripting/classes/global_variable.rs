use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::EngineExt;

use super::{Float, Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("GlobalVariable", "GetValue", get_value);
    engine.register_if_exists("GlobalVariable", "SetValue", set_value);
}

/// `GetValue`
///
/// ```text
/// float Function GetValue() native
/// ```
async fn get_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<Float, Error> {
    let id = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    let globals = caller.data().globals().await;
    let value = *globals.variables.get(&id).ok_or_else(|| todo!())?;
    Ok(value)
}

/// `SetValue`
///
/// ```text
/// Function SetValue(float afNewValue) native
/// ```
async fn set_value(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    value: Float,
) -> Result<(), Error> {
    let id = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    let mut globals = caller.data_mut().globals_mut().await;
    globals.variables.insert(id, value);
    Ok(())
}
