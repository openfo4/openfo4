use openfo4_script::runtime::{Caller, Exception};

use super::{Float, Object, State};

/// `GetValue`
///
/// ```text
/// float Function GetValue() native
/// ```
pub fn get_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<Float, Exception> {
    let value = caller
        .data()
        .provider
        .get_global(self_ptr)
        .unwrap_or_default();

    Ok(value)
}

/// `SetValue`
///
/// ```text
/// Function SetValue(float afNewValue) native
/// ```
pub fn set_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    value: Float,
) -> Result<(), Exception> {
    caller.data().provider.set_global(self_ptr, value);
    Ok(())
}
