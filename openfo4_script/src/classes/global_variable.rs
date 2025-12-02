use crate::classes::{Float, Object, State};
use crate::runtime::Caller;

/// `GetValue`
///
/// ```text
/// float Function GetValue() native
/// ```
pub fn get_value(caller: Caller<'_, State>, self_ptr: Object) -> Float {
    todo!()
}

/// `SetValue`
///
/// ```text
/// Function SetValue(float afNewValue) native
/// ```
pub fn set_value(caller: Caller<'_, State>, self_ptr: Object, value: Float) {
    todo!()
}
