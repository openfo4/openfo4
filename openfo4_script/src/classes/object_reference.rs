use crate::classes::{Float, State};
use crate::runtime::Caller;

/// `GetValue`
///
/// ```text
/// float Function GetValue(ActorValue akAV) native
/// ```
pub fn get_value(caller: Caller<'_, State>, self_ptr: isize, actor_value: isize) -> Float {
    todo!()
}

/// `SetValue`
///
/// ```text
/// Function SetValue(ActorValue akAV, float afValue) native
/// ```
pub fn set_value(caller: Caller<'_, State>, self_ptr: isize, actor_value: isize, value: Float) {
    todo!()
}

/// `GetBaseValue`
///
/// ```text
/// float Function GetBaseValue(ActorValue akAV) native
/// ```
pub fn get_base_value(caller: Caller<'_, State>, self_ptr: isize, actor_value: isize) -> Float {
    todo!()
}
