use std::time::Duration;

use crate::classes::{Bool, Float, Int, Object, State, Timer};
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
    caller: Caller<'_, State>,
    object: Object,
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

/// `RegisterForAnimationEvent`
///
/// ```text
/// bool Function RegisterForAnimationEvent(ObjectReference akSender, string asEventName) native
/// ```
pub fn register_for_animation_event(
    caller: Caller<'_, State>,
    object: Object,
    sender: Object,
    event_name: isize,
) -> Bool {
    todo!()
}

/// `UnregisterForLOS`
///
/// ```text
/// Function UnregisterForLOS(ObjectReference akViewer, ObjectReference akTarget) native
/// ```
pub fn unregister_for_los(
    caller: Caller<'_, State>,
    object: Object,
    viewer: Object,
    target: Object,
) {
    todo!()
}

/// `UnregisterForRemoteEvent`
///
/// ```text
/// Function UnregisterForRemoteEvent(ScriptObject akEventSource, string asEventName) native
/// ```
pub fn unregister_for_remote_event(
    caller: Caller<'_, State>,
    object: Object,
    event_source: Object,
    script_event_name: isize,
) {
    todo!()
}
