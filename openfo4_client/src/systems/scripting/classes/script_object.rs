use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("ScriptObject", "AddInventoryEventFilter", add_inventory_event_filter);
    engine.register_native_function("ScriptObject", "CallFunction", call_function);
    engine.register_native_function("ScriptObject", "CallFunctionNoWait", call_function_no_wait);
    engine.register_native_function("ScriptObject", "CancelTimer", cancel_timer);
    engine.register_native_function("ScriptObject", "CancelTimerGameTime", cancel_timer_game_time);
    engine.register_native_function("ScriptObject", "GetPropertyValue", get_property_value);
    engine.register_native_function("ScriptObject", "CastAs", cast_as);
    engine.register_native_function("ScriptObject", "IsBoundGameObjectAvailable", is_bound_game_object_available);
    engine.register_native_function("ScriptObject", "RegisterForAnimationEvent", register_for_animation_event);
    engine.register_native_function("ScriptObject", "RegisterForCustomEvent", register_for_custom_event);
    engine.register_native_function("ScriptObject", "RegisterForDetectionLOSGain", register_for_detection_los_gain);
    engine.register_native_function("ScriptObject", "RegisterForDetectionLOSLost", register_for_detection_los_lost);
    engine.register_native_function("ScriptObject", "RegisterForDirectLOSGain", register_for_direct_los_gain);
    engine.register_native_function("ScriptObject", "RegisterForDirectLOSLost", register_for_direct_los_lost);
    engine.register_native_function("ScriptObject", "RegisterForDistanceLessThanEvent", register_for_distance_less_than_event);
    engine.register_native_function("ScriptObject", "RegisterForDistanceGreaterThanEvent", register_for_distance_greater_than_event);
    engine.register_native_function("ScriptObject", "RegisterForHitEvent", register_for_hit_event);
    engine.register_native_function("ScriptObject", "RegisterForMagicEffectApplyEvent", register_for_magic_effect_apply_event);
    engine.register_native_function("ScriptObject", "RegisterForMenuOpenCloseEvent", register_for_menu_open_close_event);
    engine.register_native_function("ScriptObject", "RegisterForPlayerSleep", register_for_player_sleep);
    engine.register_native_function("ScriptObject", "RegisterForPlayerTeleport", register_for_player_teleport);
    engine.register_native_function("ScriptObject", "RegisterForPlayerWait", register_for_player_wait);
    engine.register_native_function("ScriptObject", "RegisterForRadiationDamageEvent", register_for_radiation_damage_event);
    engine.register_native_function("ScriptObject", "RegisterForRemoteEvent", register_for_remote_event);
    engine.register_native_function("ScriptObject", "RegisterForTrackedStatsEvent", register_for_tracked_stats_event);
    engine.register_native_function("ScriptObject", "RegisterForLooksMenuEvent", register_for_looks_menu_event);
    engine.register_native_function("ScriptObject", "RegisterForTutorialEvent", register_for_tutorial_event);
    engine.register_native_function("ScriptObject", "RemoveAllInventoryEventFilters", remove_all_inventory_event_filters);
    engine.register_native_function("ScriptObject", "RemoveInventoryEventFilter", remove_inventory_event_filter);
    engine.register_native_function("ScriptObject", "SendCustomEvent", send_custom_event);
    engine.register_native_function("ScriptObject", "SetPropertyValue", set_property_value);
    engine.register_native_function("ScriptObject", "SetPropertyValueNoWait", set_property_value_no_wait);
    engine.register_native_function("ScriptObject", "StartTimer", start_timer);
    engine.register_native_function("ScriptObject", "StartTimerGameTime", start_timer_game_time);
    engine.register_native_function("ScriptObject", "UnregisterForAllEvents", unregister_for_all_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllCustomEvents", unregister_for_all_custom_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllHitEvents", unregister_for_all_hit_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllMagicEffectApplyEvents", unregister_for_all_magic_effect_apply_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllMenuOpenCloseEvents", unregister_for_all_menu_open_close_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllRadiationDamageEvents", unregister_for_all_radiation_damage_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllRemoteEvents", unregister_for_all_remote_events);
    engine.register_native_function("ScriptObject", "UnregisterForAllTrackedStatsEvents", unregister_for_all_tracked_stats_events);
    engine.register_native_function("ScriptObject", "UnregisterForAnimationEvent", unregister_for_animation_event);
    engine.register_native_function("ScriptObject", "UnregisterForCustomEvent", unregister_for_custom_event);
    engine.register_native_function("ScriptObject", "UnregisterForDistanceEvents", unregister_for_distance_events);
    engine.register_native_function("ScriptObject", "UnregisterForLOS", unregister_for_los);
    engine.register_native_function("ScriptObject", "UnregisterForMagicEffectApplyEvent", unregister_for_magic_effect_apply_event);
    engine.register_native_function("ScriptObject", "UnregisterForMenuOpenCloseEvent", unregister_for_menu_open_close_event);
    engine.register_native_function("ScriptObject", "UnregisterForPlayerSleep", unregister_for_player_sleep);
    engine.register_native_function("ScriptObject", "UnregisterForPlayerTeleport", unregister_for_player_teleport);
    engine.register_native_function("ScriptObject", "UnregisterForPlayerWait", unregister_for_player_wait);
    engine.register_native_function("ScriptObject", "UnregisterForRadiationDamageEvent", unregister_for_radiation_damage_event);
    engine.register_native_function("ScriptObject", "UnregisterForRemoteEvent", unregister_for_remote_event);
    engine.register_native_function("ScriptObject", "UnregisterForTrackedStatsEvent", unregister_for_tracked_stats_event);
    engine.register_native_function("ScriptObject", "UnregisterForLooksMenuEvent", unregister_for_looks_menu_event);
    engine.register_native_function("ScriptObject", "UnregisterForTutorialEvent", unregister_for_tutorial_event);
}

/// `AddInventoryEventFilter`
///
/// ```text
/// Function(Form akFilter) native
/// ```
fn add_inventory_event_filter(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_filter: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `CallFunction`
///
/// ```text
/// Var Function(string asFuncName, Var[] aParams) native
/// ```
fn call_function(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_func_name: isize,
    a_params: isize,
) -> Result<Object, Exception> {
    todo!()
}

/// `CallFunctionNoWait`
///
/// ```text
/// Function(string asFuncName, Var[] aParams) native
/// ```
fn call_function_no_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_func_name: isize,
    a_params: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `CancelTimer`
///
/// ```text
/// Function(int aiTimerID) native
/// ```
fn cancel_timer(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_timer_id: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `CancelTimerGameTime`
///
/// ```text
/// Function(int aiTimerID) native
/// ```
fn cancel_timer_game_time(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_timer_id: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `GetPropertyValue`
///
/// ```text
/// Var Function(string asPropertyName) native
/// ```
fn get_property_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_property_name: isize,
) -> Result<Object, Exception> {
    todo!()
}

/// `CastAs`
///
/// ```text
/// ScriptObject Function(string asScriptName) native
/// ```
fn cast_as(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_script_name: isize,
) -> Result<Object, Exception> {
    todo!()
}

/// `IsBoundGameObjectAvailable`
///
/// ```text
/// bool Function() native
/// ```
fn is_bound_game_object_available(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `RegisterForAnimationEvent`
///
/// ```text
/// bool Function(ObjectReference akSender, string asEventName) native
/// ```
fn register_for_animation_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_sender: Object,
    as_event_name: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `RegisterForCustomEvent`
///
/// ```text
/// Function(ScriptObject akSender, CustomEventName asEventName) native
/// ```
fn register_for_custom_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_sender: Object,
    as_event_name: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDetectionLOSGain`
///
/// ```text
/// Function(Actor akViewer, ObjectReference akTarget) native
/// ```
fn register_for_detection_los_gain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_viewer: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDetectionLOSLost`
///
/// ```text
/// Function(Actor akViewer, ObjectReference akTarget) native
/// ```
fn register_for_detection_los_lost(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_viewer: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDirectLOSGain`
///
/// ```text
/// Function(ObjectReference akViewer, ObjectReference akTarget, string asViewerNode, string asTargetNode) native
/// ```
fn register_for_direct_los_gain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_viewer: Object,
    ak_target: Object,
    as_viewer_node: isize,
    as_target_node: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDirectLOSLost`
///
/// ```text
/// Function(ObjectReference akViewer, ObjectReference akTarget, string asViewerNode, string asTargetNode) native
/// ```
fn register_for_direct_los_lost(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_viewer: Object,
    ak_target: Object,
    as_viewer_node: isize,
    as_target_node: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDistanceLessThanEvent`
///
/// ```text
/// Function(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native
/// ```
fn register_for_distance_less_than_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_obj1: Object,
    ak_obj2: Object,
    af_distance: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForDistanceGreaterThanEvent`
///
/// ```text
/// Function(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native
/// ```
fn register_for_distance_greater_than_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_obj1: Object,
    ak_obj2: Object,
    af_distance: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForHitEvent`
///
/// ```text
/// Function RegisterForHitEvent(ScriptObject akTarget, ScriptObject akAggressorFilter, Form akSourceFilter, Form akProjectileFilter, int aiPowerFilter, int aiSneakFilter, int aiBashFilter, int aiBlockFilter, bool abMatch) native
/// ```
fn register_for_hit_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
    agressor_filter: Object,
    source_filter: Object,
    projectile_filter: Object,
    power_filter: i32,
    sneak_filter: i32,
    bash_filter: i32,
    bock_filter: i32,
    ab_match: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForMagicEffectApplyEvent`
///
/// ```text
/// Function(ScriptObject akTarget, ScriptObject akCasterFilter, Form akEffectFilter, bool abMatch) native
/// ```
fn register_for_magic_effect_apply_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    ak_caster_filter: Object,
    ak_effect_filter: Object,
    ab_match: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForMenuOpenCloseEvent`
///
/// ```text
/// Function(string asMenuName) native
/// ```
fn register_for_menu_open_close_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_menu_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForPlayerSleep`
///
/// ```text
/// Function() native
/// ```
fn register_for_player_sleep(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForPlayerTeleport`
///
/// ```text
/// Function() native
/// ```
fn register_for_player_teleport(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForPlayerWait`
///
/// ```text
/// Function() native
/// ```
fn register_for_player_wait(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForRadiationDamageEvent`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn register_for_radiation_damage_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForRemoteEvent`
///
/// ```text
/// bool Function(ScriptObject akEventSource, ScriptEventName asEventName) native
/// ```
fn register_for_remote_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_event_source: Object,
    as_event_name: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `RegisterForTrackedStatsEvent`
///
/// ```text
/// Function(string asStat, int aiThreshold) native
/// ```
fn register_for_tracked_stats_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_stat: isize,
    ai_threshold: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForLooksMenuEvent`
///
/// ```text
/// Function() native
/// ```
fn register_for_looks_menu_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForTutorialEvent`
///
/// ```text
/// Function(String asEventName) native
/// ```
fn register_for_tutorial_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_event_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveAllInventoryEventFilters`
///
/// ```text
/// Function() native
/// ```
fn remove_all_inventory_event_filters(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveInventoryEventFilter`
///
/// ```text
/// Function(Form akFilter) native
/// ```
fn remove_inventory_event_filter(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_filter: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SendCustomEvent`
///
/// ```text
/// Function(CustomEventName asEvent, Var[] akArgs) native
/// ```
fn send_custom_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_event: Object,
    ak_args: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPropertyValue`
///
/// ```text
/// Function(string asPropertyName, Var aValue) native
/// ```
fn set_property_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_property_name: isize,
    a_value: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPropertyValueNoWait`
///
/// ```text
/// Function(string asPropertyName, Var aValue) native
/// ```
fn set_property_value_no_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_property_name: isize,
    a_value: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `StartTimer`
///
/// ```text
/// Function(float afInterval, int aiTimerID) native
/// ```
fn start_timer(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_interval: f32,
    ai_timer_id: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `StartTimerGameTime`
///
/// ```text
/// Function(float afInterval, int aiTimerID) native
/// ```
fn start_timer_game_time(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_interval: f32,
    ai_timer_id: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_events(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllCustomEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_custom_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllHitEvents`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn unregister_for_all_hit_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllMagicEffectApplyEvents`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn unregister_for_all_magic_effect_apply_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllMenuOpenCloseEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_menu_open_close_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllRadiationDamageEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_radiation_damage_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllRemoteEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_remote_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAllTrackedStatsEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_tracked_stats_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForAnimationEvent`
///
/// ```text
/// Function(ObjectReference akSender, string asEventName) native
/// ```
fn unregister_for_animation_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_sender: Object,
    as_event_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForCustomEvent`
///
/// ```text
/// Function(ScriptObject akSender, CustomEventName asEventName) native
/// ```
fn unregister_for_custom_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_sender: Object,
    as_event_name: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForDistanceEvents`
///
/// ```text
/// Function(ScriptObject akObj1, ScriptObject akObj2) native
/// ```
fn unregister_for_distance_events(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_obj1: Object,
    ak_obj2: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForLOS`
///
/// ```text
/// Function(ObjectReference akViewer, ObjectReference akTarget) native
/// ```
fn unregister_for_los(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_viewer: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForMagicEffectApplyEvent`
///
/// ```text
/// Function(ScriptObject akTarget, ScriptObject akCasterFilter, Form akEffectFilter, bool abMatch) native
/// ```
fn unregister_for_magic_effect_apply_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    ak_caster_filter: Object,
    ak_effect_filter: Object,
    ab_match: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForMenuOpenCloseEvent`
///
/// ```text
/// Function(string asMenuName) native
/// ```
fn unregister_for_menu_open_close_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_menu_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForPlayerSleep`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_player_sleep(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForPlayerTeleport`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_player_teleport(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForPlayerWait`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_player_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForRadiationDamageEvent`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn unregister_for_radiation_damage_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForRemoteEvent`
///
/// ```text
/// Function(ScriptObject akEventSource, ScriptEventName asEventName) native
/// ```
fn unregister_for_remote_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_event_source: Object,
    as_event_name: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForTrackedStatsEvent`
///
/// ```text
/// Function(string asStat) native
/// ```
fn unregister_for_tracked_stats_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_stat: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForLooksMenuEvent`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_looks_menu_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForTutorialEvent`
///
/// ```text
/// Function(String asEventName) native
/// ```
fn unregister_for_tutorial_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_event_name: isize,
) -> Result<(), Exception> {
    todo!()
}
