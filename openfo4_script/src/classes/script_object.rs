use crate::classes::{Int, Object, State};
use crate::runtime::Caller;

/// CallFunction
///
/// ```text
/// Var Function CallFunction(string asFuncName, Var[] aParams) native
/// ```
pub fn call_function(Caller: Caller<'_, State>) {
    todo!()
}

/// `CancelTimerGameTime`
///
/// ```text
/// Function CancelTimerGameTime(int aiTimerID = 0) native
/// ```
pub fn cancel_timer_game_time(Caller: Caller<'_, State>, self_ptr: Object, timer_id: Int) {
    todo!()
}
