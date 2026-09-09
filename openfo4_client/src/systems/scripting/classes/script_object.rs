use std::num::NonZeroU32;
use std::time::Duration;

use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::registration::{Event, EventKind, EventSender};
use crate::systems::scripting::state::{State, TimerId};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ScriptObject", "AddInventoryEventFilter", add_inventory_event_filter);
    engine.register_if_exists("ScriptObject", "CallFunction", call_function);
    engine.register_if_exists("ScriptObject", "CallFunctionNoWait", call_function_no_wait);
    engine.register_if_exists("ScriptObject", "CancelTimer", cancel_timer);
    engine.register_if_exists("ScriptObject", "CancelTimerGameTime", cancel_timer_game_time);
    engine.register_if_exists("ScriptObject", "GetPropertyValue", get_property_value);
    engine.register_if_exists("ScriptObject", "CastAs", cast_as);
    engine.register_if_exists("ScriptObject", "IsBoundGameObjectAvailable", is_bound_game_object_available);
    engine.register_if_exists("ScriptObject", "RegisterForAnimationEvent", register_for_animation_event);
    engine.register_if_exists("ScriptObject", "RegisterForCustomEvent", register_for_custom_event);
    engine.register_if_exists("ScriptObject", "RegisterForDetectionLOSGain", register_for_detection_los_gain);
    engine.register_if_exists("ScriptObject", "RegisterForDetectionLOSLost", register_for_detection_los_lost);
    engine.register_if_exists("ScriptObject", "RegisterForDirectLOSGain", register_for_direct_los_gain);
    engine.register_if_exists("ScriptObject", "RegisterForDirectLOSLost", register_for_direct_los_lost);
    engine.register_if_exists("ScriptObject", "RegisterForDistanceLessThanEvent", register_for_distance_less_than_event);
    engine.register_if_exists("ScriptObject", "RegisterForDistanceGreaterThanEvent", register_for_distance_greater_than_event);
    engine.register_if_exists("ScriptObject", "RegisterForHitEvent", register_for_hit_event);
    engine.register_if_exists("ScriptObject", "RegisterForMagicEffectApplyEvent", register_for_magic_effect_apply_event);
    engine.register_if_exists("ScriptObject", "RegisterForMenuOpenCloseEvent", register_for_menu_open_close_event);
    engine.register_if_exists("ScriptObject", "RegisterForPlayerSleep", register_for_player_sleep);
    engine.register_if_exists("ScriptObject", "RegisterForPlayerTeleport", register_for_player_teleport);
    engine.register_if_exists("ScriptObject", "RegisterForPlayerWait", register_for_player_wait);
    engine.register_if_exists("ScriptObject", "RegisterForRadiationDamageEvent", register_for_radiation_damage_event);
    engine.register_if_exists("ScriptObject", "RegisterForRemoteEvent", register_for_remote_event);
    engine.register_if_exists("ScriptObject", "RegisterForTrackedStatsEvent", register_for_tracked_stats_event);
    engine.register_if_exists("ScriptObject", "RegisterForLooksMenuEvent", register_for_looks_menu_event);
    engine.register_if_exists("ScriptObject", "RegisterForTutorialEvent", register_for_tutorial_event);
    engine.register_if_exists("ScriptObject", "RemoveAllInventoryEventFilters", remove_all_inventory_event_filters);
    engine.register_if_exists("ScriptObject", "RemoveInventoryEventFilter", remove_inventory_event_filter);
    engine.register_if_exists("ScriptObject", "SendCustomEvent", send_custom_event);
    engine.register_if_exists("ScriptObject", "SetPropertyValue", set_property_value);
    engine.register_if_exists("ScriptObject", "SetPropertyValueNoWait", set_property_value_no_wait);
    engine.register_if_exists("ScriptObject", "StartTimer", start_timer);
    engine.register_if_exists("ScriptObject", "StartTimerGameTime", start_timer_game_time);
    engine.register_if_exists("ScriptObject", "UnregisterForAllEvents", unregister_for_all_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllCustomEvents", unregister_for_all_custom_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllHitEvents", unregister_for_all_hit_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllMagicEffectApplyEvents", unregister_for_all_magic_effect_apply_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllMenuOpenCloseEvents", unregister_for_all_menu_open_close_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllRadiationDamageEvents", unregister_for_all_radiation_damage_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllRemoteEvents", unregister_for_all_remote_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAllTrackedStatsEvents", unregister_for_all_tracked_stats_events);
    engine.register_if_exists("ScriptObject", "UnregisterForAnimationEvent", unregister_for_animation_event);
    engine.register_if_exists("ScriptObject", "UnregisterForCustomEvent", unregister_for_custom_event);
    engine.register_if_exists("ScriptObject", "UnregisterForDistanceEvents", unregister_for_distance_events);
    engine.register_if_exists("ScriptObject", "UnregisterForLOS", unregister_for_los);
    engine.register_if_exists("ScriptObject", "UnregisterForMagicEffectApplyEvent", unregister_for_magic_effect_apply_event);
    engine.register_if_exists("ScriptObject", "UnregisterForMenuOpenCloseEvent", unregister_for_menu_open_close_event);
    engine.register_if_exists("ScriptObject", "UnregisterForPlayerSleep", unregister_for_player_sleep);
    engine.register_if_exists("ScriptObject", "UnregisterForPlayerTeleport", unregister_for_player_teleport);
    engine.register_if_exists("ScriptObject", "UnregisterForPlayerWait", unregister_for_player_wait);
    engine.register_if_exists("ScriptObject", "UnregisterForRadiationDamageEvent", unregister_for_radiation_damage_event);
    engine.register_if_exists("ScriptObject", "UnregisterForRemoteEvent", unregister_for_remote_event);
    engine.register_if_exists("ScriptObject", "UnregisterForTrackedStatsEvent", unregister_for_tracked_stats_event);
    engine.register_if_exists("ScriptObject", "UnregisterForLooksMenuEvent", unregister_for_looks_menu_event);
    engine.register_if_exists("ScriptObject", "UnregisterForHitEvent", unregister_for_hit_event);
    engine.register_if_exists("ScriptObject", "UnregisterForTutorialEvent", unregister_for_tutorial_event);
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
) -> Result<(), Error> {
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
    as_func_name: Object,
    a_params: Object,
) -> Result<Object, Error> {
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
    as_func_name: Object,
    a_params: Object,
) -> Result<(), Error> {
    todo!()
}

/// `CancelTimer`
///
/// ```text
/// Function(int aiTimerID) native
/// ```
fn cancel_timer(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    timer_id: i32,
) -> Result<(), Error> {
    let timer_id = TimerId(timer_id);
    caller.data_mut().real_timers.remove(self_ptr, timer_id);
    Ok(())
}

/// `CancelTimerGameTime`
///
/// ```text
/// Function(int aiTimerID) native
/// ```
fn cancel_timer_game_time(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    timer_id: i32,
) -> Result<(), Error> {
    let timer_id = TimerId(timer_id);
    caller.data_mut().game_timers.remove(self_ptr, timer_id);
    Ok(())
}

/// `GetPropertyValue`
///
/// ```text
/// Var Function(string asPropertyName) native
/// ```
fn get_property_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_property_name: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `CastAs`
///
/// ```text
/// ScriptObject Function(string asScriptName) native
/// ```
fn cast_as(caller: Caller<'_, State>, self_ptr: Object, script_name: Object) -> Object {
    let script = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let scripts = caller.data().objects.get_by_id(script.id()).unwrap();

    let name = caller.string(script_name);

    // - CastAs is allowed to return completely different script hierarchy
    //   than the current one (e.g. if an object has the scripts A->B and C->D
    //   calling CastAs(B, "D") will return the C->D object).
    // - If both hierarchies have the same script, which one is returned is not
    //   specified (e.g. calling CastAs(B, "A") with scripts A->B A->C is allowed
    //   to return either A->B or A->C).
    match scripts.get(name) {
        Some(target) => target.get().object.addr(),
        None => Object::NULL,
    }
}

/// `IsBoundGameObjectAvailable`
///
/// ```text
/// bool Function() native
/// ```
fn is_bound_game_object_available(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `RegisterForAnimationEvent`
///
/// ```text
/// bool Function(ObjectReference akSender, string asEventName) native
/// ```
fn register_for_animation_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    sender: Object,
    event_name: Object,
) -> Result<bool, Error> {
    if sender.is_null() {
        return Ok(false);
    }

    let event_name = caller.string(event_name).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::Animation { event_name },
            sender: EventSender::Object(sender),
            limit: None,
        },
    );

    Ok(true)
}

/// `RegisterForCustomEvent`
///
/// ```text
/// Function(ScriptObject akSender, CustomEventName asEventName) native
/// ```
fn register_for_custom_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    sender: Object,
    event_name: Object,
) -> Result<(), Error> {
    if sender.is_null() {
        return Ok(());
    }

    let event_name = caller.string(event_name).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::Custom(event_name),
            sender: EventSender::Object(sender),
            limit: None,
        },
    );

    Ok(())
}

/// `RegisterForDetectionLOSGain`
///
/// ```text
/// Function(Actor akViewer, ObjectReference akTarget) native
/// ```
fn register_for_detection_los_gain(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    viewer: Object,
    target: Object,
) -> Result<(), Error> {
    if viewer.is_null() || target.is_null() {
        return Ok(());
    }

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::DetectionLosGain { viewer, target },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForDetectionLOSLost`
///
/// ```text
/// Function(Actor akViewer, ObjectReference akTarget) native
/// ```
fn register_for_detection_los_lost(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    viewer: Object,
    target: Object,
) -> Result<(), Error> {
    if viewer.is_null() || target.is_null() {
        return Ok(());
    }

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::DetectionLosLost { viewer, target },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForDirectLOSGain`
///
/// ```text
/// Function(ObjectReference akViewer, ObjectReference akTarget, string asViewerNode, string asTargetNode) native
/// ```
fn register_for_direct_los_gain(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    viewer: Object,
    target: Object,
    viewer_node: Object,
    target_node: Object,
) -> Result<(), Error> {
    if viewer.is_null() || target.is_null() {
        return Ok(());
    }

    let viewer_node = caller.string(viewer_node).to_owned();
    let target_node = caller.string(target_node).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::DirectLosGain {
                viewer,
                target,
                viewer_node,
                target_node,
            },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForDirectLOSLost`
///
/// ```text
/// Function(ObjectReference akViewer, ObjectReference akTarget, string asViewerNode, string asTargetNode) native
/// ```
fn register_for_direct_los_lost(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    viewer: Object,
    target: Object,
    viewer_node: Object,
    target_node: Object,
) -> Result<(), Error> {
    if viewer.is_null() || target.is_null() {
        return Ok(());
    }

    let viewer_node = caller.string(viewer_node).to_owned();
    let target_node = caller.string(target_node).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::DirectLosLost {
                viewer,
                target,
                viewer_node,
                target_node,
            },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForDistanceLessThanEvent`
///
/// ```text
/// Function(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native
/// ```
fn register_for_distance_less_than_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    object1: Object,
    object2: Object,
    distance: f32,
) -> Result<(), Error> {
    if object1.is_null() || object2.is_null() || !distance.is_finite() {
        return Ok(());
    }

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::DistanceLessThan {
                object1,
                object2,
                distance,
            },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForDistanceGreaterThanEvent`
///
/// ```text
/// Function(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native
/// ```
fn register_for_distance_greater_than_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    object1: Object,
    object2: Object,
    distance: f32,
) -> Result<(), Error> {
    if object1.is_null() || object2.is_null() || !distance.is_finite() {
        return Ok(());
    }

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::DistanceGreaterThan {
                object1,
                object2,
                distance,
            },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForHitEvent`
///
/// ```text
/// Function RegisterForHitEvent(ScriptObject akTarget, ScriptObject akAggressorFilter, Form akSourceFilter, Form akProjectileFilter, int aiPowerFilter, int aiSneakFilter, int aiBashFilter, int aiBlockFilter, bool abMatch) native
/// ```
fn register_for_hit_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
    agressor_filter: Object,
    source_filter: Object,
    projectile_filter: Object,
    power_filter: i32,
    sneak_filter: i32,
    bash_filter: i32,
    block_filter: i32,
    must_match: bool,
) -> Result<(), Error> {
    if target.is_null() {
        return Ok(());
    }

    let aggressor_filter = (!agressor_filter.is_null()).then_some(agressor_filter);
    let source_filter = (!source_filter.is_null()).then_some(source_filter);
    let projectile_filter = (!projectile_filter.is_null()).then_some(projectile_filter);

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::Hit {
                aggressor_filter,
                source_filter,
                projectile_filter,
                power_filter,
                sneak_filter,
                bash_filter,
                block_filter,
                filters_must_match: must_match,
            },
            sender: EventSender::Object(target),
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForMagicEffectApplyEvent`
///
/// ```text
/// Function(ScriptObject akTarget, ScriptObject akCasterFilter, Form akEffectFilter, bool abMatch) native
/// ```
fn register_for_magic_effect_apply_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
    caster_filter: Object,
    effect_filter: Object,
    must_match: bool,
) -> Result<(), Error> {
    if target.is_null() {
        return Ok(());
    }

    let caster_filter = (!caster_filter.is_null()).then_some(caster_filter);
    let effect_filter = (!effect_filter.is_null()).then_some(effect_filter);

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::MagicEffectApply {
                caster_filter,
                effect_filter,
                filters_must_match: must_match,
            },
            sender: EventSender::Object(target),
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForMenuOpenCloseEvent`
///
/// ```text
/// Function(string asMenuName) native
/// ```
fn register_for_menu_open_close_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    menu_name: Object,
) -> Result<(), Error> {
    let menu_name = caller.string(menu_name).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::MenuOpenClose(menu_name),
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `RegisterForPlayerSleep`
///
/// ```text
/// Function() native
/// ```
fn register_for_player_sleep(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::PlayerSleep,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `RegisterForPlayerTeleport`
///
/// ```text
/// Function() native
/// ```
fn register_for_player_teleport(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::PlayerTeleport,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `RegisterForPlayerWait`
///
/// ```text
/// Function() native
/// ```
fn register_for_player_wait(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::PlayerWait,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `RegisterForRadiationDamageEvent`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn register_for_radiation_damage_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
) -> Result<(), Error> {
    if target.is_null() {
        return Ok(());
    }

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::RadiationDamage,
            sender: EventSender::Object(target),
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForRemoteEvent`
///
/// ```text
/// bool Function(ScriptObject akEventSource, ScriptEventName asEventName) native
/// ```
fn register_for_remote_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    event_source: Object,
    event_name: Object,
) -> Result<bool, Error> {
    if event_source.is_null() {
        return Ok(false);
    }

    let event_name = caller.string(event_name);
    let Ok(kind) = event_name.parse() else {
        tracing::warn!("unknown event name: {}", event_name);
        return Ok(false);
    };

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            sender: EventSender::Object(event_source),
            kind,
            limit: None,
        },
    );

    Ok(true)
}

/// `RegisterForTrackedStatsEvent`
///
/// ```text
/// Function(string asStat, int aiThreshold) native
/// ```
fn register_for_tracked_stats_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    stat: Object,
    threshold: i32,
) -> Result<(), Error> {
    let stat = caller.string(stat).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::TrackedStats { stat, threshold },
            sender: EventSender::Any,
            // Only receive event once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RegisterForLooksMenuEvent`
///
/// ```text
/// Function() native
/// ```
fn register_for_looks_menu_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::LooksMenu,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `RegisterForTutorialEvent`
///
/// ```text
/// Function(String asEventName) native
/// ```
fn register_for_tutorial_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    event_name: Object,
) -> Result<(), Error> {
    let name = caller.string(event_name).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::TutorialEvent { name },
            sender: EventSender::Any,
            // Event is only received once.
            limit: Some(NonZeroU32::MIN),
        },
    );

    Ok(())
}

/// `RemoveAllInventoryEventFilters`
///
/// ```text
/// Function() native
/// ```
fn remove_all_inventory_event_filters(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
    ak_args: Object,
) -> Result<(), Error> {
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
    as_property_name: Object,
    a_value: Object,
) -> Result<(), Error> {
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
    as_property_name: Object,
    a_value: Object,
) -> Result<(), Error> {
    todo!()
}

/// `StartTimer`
///
/// ```text
/// Function(float afInterval, int aiTimerID) native
/// ```
fn start_timer(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    interval: f32,
    timer_id: i32,
) -> Result<(), Error> {
    let duration = Duration::from_secs_f32(interval);
    let timer_id = TimerId(timer_id);

    caller
        .data_mut()
        .real_timers
        .insert(self_ptr, timer_id, duration);
    Ok(())
}

/// `StartTimerGameTime`
///
/// ```text
/// Function(float afInterval, int aiTimerID) native
/// ```
fn start_timer_game_time(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    interval: f32,
    timer_id: i32,
) -> Result<(), Error> {
    // interval is in game-hours.
    let duration = Duration::from_secs_f32(interval * 3600.0);
    let timer_id = TimerId(timer_id);

    caller
        .data_mut()
        .game_timers
        .insert(self_ptr, timer_id, duration);
    Ok(())
}

/// `UnregisterForAllEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_events(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    caller.data_mut().registration.unregister_all(self_ptr);
    Ok(())
}

/// `UnregisterForAllCustomEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_custom_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| matches!(event.kind, EventKind::Custom(_)));
    Ok(())
}

/// `UnregisterForAllHitEvents`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn unregister_for_all_hit_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
) -> Result<(), Error> {
    let all_targets = target.is_null();

    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| {
            if !matches!(
                event.kind,
                EventKind::Hit {
                    aggressor_filter: _,
                    source_filter: _,
                    projectile_filter: _,
                    power_filter: _,
                    sneak_filter: _,
                    bash_filter: _,
                    block_filter: _,
                    filters_must_match: _
                }
            ) {
                return false;
            }

            if all_targets {
                true
            } else {
                event.sender == EventSender::Object(target)
            }
        });

    Ok(())
}

/// `UnregisterForAllMagicEffectApplyEvents`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn unregister_for_all_magic_effect_apply_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
) -> Result<(), Error> {
    let all_targets = target.is_null();

    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| {
            if !matches!(
                event.kind,
                EventKind::MagicEffectApply {
                    caster_filter: _,
                    effect_filter: _,
                    filters_must_match: _
                }
            ) {
                return false;
            }

            if all_targets {
                true
            } else {
                event.sender == EventSender::Object(target)
            }
        });

    Ok(())
}

/// `UnregisterForAllMenuOpenCloseEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_menu_open_close_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| {
            matches!(event.kind, EventKind::MenuOpenClose(_))
        });
    Ok(())
}

/// `UnregisterForAllRadiationDamageEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_radiation_damage_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| {
            matches!(event.kind, EventKind::RadiationDamage)
        });

    Ok(())
}

/// `UnregisterForAllRemoteEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_remote_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| match event.sender {
            EventSender::Any => false,
            EventSender::Object(object) => object != self_ptr,
        });
    Ok(())
}

/// `UnregisterForAllTrackedStatsEvents`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_all_tracked_stats_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| {
            matches!(
                event.kind,
                EventKind::TrackedStats {
                    stat: _,
                    threshold: _
                }
            )
        });
    Ok(())
}

/// `UnregisterForAnimationEvent`
///
/// ```text
/// Function(ObjectReference akSender, string asEventName) native
/// ```
fn unregister_for_animation_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    sender: Object,
    event_name: Object,
) -> Result<(), Error> {
    if sender.is_null() {
        return Ok(());
    }

    let event_name = caller.string(event_name).to_owned();

    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::Animation { event_name },
            sender: EventSender::Object(sender),
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForCustomEvent`
///
/// ```text
/// Function(ScriptObject akSender, CustomEventName asEventName) native
/// ```
fn unregister_for_custom_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    sender: Object,
    event_name: Object,
) -> Result<(), Error> {
    if sender.is_null() {
        return Ok(());
    }

    let event_name = caller.string(event_name).to_owned();

    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::Custom(event_name),
            sender: EventSender::Object(sender),
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForDistanceEvents`
///
/// ```text
/// Function(ScriptObject akObj1, ScriptObject akObj2) native
/// ```
fn unregister_for_distance_events(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    object1: Object,
    object2: Object,
) -> Result<(), Error> {
    if object1.is_null() || object2.is_null() {
        return Ok(());
    }

    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| {
            matches!(
                event.kind,
                EventKind::DistanceLessThan {
                    object1: _,
                    object2: _,
                    distance: _
                } | EventKind::DistanceGreaterThan {
                    object1: _,
                    object2: _,
                    distance: _
                }
            )
        });

    Ok(())
}

/// `UnregisterForLOS`
///
/// ```text
/// Function(ObjectReference akViewer, ObjectReference akTarget) native
/// ```
fn unregister_for_los(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    viewer: Object,
    target: Object,
) -> Result<(), Error> {
    if viewer.is_null() || target.is_null() {
        return Ok(());
    }

    let viewer2 = viewer;
    let target2 = target;

    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| match event.kind {
            EventKind::DetectionLosGain { viewer, target }
            | EventKind::DetectionLosLost { viewer, target }
            | EventKind::DirectLosGain {
                viewer,
                target,
                viewer_node: _,
                target_node: _,
            }
            | EventKind::DirectLosLost {
                viewer,
                target,
                viewer_node: _,
                target_node: _,
            } => viewer == viewer2 && target == target2,
            _ => false,
        });

    Ok(())
}

/// `UnregisterForMagicEffectApplyEvent`
///
/// ```text
/// Function(ScriptObject akTarget, ScriptObject akCasterFilter, Form akEffectFilter, bool abMatch) native
/// ```
fn unregister_for_magic_effect_apply_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
    caster_filter: Object,
    effect_filter: Object,
    must_match: bool,
) -> Result<(), Error> {
    if target.is_null() {
        return Ok(());
    }

    let caster_filter2 = (!caster_filter.is_null()).then_some(caster_filter);
    let effect_filter2 = (!effect_filter.is_null()).then_some(effect_filter);

    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| match event.kind {
            EventKind::MagicEffectApply {
                caster_filter,
                effect_filter,
                filters_must_match,
            } => {
                caster_filter == caster_filter2
                    && effect_filter == effect_filter2
                    && filters_must_match == must_match
                    && event.sender == EventSender::Object(target)
            }
            _ => false,
        });

    Ok(())
}

/// `UnregisterForMenuOpenCloseEvent`
///
/// ```text
/// Function(string asMenuName) native
/// ```
fn unregister_for_menu_open_close_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    menu_name: Object,
) -> Result<(), Error> {
    let menu_name = caller.string(menu_name).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::MenuOpenClose(menu_name),
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForPlayerSleep`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_player_sleep(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::PlayerSleep,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForPlayerTeleport`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_player_teleport(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::PlayerTeleport,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForPlayerWait`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_player_wait(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::PlayerWait,
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForRadiationDamageEvent`
///
/// ```text
/// Function(ScriptObject akTarget) native
/// ```
fn unregister_for_radiation_damage_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
) -> Result<(), Error> {
    if target.is_null() {
        return Ok(());
    }

    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::RadiationDamage,
            sender: EventSender::Object(target),
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForRemoteEvent`
///
/// ```text
/// Function(ScriptObject akEventSource, ScriptEventName asEventName) native
/// ```
fn unregister_for_remote_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    event_source: Object,
    event_name: Object,
) -> Result<(), Error> {
    if event_source.is_null() {
        return Ok(());
    }

    let event_name = caller.string(event_name);
    let Ok(kind) = event_name.parse() else {
        tracing::warn!("unknown event name: {}", event_name);
        return Ok(());
    };

    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            sender: EventSender::Object(event_source),
            kind,
            limit: None,
        },
    );

    Ok(())
}

/// `UnregisterForTrackedStatsEvent`
///
/// ```text
/// Function(string asStat) native
/// ```
fn unregister_for_tracked_stats_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    stat: Object,
) -> Result<(), Error> {
    let name = caller.string(stat).to_owned();

    caller
        .data_mut()
        .registration
        .unregister_matching(self_ptr, |event| match &event.kind {
            EventKind::TrackedStats { stat, threshold: _ } => *stat == name,
            _ => false,
        });

    Ok(())
}

/// `UnregisterForLooksMenuEvent`
///
/// ```text
/// Function() native
/// ```
fn unregister_for_looks_menu_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    caller.data_mut().registration.unregister(
        self_ptr,
        Event {
            kind: EventKind::LooksMenu,
            sender: EventSender::Any,
            limit: None,
        },
    );
    Ok(())
}

/// `UnregisterForHitEvent`
///
/// ```text
/// Function UnregisterForHitEvent(ScriptObject akTarget, ScriptObject akAggressorFilter, Form akSourceFilter, Form akProjectileFilter = None, int aiPowerFilter, int aiSneakFilter, int aiBashFilter, int aiBlockFilter, bool abMatch) native
/// ```
fn unregister_for_hit_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    target: Object,
    aggressor_filter: Object,
    source_filter: Object,
    projectile_filter: Object,
    power_filter: i32,
    sneak_filter: i32,
    bash_filter: i32,
    block_filter: i32,
    must_match: bool,
) -> Result<(), Error> {
    todo!()
}

/// `UnregisterForTutorialEvent`
///
/// ```text
/// Function(String asEventName) native
/// ```
fn unregister_for_tutorial_event(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    event_name: Object,
) -> Result<(), Error> {
    let name = caller.string(event_name).to_owned();

    caller.data_mut().registration.register(
        self_ptr,
        Event {
            kind: EventKind::TutorialEvent { name },
            sender: EventSender::Any,
            limit: None,
        },
    );

    Ok(())
}
