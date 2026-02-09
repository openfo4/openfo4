use openfo4_script::runtime::{Caller, Exception};

use super::{Float, Object, State};

/// `GetValue`
///
/// ```text
/// float Function GetValue() native
/// ```
pub fn get_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<Float, Exception> {
    let id = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;
    let value = *caller
        .data()
        .globals()
        .variables
        .get(&id)
        .ok_or(Exception::Error)?;
    Ok(value)
}

/// `SetValue`
///
/// ```text
/// Function SetValue(float afNewValue) native
/// ```
pub fn set_value(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    value: Float,
) -> Result<(), Exception> {
    let id = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;
    caller.data_mut().globals_mut().variables.insert(id, value);
    Ok(())
}
