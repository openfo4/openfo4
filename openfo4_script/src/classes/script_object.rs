use crate::classes::{Float, Int, Object, State};
use crate::runtime::Caller;

/// CallFunction
///
/// ```text
/// Var Function CallFunction(string asFuncName, Var[] aParams) native
/// ```
pub fn call_function(Caller: Caller<'_, State>) {
    todo!()
}

/// `StartTimerGameTime`
///
/// ```text
/// Function StartTimerGameTime(float afInterval, int aiTimerID = 0) native
/// ```
pub fn start_timer_game_time(
    Caller: Caller<'_, State>,
    self_ptr: Object,
    interval: Float,
    timer_id: Int,
) {
}

/// `CancelTimerGameTime`
///
/// ```text
/// Function CancelTimerGameTime(int aiTimerID = 0) native
/// ```
pub fn cancel_timer_game_time(Caller: Caller<'_, State>, self_ptr: Object, timer_id: Int) {
    todo!()
}
