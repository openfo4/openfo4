use std::time::Duration;

use crate::classes::{Bool, Float, Int, Object, State, Timer};
use crate::runtime::{Caller, Exception};

/// CallFunction
///
/// ```text
/// Var Function CallFunction(string asFuncName, Var[] aParams) native
/// ```
pub fn call_function(Caller: Caller<'_, State>) {
    todo!()
}

/// `StartTimer`
///
/// ```text
/// Function StartTimer(float afInterval, int aiTimerID = 0) native
/// ```
pub fn start_timer(
    caller: Caller<'_, State>,
    object: Object,
    interval: Float,
    timer_id: Int,
) -> Result<(), Exception> {
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
) -> Result<(), Exception> {
    todo!()
}

/// `CancelTimer`
///
/// ```text
/// Function CancelTimer(int aiTimerID = 0) native
/// ```
pub fn cancel_timer(
    caller: Caller<'_, State>,
    object: Object,
    timer_id: Int,
) -> Result<(), Exception> {
    todo!()
}

/// `CancelTimerGameTime`
///
/// ```text
/// Function CancelTimerGameTime(int aiTimerID = 0) native
/// ```
pub fn cancel_timer_game_time(
    Caller: Caller<'_, State>,
    self_ptr: Object,
    timer_id: Int,
) -> Result<(), Exception> {
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
) -> Result<Bool, Exception> {
    todo!()
}

/// `RegisterForCustomEvent`
///
/// ```text
/// Function RegisterForCustomEvent(ScriptObject akSender, CustomEventName asEventName) native
/// ```
pub fn register_for_custom_event(
    caller: Caller<'_, State>,
    object: Object,
    sender: Object,
    event_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDistanceGreaterThanEvent`
///
/// ```text
/// Function RegisterForDistanceGreaterThanEvent(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native
/// ```
pub fn register_for_distance_greater_than_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    object1: Object,
    object2: Object,
    distance: Float,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDistanceLessThanEvent`

/// ```text
/// Function RegisterForDistanceLessThanEvent(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native
/// ```
pub fn register_for_distance_less_than_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    object1: Object,
    object2: Object,
    distance: Float,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForHitEvent`
///
/// ```text
/// Function RegisterForHitEvent(ScriptObject akTarget, ScriptObject akAggressorFilter = None, Form akSourceFilter = None, Form akProjectileFilter = None, int aiPowerFilter = -1, int aiSneakFilter = -1, int aiBashFilter = -1, int aiBlockFilter = -1, bool abMatch = true) native
/// ```
pub fn register_for_hit_event(
    caller: Caller<'_, State>,
    object: Object,
    target: Object,
    agressor_filter: Object,
    source_filter: Object,
    projectile_filter: Object,
    power_filter: Int,
    sneak_filter: Int,
    bash_filter: Int,
    block_filter: Int,
    whitelist_filter: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForMenuOpenCloseEvent`
///
/// ```text
/// Function RegisterForMenuOpenCloseEvent(string asMenuName) native
/// ```
pub fn register_for_menu_open_close_event(
    caller: Caller<'_, State>,
    object: Object,
    menu_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForPlayerSleep`
///
/// ```text
/// Function RegisterForPlayerSleep() native
/// ```
pub fn register_for_player_sleep(
    caller: Caller<'_, State>,
    object: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForPlayerTeleport`
///
/// ```text
/// Function RegisterForPlayerTeleport() native
/// ```
pub fn register_for_player_teleport(
    caller: Caller<'_, State>,
    object: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForPlayerWait`
///
/// ```text
/// Function RegisterForPlayerWait() native
/// ```
pub fn register_for_player_wait(
    caller: Caller<'_, State>,
    object: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForRemoteEvent`
///
/// ```text
/// bool RegisterForRemoteEvent(ScriptObject akEventSource, ScriptEventName asEventName) native
/// ```
pub fn register_for_remote_event(
    caller: Caller<'_, State>,
    object: Object,
    source: Object,
    event_name: isize,
) -> Result<Bool, Exception> {
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
