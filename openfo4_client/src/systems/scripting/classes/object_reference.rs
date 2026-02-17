use game_common::components::Transform;
use hashbrown::HashSet;
use hecs::Entity;
use openfo4_animation::Value;
use openfo4_animation::driver::AnimationDriver;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::state::npc::ObjectKeywords;
use crate::systems::inventory::Inventory;
use crate::systems::linked_refs::LinkedRefs;
use crate::systems::scripting::classes::{Object, State};
use crate::systems::scripting::effects::AddItemEffect;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("ObjectReference", "Activate", activate);
    engine.register_native_function("ObjectReference", "AddDependentAnimatedObjectReference", add_dependent_animated_object_reference);
    engine.register_native_function("ObjectReference", "AddItem", add_item);
    engine.register_native_function("ObjectReference", "AddKeyword", add_keyword);
    engine.register_native_function("ObjectReference", "AddTextReplacementData", add_text_replacement_data);
    engine.register_native_function("ObjectReference", "AddToMap", add_to_map);
    engine.register_native_function("ObjectReference", "ApplyConveyorBelt", apply_conveyor_belt);
    engine.register_native_function("ObjectReference", "ApplyHavokImpulse", apply_havok_impulse);
    engine.register_native_function("ObjectReference", "ApplyFanMotor", apply_fan_motor);
    engine.register_native_function("ObjectReference", "FanMotorOn", fan_motor_on);
    engine.register_native_function("ObjectReference", "IsFanMotorOn", is_fan_motor_on);
    engine.register_native_function("ObjectReference", "AttachMod", attach_mod);
    engine.register_native_function("ObjectReference", "AttachModToInventoryItem", attach_mod_to_inventory_item);
    engine.register_native_function("ObjectReference", "AttachTo", attach_to);
    engine.register_native_function("ObjectReference", "BlockActivation", block_activation);
    engine.register_native_function("ObjectReference", "CalculateEncounterLevel", calculate_encounter_level);
    engine.register_native_function("ObjectReference", "CanFastTravelToMarker", can_fast_travel_to_marker);
    engine.register_native_function("ObjectReference", "CanProduceForWorkshop", can_produce_for_workshop);
    engine.register_native_function("ObjectReference", "ClearDestruction", clear_destruction);
    engine.register_native_function("ObjectReference", "ClearFromOldLocations", clear_from_old_locations);
    engine.register_native_function("ObjectReference", "ConveyorBeltOn", conveyor_belt_on);
    engine.register_native_function("ObjectReference", "CountActorsLinkedToMe", count_actors_linked_to_me);
    engine.register_native_function("ObjectReference", "CountLinkedRefChain", count_linked_ref_chain);
    engine.register_native_function("ObjectReference", "CountRefsLinkedToMe", count_refs_linked_to_me);
    engine.register_native_function("ObjectReference", "CreateDetectionEvent", create_detection_event);
    engine.register_native_function("ObjectReference", "DamageObject", damage_object);
    engine.register_native_function("ObjectReference", "DamageValue", damage_value);
    engine.register_native_function("ObjectReference", "Delete", delete);
    engine.register_native_function("ObjectReference", "Disable", disable);
    engine.register_native_function("ObjectReference", "DisableLinkChain", disable_link_chain);
    engine.register_native_function("ObjectReference", "DisableNoWait", disable_no_wait);
    engine.register_native_function("ObjectReference", "Drop", drop);
    engine.register_native_function("ObjectReference", "DropFirstObject", drop_first_object);
    engine.register_native_function("ObjectReference", "DropObject", drop_object);
    engine.register_native_function("ObjectReference", "Enable", enable);
    engine.register_native_function("ObjectReference", "EnableFastTravel", enable_fast_travel);
    engine.register_native_function("ObjectReference", "EnableLinkChain", enable_link_chain);
    engine.register_native_function("ObjectReference", "EnableNoWait", enable_no_wait);
    engine.register_native_function("ObjectReference", "FindAllReferencesOfType", find_all_references_of_type);
    engine.register_native_function("ObjectReference", "FindAllReferencesWithKeyword", find_all_references_with_keyword);
    engine.register_native_function("ObjectReference", "ForceAddRagdollToWorld", force_add_ragdoll_to_world);
    engine.register_native_function("ObjectReference", "ForceRemoveRagdollFromWorld", force_remove_ragdoll_from_world);
    engine.register_native_function("ObjectReference", "GetActorOwner", get_actor_owner);
    engine.register_native_function("ObjectReference", "GetActorRefOwner", get_actor_ref_owner);
    engine.register_native_function("ObjectReference", "GetActorsLinkedToMe", get_actors_linked_to_me);
    engine.register_native_function("ObjectReference", "GetAngleX", get_angle_x);
    engine.register_native_function("ObjectReference", "GetAngleY", get_angle_y);
    engine.register_native_function("ObjectReference", "GetAngleZ", get_angle_z);
    engine.register_native_function("ObjectReference", "GetAnimationVariableBool", get_animation_variable_bool);
    engine.register_native_function("ObjectReference", "GetAnimationVariableInt", get_animation_variable_int);
    engine.register_native_function("ObjectReference", "GetAnimationVariableFloat", get_animation_variable_float);
    engine.register_native_function("ObjectReference", "GetBaseObject", get_base_object);
    engine.register_native_function("ObjectReference", "GetBaseValue", get_base_value);
    engine.register_native_function("ObjectReference", "GetCurrentDestructionStage", get_current_destruction_stage);
    engine.register_native_function("ObjectReference", "GetCurrentLocation", get_current_location);
    engine.register_native_function("ObjectReference", "GetCurrentScene", get_current_scene);
    engine.register_native_function("ObjectReference", "GetDistance", get_distance);
    engine.register_native_function("ObjectReference", "GetEditorLocation", get_editor_location);
    engine.register_native_function("ObjectReference", "GetEncounterZone", get_encounter_zone);
    engine.register_native_function("ObjectReference", "GetFactionOwner", get_faction_owner);
    engine.register_native_function("ObjectReference", "GetHeadingAngle", get_heading_angle);
    engine.register_native_function("ObjectReference", "GetHeight", get_height);
    engine.register_native_function("ObjectReference", "GetInventoryValue", get_inventory_value);
    engine.register_native_function("ObjectReference", "GetItemCount", get_item_count);
    engine.register_native_function("ObjectReference", "GetComponentCount", get_component_count);
    engine.register_native_function("ObjectReference", "GetContainer", get_container);
    engine.register_native_function("ObjectReference", "GetItemHealthPercent", get_item_health_percent);
    engine.register_native_function("ObjectReference", "GetKey", get_key);
    engine.register_native_function("ObjectReference", "GetLength", get_length);
    engine.register_native_function("ObjectReference", "GetLinkedRef", get_linked_ref);
    engine.register_native_function("ObjectReference", "GetLinkedRefChain", get_linked_ref_chain);
    engine.register_native_function("ObjectReference", "GetLinkedRefChildren", get_linked_ref_children);
    engine.register_native_function("ObjectReference", "GetLockLevel", get_lock_level);
    engine.register_native_function("ObjectReference", "GetLocRefTypes", get_loc_ref_types);
    engine.register_native_function("ObjectReference", "GetNthLinkedRef", get_nth_linked_ref);
    engine.register_native_function("ObjectReference", "GetMass", get_mass);
    engine.register_native_function("ObjectReference", "GetOpenState", get_open_state);
    engine.register_native_function("ObjectReference", "GetParentCell", get_parent_cell);
    engine.register_native_function("ObjectReference", "GetPositionX", get_position_x);
    engine.register_native_function("ObjectReference", "GetPositionY", get_position_y);
    engine.register_native_function("ObjectReference", "GetPositionZ", get_position_z);
    engine.register_native_function("ObjectReference", "GetRadioFrequency", get_radio_frequency);
    engine.register_native_function("ObjectReference", "GetRadioVolume", get_radio_volume);
    engine.register_native_function("ObjectReference", "GetResourceDamage", get_resource_damage);
    engine.register_native_function("ObjectReference", "GetSafePosition", get_safe_position);
    engine.register_native_function("ObjectReference", "GetRefsLinkedToMe", get_refs_linked_to_me);
    engine.register_native_function("ObjectReference", "GetWorkshopOwnedObjects", get_workshop_owned_objects);
    engine.register_native_function("ObjectReference", "GetWorkshopResourceDamage", get_workshop_resource_damage);
    engine.register_native_function("ObjectReference", "GetWorkshopResourceObjects", get_workshop_resource_objects);
    engine.register_native_function("ObjectReference", "GetScale", get_scale);
    engine.register_native_function("ObjectReference", "GetTeleportCell", get_teleport_cell);
    engine.register_native_function("ObjectReference", "GetTransitionCell", get_transition_cell);
    engine.register_native_function("ObjectReference", "GetTransmitterDistance", get_transmitter_distance);
    engine.register_native_function("ObjectReference", "GetTriggerObjectCount", get_trigger_object_count);
    engine.register_native_function("ObjectReference", "GetValue", get_value);
    engine.register_native_function("ObjectReference", "GetValuePercentage", get_value_percentage);
    engine.register_native_function("ObjectReference", "GetVoiceType", get_voice_type);
    engine.register_native_function("ObjectReference", "GetWidth", get_width);
    engine.register_native_function("ObjectReference", "GetWorldSpace", get_world_space);
    engine.register_native_function("ObjectReference", "HasActorRefOwner", has_actor_ref_owner);
    engine.register_native_function("ObjectReference", "HasDirectLOS", has_direct_los);
    engine.register_native_function("ObjectReference", "HasEffectKeyword", has_effect_keyword);
    engine.register_native_function("ObjectReference", "HasKeyword", has_keyword);
    engine.register_native_function("ObjectReference", "HasKeywordInFormList", has_keyword_in_form_list);
    engine.register_native_function("ObjectReference", "HasLocRefType", has_loc_ref_type);
    engine.register_native_function("ObjectReference", "HasNode", has_node);
    engine.register_native_function("ObjectReference", "HasSharedPowerGrid", has_shared_power_grid);
    engine.register_native_function("ObjectReference", "IgnoreFriendlyHits", ignore_friendly_hits);
    engine.register_native_function("ObjectReference", "InterruptCast", interrupt_cast);
    engine.register_native_function("ObjectReference", "IsActivateChild", is_activate_child);
    engine.register_native_function("ObjectReference", "IsActivationBlocked", is_activation_blocked);
    engine.register_native_function("ObjectReference", "Is3DLoaded", is3_d_loaded);
    engine.register_native_function("ObjectReference", "IsConveyorBeltOn", is_conveyor_belt_on);
    engine.register_native_function("ObjectReference", "IsCreated", is_created);
    engine.register_native_function("ObjectReference", "IsDeleted", is_deleted);
    engine.register_native_function("ObjectReference", "IsDestroyed", is_destroyed);
    engine.register_native_function("ObjectReference", "IsDisabled", is_disabled);
    engine.register_native_function("ObjectReference", "IsFurnitureInUse", is_furniture_in_use);
    engine.register_native_function("ObjectReference", "IsFurnitureMarkerInUse", is_furniture_marker_in_use);
    engine.register_native_function("ObjectReference", "IsIgnoringFriendlyHits", is_ignoring_friendly_hits);
    engine.register_native_function("ObjectReference", "IsInDialogueWithPlayer", is_in_dialogue_with_player);
    engine.register_native_function("ObjectReference", "IsLockBroken", is_lock_broken);
    engine.register_native_function("ObjectReference", "IsLocked", is_locked);
    engine.register_native_function("ObjectReference", "IsMapMarkerVisible", is_map_marker_visible);
    engine.register_native_function("ObjectReference", "IsOwnedBy", is_owned_by);
    engine.register_native_function("ObjectReference", "IsPowered", is_powered);
    engine.register_native_function("ObjectReference", "IsQuestItem", is_quest_item);
    engine.register_native_function("ObjectReference", "IsRadioOn", is_radio_on);
    engine.register_native_function("ObjectReference", "IsRefInTransitionCell", is_ref_in_transition_cell);
    engine.register_native_function("ObjectReference", "IsTeleportAreaLoaded", is_teleport_area_loaded);
    engine.register_native_function("ObjectReference", "IsWithinBuildableArea", is_within_buildable_area);
    engine.register_native_function("ObjectReference", "KnockAreaEffect", knock_area_effect);
    engine.register_native_function("ObjectReference", "Lock", lock);
    engine.register_native_function("ObjectReference", "MakeRadioReceiver", make_radio_receiver);
    engine.register_native_function("ObjectReference", "MakeTransmitterRepeater", make_transmitter_repeater);
    engine.register_native_function("ObjectReference", "ModValue", mod_value);
    engine.register_native_function("ObjectReference", "MoveTo", move_to);
    engine.register_native_function("ObjectReference", "MoveToMyEditorLocation", move_to_my_editor_location);
    engine.register_native_function("ObjectReference", "MoveToNearestNavmeshLocation", move_to_nearest_navmesh_location);
    engine.register_native_function("ObjectReference", "MoveToNode", move_to_node);
    engine.register_native_function("ObjectReference", "OpenWorkshopSettlementMenu", open_workshop_settlement_menu);
    engine.register_native_function("ObjectReference", "OpenWorkshopSettlementMenuEx", open_workshop_settlement_menu_ex);
    engine.register_native_function("ObjectReference", "RaidTargetsAvailable", raid_targets_available);
    engine.register_native_function("ObjectReference", "PlaceAtMe", place_at_me);
    engine.register_native_function("ObjectReference", "PlaceActorAtMe", place_actor_at_me);
    engine.register_native_function("ObjectReference", "PlaceAtNode", place_at_node);
    engine.register_native_function("ObjectReference", "PlayAnimation", play_animation);
    engine.register_native_function("ObjectReference", "PlayAnimationAndWait", play_animation_and_wait);
    engine.register_native_function("ObjectReference", "PlayGamebryoAnimation", play_gamebryo_animation);
    engine.register_native_function("ObjectReference", "PlayImpactEffect", play_impact_effect);
    engine.register_native_function("ObjectReference", "PlaySyncedAnimationSS", play_synced_animation_ss);
    engine.register_native_function("ObjectReference", "PlaySyncedAnimationAndWaitSS", play_synced_animation_and_wait_ss);
    engine.register_native_function("ObjectReference", "PlayTerrainEffect", play_terrain_effect);
    engine.register_native_function("ObjectReference", "PauseAudio", pause_audio);
    engine.register_native_function("ObjectReference", "PreloadExteriorCell", preload_exterior_cell);
    engine.register_native_function("ObjectReference", "PreloadTargetArea", preload_target_area);
    engine.register_native_function("ObjectReference", "ProcessTrapHit", process_trap_hit);
    engine.register_native_function("ObjectReference", "PushActorAway", push_actor_away);
    engine.register_native_function("ObjectReference", "RecalculateResources", recalculate_resources);
    engine.register_native_function("ObjectReference", "RemoveAllItems", remove_all_items);
    engine.register_native_function("ObjectReference", "RemoveComponents", remove_components);
    engine.register_native_function("ObjectReference", "RemoveItem", remove_item);
    engine.register_native_function("ObjectReference", "RemoveItemByComponent", remove_item_by_component);
    engine.register_native_function("ObjectReference", "RemoveKeyword", remove_keyword);
    engine.register_native_function("ObjectReference", "RemoveAllMods", remove_all_mods);
    engine.register_native_function("ObjectReference", "RemoveAllModsFromInventoryItem", remove_all_mods_from_inventory_item);
    engine.register_native_function("ObjectReference", "RemoveMod", remove_mod);
    engine.register_native_function("ObjectReference", "RemoveModFromInventoryItem", remove_mod_from_inventory_item);
    engine.register_native_function("ObjectReference", "RemoveDependentAnimatedObjectReference", remove_dependent_animated_object_reference);
    engine.register_native_function("ObjectReference", "Repair", repair);
    engine.register_native_function("ObjectReference", "Reset", reset);
    engine.register_native_function("ObjectReference", "ResetKeyword", reset_keyword);
    engine.register_native_function("ObjectReference", "RestoreValue", restore_value);
    engine.register_native_function("ObjectReference", "ResumeAudio", resume_audio);
    engine.register_native_function("ObjectReference", "ReverseConveyorBelt", reverse_conveyor_belt);
    engine.register_native_function("ObjectReference", "Say", say);
    engine.register_native_function("ObjectReference", "SayCustom", say_custom);
    engine.register_native_function("ObjectReference", "SendStealAlarm", send_steal_alarm);
    engine.register_native_function("ObjectReference", "SetActivateTextOverride", set_activate_text_override);
    engine.register_native_function("ObjectReference", "SetActorCause", set_actor_cause);
    engine.register_native_function("ObjectReference", "SetActorOwner", set_actor_owner);
    engine.register_native_function("ObjectReference", "SetActorRefOwner", set_actor_ref_owner);
    engine.register_native_function("ObjectReference", "SetAngle", set_angle);
    engine.register_native_function("ObjectReference", "SetAnimationVariableBool", set_animation_variable_bool);
    engine.register_native_function("ObjectReference", "SetAnimationVariableInt", set_animation_variable_int);
    engine.register_native_function("ObjectReference", "SetAnimationVariableFloat", set_animation_variable_float);
    engine.register_native_function("ObjectReference", "SetAttractionActive", set_attraction_active);
    engine.register_native_function("ObjectReference", "SetConveyorBeltVelocity", set_conveyor_belt_velocity);
    engine.register_native_function("ObjectReference", "SetDestroyed", set_destroyed);
    engine.register_native_function("ObjectReference", "SetDirectAtTarget", set_direct_at_target);
    engine.register_native_function("ObjectReference", "SetFactionOwner", set_faction_owner);
    engine.register_native_function("ObjectReference", "SetHarvested", set_harvested);
    engine.register_native_function("ObjectReference", "SetLinkedRef", set_linked_ref);
    engine.register_native_function("ObjectReference", "SetLockLevel", set_lock_level);
    engine.register_native_function("ObjectReference", "SetLocRefType", set_loc_ref_type);
    engine.register_native_function("ObjectReference", "SetMotionType", set_motion_type);
    engine.register_native_function("ObjectReference", "SetNoFavorAllowed", set_no_favor_allowed);
    engine.register_native_function("ObjectReference", "SetOpen", set_open);
    engine.register_native_function("ObjectReference", "SetPersistLoc", set_persist_loc);
    engine.register_native_function("ObjectReference", "SetPlayerHasTaken", set_player_has_taken);
    engine.register_native_function("ObjectReference", "SetPosition", set_position);
    engine.register_native_function("ObjectReference", "SetRadioOn", set_radio_on);
    engine.register_native_function("ObjectReference", "SetRadioFrequency", set_radio_frequency);
    engine.register_native_function("ObjectReference", "SetRadioVolume", set_radio_volume);
    engine.register_native_function("ObjectReference", "SetScale", set_scale);
    engine.register_native_function("ObjectReference", "SetValue", set_value);
    engine.register_native_function("ObjectReference", "TranslateTo", translate_to);
    engine.register_native_function("ObjectReference", "SplineTranslateTo", spline_translate_to);
    engine.register_native_function("ObjectReference", "SplineTranslateToRefNode", spline_translate_to_ref_node);
    engine.register_native_function("ObjectReference", "StartWorkshop", start_workshop);
    engine.register_native_function("ObjectReference", "StopTranslation", stop_translation);
    engine.register_native_function("ObjectReference", "StoreInWorkshop", store_in_workshop);
    engine.register_native_function("ObjectReference", "TetherToHorse", tether_to_horse);
    engine.register_native_function("ObjectReference", "WaitForAnimationEvent", wait_for_animation_event);
    engine.register_native_function("ObjectReference", "WaitFor3DLoad", wait_for3_d_load);
    engine.register_native_function("ObjectReference", "WaitForWorkshopResourceRecalc", wait_for_workshop_resource_recalc);
    engine.register_native_function("ObjectReference", "Scrap", scrap);
}

/// `Activate`
///
/// ```text
/// bool Function(ObjectReference akActivator, bool abDefaultProcessingOnly) native
/// ```
fn activate(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_activator: Object,
    ab_default_processing_only: bool,
) -> Result<bool, Exception> {
    todo!()
}

/// `AddDependentAnimatedObjectReference`
///
/// ```text
/// bool Function(ObjectReference akDependent) native
/// ```
fn add_dependent_animated_object_reference(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_dependent: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `AddItem`
///
/// ```text
/// Function(Form akItemToAdd, int aiCount, bool abSilent) native
/// ```
fn add_item(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    count: i32,
    silent: bool,
) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;

    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .map(|s| s.record())
        .flatten()
        .ok_or(Exception::Error)?;

    // Ignore negative item counts.
    let Ok(count) = u64::try_from(count) else {
        return Ok(());
    };

    caller.data_mut().run_effect_and_wait(AddItemEffect {
        entity,
        item,
        count,
    })?;

    Ok(())
}

/// `AddKeyword`
///
/// ```text
/// Function(Keyword apKeyword) native
/// ```
fn add_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let mut keywords = caller
        .data()
        .world()
        .get::<&mut ObjectKeywords>(entity)
        .unwrap();
    keywords.add(keyword);

    Ok(())
}

/// `AddTextReplacementData`
///
/// ```text
/// Function(string asTokenLabel, Form akForm) native
/// ```
fn add_text_replacement_data(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_token_label: isize,
    ak_form: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `AddToMap`
///
/// ```text
/// Function(bool abAllowFastTravel) native
/// ```
fn add_to_map(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_allow_fast_travel: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `ApplyConveyorBelt`
///
/// ```text
/// Function(string aTarget, float aLinVelX, float aLinVelY, float aLinVelZ, bool abOn, bool abReverse) native
/// ```
fn apply_conveyor_belt(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_target: isize,
    a_lin_vel_x: f32,
    a_lin_vel_y: f32,
    a_lin_vel_z: f32,
    ab_on: bool,
    ab_reverse: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `ApplyHavokImpulse`
///
/// ```text
/// Function(float afX, float afY, float afZ, float afMagnitude) native
/// ```
fn apply_havok_impulse(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_magnitude: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ApplyFanMotor`
///
/// ```text
/// Function(string aTarget, float aAxisX, float aAxisY, float aAxisZ, float aForce, bool abOn) native
/// ```
fn apply_fan_motor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_target: isize,
    a_axis_x: f32,
    a_axis_y: f32,
    a_axis_z: f32,
    a_force: f32,
    ab_on: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `FanMotorOn`
///
/// ```text
/// Function(bool abOn) native
/// ```
fn fan_motor_on(caller: Caller<'_, State>, self_ptr: Object, ab_on: bool) -> Result<(), Exception> {
    todo!()
}

/// `IsFanMotorOn`
///
/// ```text
/// bool Function() native
/// ```
fn is_fan_motor_on(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `AttachMod`
///
/// ```text
/// bool Function(ObjectMod akMod, int aiAttachIndex) native
/// ```
fn attach_mod(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_mod: Object,
    ai_attach_index: i32,
) -> Result<bool, Exception> {
    todo!()
}

/// `AttachModToInventoryItem`
///
/// ```text
/// bool Function(Form akItem, ObjectMod akMod) native
/// ```
fn attach_mod_to_inventory_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item: Object,
    ak_mod: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `AttachTo`
///
/// ```text
/// Function(ObjectReference akParent) native
/// ```
fn attach_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_parent: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `BlockActivation`
///
/// ```text
/// Function(bool abBlocked, bool abHideActivateText) native
/// ```
fn block_activation(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_blocked: bool,
    ab_hide_activate_text: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `CalculateEncounterLevel`
///
/// ```text
/// int Function(int aiDifficulty) native
/// ```
fn calculate_encounter_level(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_difficulty: i32,
) -> Result<i32, Exception> {
    todo!()
}

/// `CanFastTravelToMarker`
///
/// ```text
/// bool Function() native
/// ```
fn can_fast_travel_to_marker(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `CanProduceForWorkshop`
///
/// ```text
/// bool Function() native
/// ```
fn can_produce_for_workshop(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `ClearDestruction`
///
/// ```text
/// Function() native
/// ```
fn clear_destruction(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ClearFromOldLocations`
///
/// ```text
/// Function() native
/// ```
fn clear_from_old_locations(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ConveyorBeltOn`
///
/// ```text
/// Function(bool abOn) native
/// ```
fn conveyor_belt_on(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_on: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `CountActorsLinkedToMe`
///
/// ```text
/// int Function(Keyword apLinkKeyword, Keyword apExcludeKeyword) native
/// ```
fn count_actors_linked_to_me(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_link_keyword: Object,
    ap_exclude_keyword: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `CountLinkedRefChain`
///
/// ```text
/// int Function(keyword apKeyword, int maxExpectedLinkedRefs) native
/// ```
fn count_linked_ref_chain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
    max_expected_linked_refs: i32,
) -> Result<i32, Exception> {
    let Ok(max_count) = usize::try_from(max_expected_linked_refs) else {
        return Ok(0);
    };

    let iter = LinkedRefChainIter::new(&caller, self_ptr, keyword).ok_or(Exception::Error)?;
    Ok(iter.take(max_count).count().try_into().unwrap_or(i32::MAX))
}

/// `CountRefsLinkedToMe`
///
/// ```text
/// int Function(Keyword apLinkKeyword, Keyword apExcludeKeyword) native
/// ```
fn count_refs_linked_to_me(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_link_keyword: Object,
    ap_exclude_keyword: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `CreateDetectionEvent`
///
/// ```text
/// Function(Actor akOwner, int aiSoundLevel) native
/// ```
fn create_detection_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_owner: Object,
    ai_sound_level: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `DamageObject`
///
/// ```text
/// Function(float afDamage) native
/// ```
fn damage_object(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_damage: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `DamageValue`
///
/// ```text
/// Function(ActorValue akAV, float afDamage) native
/// ```
fn damage_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
    af_damage: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `Delete`
///
/// ```text
/// Function() native
/// ```
fn delete(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Disable`
///
/// ```text
/// Function(bool abFadeOut) native
/// ```
fn disable(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_fade_out: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `DisableLinkChain`
///
/// ```text
/// Function(Keyword apKeyword, bool abFadeOut) native
/// ```
fn disable_link_chain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
    ab_fade_out: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `DisableNoWait`
///
/// ```text
/// Function(bool abFadeOut) native
/// ```
fn disable_no_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_fade_out: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `Drop`
///
/// ```text
/// Function(bool abSilent) native
/// ```
fn drop(caller: Caller<'_, State>, self_ptr: Object, ab_silent: bool) -> Result<(), Exception> {
    todo!()
}

/// `DropFirstObject`
///
/// ```text
/// ObjectReference Function(bool abInitiallyDisabled) native
/// ```
fn drop_first_object(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_initially_disabled: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `DropObject`
///
/// ```text
/// ObjectReference Function(Form akObject, int aiCount) native
/// ```
fn drop_object(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_object: Object,
    ai_count: i32,
) -> Result<Object, Exception> {
    todo!()
}

/// `Enable`
///
/// ```text
/// Function(bool abFadeIn) native
/// ```
fn enable(caller: Caller<'_, State>, self_ptr: Object, ab_fade_in: bool) -> Result<(), Exception> {
    todo!()
}

/// `EnableFastTravel`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_fast_travel(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableLinkChain`
///
/// ```text
/// Function(Keyword apKeyword, bool abFadeIn) native
/// ```
fn enable_link_chain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
    ab_fade_in: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableNoWait`
///
/// ```text
/// Function(bool abFadeIn) native
/// ```
fn enable_no_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_fade_in: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `FindAllReferencesOfType`
///
/// ```text
/// ObjectReference[] Function(Form akObjectOrList, float afRadius) native
/// ```
fn find_all_references_of_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_object_or_list: Object,
    af_radius: f32,
) -> Result<isize, Exception> {
    todo!()
}

/// `FindAllReferencesWithKeyword`
///
/// ```text
/// ObjectReference[] Function(Form akKeywordOrList, float afRadius) native
/// ```
fn find_all_references_with_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword_or_list: Object,
    af_radius: f32,
) -> Result<isize, Exception> {
    todo!()
}

/// `ForceAddRagdollToWorld`
///
/// ```text
/// Function() native
/// ```
fn force_add_ragdoll_to_world(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceRemoveRagdollFromWorld`
///
/// ```text
/// Function() native
/// ```
fn force_remove_ragdoll_from_world(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetActorOwner`
///
/// ```text
/// ActorBase Function() native
/// ```
fn get_actor_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetActorRefOwner`
///
/// ```text
/// Actor Function() native
/// ```
fn get_actor_ref_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetActorsLinkedToMe`
///
/// ```text
/// Actor[] Function(Keyword apLinkKeyword, Keyword apExcludeKeyword) native
/// ```
fn get_actors_linked_to_me(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_link_keyword: Object,
    ap_exclude_keyword: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetAngleX`
///
/// ```text
/// float Function() native
/// ```
fn get_angle_x(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetAngleY`
///
/// ```text
/// float Function() native
/// ```
fn get_angle_y(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetAngleZ`
///
/// ```text
/// float Function() native
/// ```
fn get_angle_z(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetAnimationVariableBool`
///
/// ```text
/// bool Function(string arVariableName) native
/// ```
fn get_animation_variable_bool(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
) -> Result<bool, Exception> {
    let name = unsafe { caller.string(variable_name) };

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(driver) = caller.data().world().get::<&AnimationDriver>(entity) else {
        return Ok(false);
    };

    if let Some(Value::Bool(value)) = driver.get_variable(name) {
        return Ok(value);
    }

    Ok(false)
}

/// `GetAnimationVariableInt`
///
/// ```text
/// int Function(string arVariableName) native
/// ```
fn get_animation_variable_int(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
) -> Result<i32, Exception> {
    let name = unsafe { caller.string(variable_name) };

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(driver) = caller.data().world().get::<&AnimationDriver>(entity) else {
        return Ok(0);
    };

    if let Some(Value::I32(value)) = driver.get_variable(name) {
        return Ok(value);
    }

    Ok(0)
}

/// `GetAnimationVariableFloat`
///
/// ```text
/// float Function(string arVariableName) native
/// ```
fn get_animation_variable_float(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
) -> Result<f32, Exception> {
    let name = unsafe { caller.string(variable_name) };

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(driver) = caller.data().world().get::<&AnimationDriver>(entity) else {
        return Ok(0.0);
    };

    if let Some(Value::F32(value)) = driver.get_variable(name) {
        return Ok(value);
    }

    Ok(0.0)
}

/// `GetBaseObject`
///
/// ```text
/// Form Function() native
/// ```
fn get_base_object(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetBaseValue`
///
/// ```text
/// float Function(ActorValue akAV) native
/// ```
fn get_base_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetCurrentDestructionStage`
///
/// ```text
/// int Function() native
/// ```
fn get_current_destruction_stage(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetCurrentLocation`
///
/// ```text
/// Location Function() native
/// ```
fn get_current_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCurrentScene`
///
/// ```text
/// Scene Function() native
/// ```
fn get_current_scene(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetDistance`
///
/// ```text
/// float Function(ObjectReference akOther) native
/// ```
fn get_distance(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<f32, Exception> {
    let this = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let other = caller.data().entity(other).ok_or(Exception::Error)?;

    // Entities are guaranteed to exist, otherwise `entity()` would return `None`.
    let this_transform = caller.data().world().get::<&Transform>(this).unwrap();
    let other_transform = caller.data().world().get::<&Transform>(other).unwrap();

    let a = this_transform.to_nif();
    let b = other_transform.to_nif();

    Ok(a.translation.distance(b.translation))
}

/// `GetEditorLocation`
///
/// ```text
/// Location Function() native
/// ```
fn get_editor_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetEncounterZone`
///
/// ```text
/// EncounterZone Function() native
/// ```
fn get_encounter_zone(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetFactionOwner`
///
/// ```text
/// Faction Function() native
/// ```
fn get_faction_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetHeadingAngle`
///
/// ```text
/// float Function(ObjectReference akOther) native
/// ```
fn get_heading_angle(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetHeight`
///
/// ```text
/// float Function() native
/// ```
fn get_height(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetInventoryValue`
///
/// ```text
/// int Function() native
/// ```
fn get_inventory_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetItemCount`
///
/// ```text
/// int Function(Form akItem) native
/// ```
fn get_item_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
) -> Result<i32, Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;

    let Ok(inventory) = caller.data().world().get::<&Inventory>(entity) else {
        return Ok(0);
    };

    let count = if item.is_null() {
        inventory
            .items()
            .fold(0_u64, |sum, (_, stack)| sum.saturating_add(stack.count))
    } else {
        let id = caller
            .data()
            .objects
            .get_by_addr(item)
            .and_then(|s| s.record())
            .ok_or(Exception::Error)?;
        inventory
            .get_by_form(id)
            .fold(0_u64, |sum, (_, stack)| sum.saturating_add(stack.count))
    };

    Ok(count.try_into().unwrap_or(i32::MAX))
}

/// `GetComponentCount`
///
/// ```text
/// int Function(Form akItem) native
/// ```
fn get_component_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetContainer`
///
/// ```text
/// ObjectReference Function() native
/// ```
fn get_container(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetItemHealthPercent`
///
/// ```text
/// float Function() native
/// ```
fn get_item_health_percent(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetKey`
///
/// ```text
/// Key Function() native
/// ```
fn get_key(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetLength`
///
/// ```text
/// float Function() native
/// ```
fn get_length(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetLinkedRef`
///
/// ```text
/// ObjectReference Function(Keyword apKeyword) native
/// ```
fn get_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Object, Exception> {
    let mut iter = LinkedRefChainIter::new(&caller, self_ptr, keyword).ok_or(Exception::Error)?;
    let addr = iter.next().unwrap_or(Object::NULL);
    Ok(addr)
}

/// `GetLinkedRefChain`
///
/// ```text
/// ObjectReference[] Function(keyword apKeyword, int iMaxExpectedLinkedRefs) native
/// ```
fn get_linked_ref_chain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
    max_linked_refs: i32,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetLinkedRefChildren`
///
/// ```text
/// ObjectReference[] Function(keyword apKeyword) native
/// ```
fn get_linked_ref_children(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetLockLevel`
///
/// ```text
/// int Function() native
/// ```
fn get_lock_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetLocRefTypes`
///
/// ```text
/// LocationRefType[] Function() native
/// ```
fn get_loc_ref_types(caller: Caller<'_, State>, self_ptr: Object) -> Result<isize, Exception> {
    todo!()
}

/// `GetNthLinkedRef`
///
/// ```text
/// ObjectReference Function(int aiLinkedRef, Keyword apKeyword) native
/// ```
fn get_nth_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    index: i32,
    keyword: Object,
) -> Result<Object, Exception> {
    let index = usize::try_from(index).map_err(|_| Exception::Error)?;

    let mut iter = LinkedRefChainIter::new(&caller, self_ptr, keyword).ok_or(Exception::Error)?;
    let addr = iter.nth(index).unwrap_or(Object::NULL);
    Ok(addr)
}

/// `GetMass`
///
/// ```text
/// float Function() native
/// ```
fn get_mass(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetOpenState`
///
/// ```text
/// int Function() native
/// ```
fn get_open_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetParentCell`
///
/// ```text
/// Cell Function() native
/// ```
fn get_parent_cell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetPositionX`
///
/// ```text
/// float Function() native
/// ```
fn get_position_x(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    let transform = get_transform(caller, self_ptr).ok_or(Exception::Error)?;
    Ok(transform.translation.x)
}

/// `GetPositionY`
///
/// ```text
/// float Function() native
/// ```
fn get_position_y(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    let transform = get_transform(caller, self_ptr).ok_or(Exception::Error)?;
    Ok(transform.translation.y)
}

/// `GetPositionZ`
///
/// ```text
/// float Function() native
/// ```
fn get_position_z(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    let transform = get_transform(caller, self_ptr).ok_or(Exception::Error)?;
    Ok(transform.translation.z)
}

/// `GetRadioFrequency`
///
/// ```text
/// float Function() native
/// ```
fn get_radio_frequency(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetRadioVolume`
///
/// ```text
/// float Function() native
/// ```
fn get_radio_volume(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetResourceDamage`
///
/// ```text
/// float Function(ActorValue akValue=None) native
/// ```
fn get_resource_damage(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_value_none: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetSafePosition`
///
/// ```text
/// float[] Function(float aSearchRadius, float aSafeRadius) native
/// ```
fn get_safe_position(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_search_radius: f32,
    a_safe_radius: f32,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetRefsLinkedToMe`
///
/// ```text
/// ObjectReference[] Function(Keyword apLinkKeyword, Keyword apExcludeKeyword) native
/// ```
fn get_refs_linked_to_me(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_link_keyword: Object,
    ap_exclude_keyword: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetWorkshopOwnedObjects`
///
/// ```text
/// ObjectReference[] Function(Actor akActor) native
/// ```
fn get_workshop_owned_objects(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetWorkshopResourceDamage`
///
/// ```text
/// float Function(ActorValue akValue) native
/// ```
fn get_workshop_resource_damage(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_value: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetWorkshopResourceObjects`
///
/// ```text
/// ObjectReference[] Function(ActorValue akAV, int aiOption) native
/// ```
fn get_workshop_resource_objects(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
    ai_option: i32,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetScale`
///
/// ```text
/// float Function() native
/// ```
fn get_scale(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    let transform = get_transform(caller, self_ptr).ok_or(Exception::Error)?;
    // Assume we have uniform scale.
    Ok(transform.scale.x)
}

/// `GetTeleportCell`
///
/// ```text
/// Cell Function() native
/// ```
fn get_teleport_cell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetTransitionCell`
///
/// ```text
/// Cell Function() native
/// ```
fn get_transition_cell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetTransmitterDistance`
///
/// ```text
/// float Function() native
/// ```
fn get_transmitter_distance(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetTriggerObjectCount`
///
/// ```text
/// int Function() native
/// ```
fn get_trigger_object_count(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetValue`
///
/// ```text
/// float Function(ActorValue akAV) native
/// ```
fn get_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetValuePercentage`
///
/// ```text
/// float Function(ActorValue akAV) native
/// ```
fn get_value_percentage(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetVoiceType`
///
/// ```text
/// VoiceType Function() native
/// ```
fn get_voice_type(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetWidth`
///
/// ```text
/// float Function() native
/// ```
fn get_width(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetWorldSpace`
///
/// ```text
/// WorldSpace Function() native
/// ```
fn get_world_space(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `HasActorRefOwner`
///
/// ```text
/// bool Function() native
/// ```
fn has_actor_ref_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `HasDirectLOS`
///
/// ```text
/// bool Function(ObjectReference akTarget, string asSourceNode, string asTargetNode) native
/// ```
fn has_direct_los(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    as_source_node: isize,
    as_target_node: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasEffectKeyword`
///
/// ```text
/// bool Function(Keyword akKeyword) native
/// ```
fn has_effect_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasKeyword`
///
/// ```text
/// bool Function(Keyword apKeyword) native
/// ```
fn has_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<bool, Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let keywords = caller
        .data()
        .world()
        .get::<&ObjectKeywords>(entity)
        .unwrap();
    Ok(keywords.contains(keyword))
}

/// `HasKeywordInFormList`
///
/// ```text
/// bool Function(FormList akKeywordList) native
/// ```
fn has_keyword_in_form_list(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword_list: Object,
) -> Result<bool, Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let form_list = caller
        .data()
        .objects
        .get_by_addr(keyword_list)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;
    let form_list = caller
        .data()
        .globals()
        .form_lists
        .get(&form_list)
        .ok_or(Exception::Error)?;

    let keywords = caller
        .data()
        .world()
        .get::<&ObjectKeywords>(entity)
        .unwrap();

    for id in form_list.iter() {
        if keywords.contains(id) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// `HasLocRefType`
///
/// ```text
/// bool Function(LocationRefType akRefType) native
/// ```
fn has_loc_ref_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ref_type: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasNode`
///
/// ```text
/// bool Function(string asNodeName) native
/// ```
fn has_node(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_node_name: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasSharedPowerGrid`
///
/// ```text
/// bool Function(ObjectReference akCompare) native
/// ```
fn has_shared_power_grid(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_compare: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IgnoreFriendlyHits`
///
/// ```text
/// Function(bool abIgnore) native
/// ```
fn ignore_friendly_hits(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_ignore: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `InterruptCast`
///
/// ```text
/// Function() native
/// ```
fn interrupt_cast(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `IsActivateChild`
///
/// ```text
/// bool Function(ObjectReference akChild) native
/// ```
fn is_activate_child(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_child: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsActivationBlocked`
///
/// ```text
/// bool Function() native
/// ```
fn is_activation_blocked(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `Is3DLoaded`
///
/// ```text
/// bool Function() native
/// ```
fn is3_d_loaded(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsConveyorBeltOn`
///
/// ```text
/// bool Function() native
/// ```
fn is_conveyor_belt_on(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsCreated`
///
/// ```text
/// bool Function() native
/// ```
fn is_created(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsDeleted`
///
/// ```text
/// bool Function() native
/// ```
fn is_deleted(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsDestroyed`
///
/// ```text
/// bool Function() native
/// ```
fn is_destroyed(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsDisabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_disabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsFurnitureInUse`
///
/// ```text
/// bool Function(bool abIgnoreReserved) native
/// ```
fn is_furniture_in_use(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_ignore_reserved: bool,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsFurnitureMarkerInUse`
///
/// ```text
/// bool Function(int aiMarker, bool abIgnoreReserved) native
/// ```
fn is_furniture_marker_in_use(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_marker: i32,
    ab_ignore_reserved: bool,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsIgnoringFriendlyHits`
///
/// ```text
/// bool Function() native
/// ```
fn is_ignoring_friendly_hits(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsInDialogueWithPlayer`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_dialogue_with_player(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsLockBroken`
///
/// ```text
/// bool Function() native
/// ```
fn is_lock_broken(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsLocked`
///
/// ```text
/// bool Function() native
/// ```
fn is_locked(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsMapMarkerVisible`
///
/// ```text
/// bool Function() native
/// ```
fn is_map_marker_visible(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsOwnedBy`
///
/// ```text
/// bool Function(Actor akOwner) native
/// ```
fn is_owned_by(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_owner: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsPowered`
///
/// ```text
/// bool Function() native
/// ```
fn is_powered(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsQuestItem`
///
/// ```text
/// bool Function() native
/// ```
fn is_quest_item(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsRadioOn`
///
/// ```text
/// bool Function() native
/// ```
fn is_radio_on(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsRefInTransitionCell`
///
/// ```text
/// bool Function(ObjectReference akRef) native
/// ```
fn is_ref_in_transition_cell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ref: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsTeleportAreaLoaded`
///
/// ```text
/// bool Function() native
/// ```
fn is_teleport_area_loaded(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsWithinBuildableArea`
///
/// ```text
/// bool Function(ObjectReference akRef) native
/// ```
fn is_within_buildable_area(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ref: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `KnockAreaEffect`
///
/// ```text
/// Function(float afMagnitude, float afRadius) native
/// ```
fn knock_area_effect(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_magnitude: f32,
    af_radius: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `Lock`
///
/// ```text
/// Function(bool abLock, bool abAsOwner) native
/// ```
fn lock(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_lock: bool,
    ab_as_owner: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `MakeRadioReceiver`
///
/// ```text
/// Function(float afFrequency, float afVolume, OutputModel aOverrideModel, bool abActive, bool abNoStatic) native
/// ```
fn make_radio_receiver(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_frequency: f32,
    af_volume: f32,
    a_override_model: Object,
    ab_active: bool,
    ab_no_static: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `MakeTransmitterRepeater`
///
/// ```text
/// Function(ObjectReference akTransmitterToRepeat, float afInnerRadius, float afOuterRadius, bool abUnlimitedRange) native
/// ```
fn make_transmitter_repeater(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_transmitter_to_repeat: Object,
    af_inner_radius: f32,
    af_outer_radius: f32,
    ab_unlimited_range: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `ModValue`
///
/// ```text
/// Function(ActorValue akAV, float afAmount) native
/// ```
fn mod_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
    af_amount: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `MoveTo`
///
/// ```text
/// Function(ObjectReference akTarget, float afXOffset, float afYOffset, float afZOffset, bool abMatchRotation) native
/// ```
fn move_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    af_x_offset: f32,
    af_y_offset: f32,
    af_z_offset: f32,
    ab_match_rotation: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `MoveToMyEditorLocation`
///
/// ```text
/// Function() native
/// ```
fn move_to_my_editor_location(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `MoveToNearestNavmeshLocation`
///
/// ```text
/// Function() native
/// ```
fn move_to_nearest_navmesh_location(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `MoveToNode`
///
/// ```text
/// Function(ObjectReference akTarget, string asNodeName, string asMatchNodeName) native
/// ```
fn move_to_node(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    as_node_name: isize,
    as_match_node_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `OpenWorkshopSettlementMenu`
///
/// ```text
/// Location Function(Keyword akActionKW, Message astrConfirm, Location aLocToHighlight) native
/// ```
fn open_workshop_settlement_menu(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_action_kw: Object,
    astr_confirm: Object,
    a_loc_to_highlight: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `OpenWorkshopSettlementMenuEx`
///
/// ```text
/// Location Function(Keyword akActionKW, Message astrConfirm, Location aLocToHighlight, FormList akIncludeKeywordList, FormList akExcludeKeywordList, bool abExcludeZeroPopulation, bool abOnlyOwnedWorkshops, bool abTurnOffHeader, bool abOnlyPotentialVassalSettlements, bool abDisableReservedByQuests) native
/// ```
fn open_workshop_settlement_menu_ex(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_action_kw: Object,
    astr_confirm: Object,
    a_loc_to_highlight: Object,
    ak_include_keyword_list: Object,
    ak_exclude_keyword_list: Object,
    ab_exclude_zero_population: bool,
    ab_only_owned_workshops: bool,
    ab_turn_off_header: bool,
    ab_only_potential_vassal_settlements: bool,
    ab_disable_reserved_by_quests: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `RaidTargetsAvailable`
///
/// ```text
/// int Function(Keyword akActionKW, Message astrConfirm, Location aLocToHighlight, FormList akIncludeKeywordList, FormList akExcludeKeywordList, bool abExcludeZeroPopulation, bool abOnlyOwnedWorkshops, bool abTurnOffHeader, bool abOnlyPotentialVassalSettlements, bool abDisableReservedByQuests) native
/// ```
fn raid_targets_available(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_action_kw: Object,
    astr_confirm: Object,
    a_loc_to_highlight: Object,
    ak_include_keyword_list: Object,
    ak_exclude_keyword_list: Object,
    ab_exclude_zero_population: bool,
    ab_only_owned_workshops: bool,
    ab_turn_off_header: bool,
    ab_only_potential_vassal_settlements: bool,
    ab_disable_reserved_by_quests: bool,
) -> Result<i32, Exception> {
    todo!()
}

/// `PlaceAtMe`
///
/// ```text
/// ObjectReference Function(Form akFormToPlace, int aiCount, bool abForcePersist, bool abInitiallyDisabled, bool abDeleteWhenAble) native
/// ```
fn place_at_me(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_form_to_place: Object,
    ai_count: i32,
    ab_force_persist: bool,
    ab_initially_disabled: bool,
    ab_delete_when_able: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `PlaceActorAtMe`
///
/// ```text
/// Actor Function(ActorBase akActorToPlace, int aiLevelMod, EncounterZone akZone) native
/// ```
fn place_actor_at_me(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor_to_place: Object,
    ai_level_mod: i32,
    ak_zone: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `PlaceAtNode`
///
/// ```text
/// ObjectReference Function(string asNodeName, Form akFormToPlace, int aiCount, bool abForcePersist, bool abInitiallyDisabled, bool abDeleteWhenAble, bool abAttach) native
/// ```
fn place_at_node(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_node_name: isize,
    ak_form_to_place: Object,
    ai_count: i32,
    ab_force_persist: bool,
    ab_initially_disabled: bool,
    ab_delete_when_able: bool,
    ab_attach: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `PlayAnimation`
///
/// ```text
/// bool Function(string asAnimation) native
/// ```
fn play_animation(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_animation: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayAnimationAndWait`
///
/// ```text
/// bool Function(string asAnimation, string asEventName) native
/// ```
fn play_animation_and_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_animation: isize,
    as_event_name: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayGamebryoAnimation`
///
/// ```text
/// bool Function(string asAnimation, bool abStartOver, float afEaseInTime) native
/// ```
fn play_gamebryo_animation(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_animation: isize,
    ab_start_over: bool,
    af_ease_in_time: f32,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayImpactEffect`
///
/// ```text
/// bool Function(ImpactDataSet akImpactEffect, string asNodeName, float afPickDirX, float afPickDirY, float afPickDirZ, float afPickLength, bool abApplyNodeRotation, bool abUseNodeLocalRotation) native
/// ```
fn play_impact_effect(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_impact_effect: Object,
    as_node_name: isize,
    af_pick_dir_x: f32,
    af_pick_dir_y: f32,
    af_pick_dir_z: f32,
    af_pick_length: f32,
    ab_apply_node_rotation: bool,
    ab_use_node_local_rotation: bool,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlaySyncedAnimationSS`
///
/// ```text
/// bool Function(string asAnimation1, ObjectReference akObj2, string asAnimation2) native
/// ```
fn play_synced_animation_ss(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_animation1: isize,
    ak_obj2: Object,
    as_animation2: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlaySyncedAnimationAndWaitSS`
///
/// ```text
/// bool Function(string asAnimation1, string asEvent1, ObjectReference akObj2, string asAnimation2, string asEvent2) native
/// ```
fn play_synced_animation_and_wait_ss(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_animation1: isize,
    as_event1: isize,
    ak_obj2: Object,
    as_animation2: isize,
    as_event2: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayTerrainEffect`
///
/// ```text
/// Function(string asEffectModelName, string asAttachBoneName) native
/// ```
fn play_terrain_effect(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_effect_model_name: isize,
    as_attach_bone_name: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `PauseAudio`
///
/// ```text
/// Function() native
/// ```
fn pause_audio(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `PreloadExteriorCell`
///
/// ```text
/// Function() native
/// ```
fn preload_exterior_cell(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Exception> {
    Ok(())
}

/// `PreloadTargetArea`
///
/// ```text
/// Function() native
/// ```
fn preload_target_area(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Exception> {
    Ok(())
}

/// `ProcessTrapHit`
///
/// ```text
/// Function(ObjectReference akTrap, float afDamage, float afPushback, float afXVel, float afYVel, float afZVel, float afXPos, float afYPos, float afZPos, int aeMaterial, float afStagger) native
/// ```
fn process_trap_hit(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_trap: Object,
    af_damage: f32,
    af_pushback: f32,
    af_x_vel: f32,
    af_y_vel: f32,
    af_z_vel: f32,
    af_x_pos: f32,
    af_y_pos: f32,
    af_z_pos: f32,
    ae_material: i32,
    af_stagger: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `PushActorAway`
///
/// ```text
/// Function(Actor akActorToPush, float aiKnockbackForce) native
/// ```
fn push_actor_away(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor_to_push: Object,
    ai_knockback_force: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `RecalculateResources`
///
/// ```text
/// Function() native
/// ```
fn recalculate_resources(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `RemoveAllItems`
///
/// ```text
/// Function(ObjectReference akTransferTo, bool abKeepOwnership) native
/// ```
fn remove_all_items(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_transfer_to: Object,
    ab_keep_ownership: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveComponents`
///
/// ```text
/// Function(Component akComponent, int aiCount, bool abSilent) native
/// ```
fn remove_components(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_component: Object,
    ai_count: i32,
    ab_silent: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveItem`
///
/// ```text
/// Function(Form akItemToRemove, int aiCount, bool abSilent, ObjectReference akOtherContainer) native
/// ```
fn remove_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item_to_remove: Object,
    ai_count: i32,
    ab_silent: bool,
    ak_other_container: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveItemByComponent`
///
/// ```text
/// Function(Form akComponentToRemove, int aiCount, bool abSilent, ObjectReference akOtherContainer) native
/// ```
fn remove_item_by_component(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_component_to_remove: Object,
    ai_count: i32,
    ab_silent: bool,
    ak_other_container: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveKeyword`
///
/// ```text
/// Function(Keyword apKeyword) native
/// ```
fn remove_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let mut keywords = caller
        .data()
        .world()
        .get::<&mut ObjectKeywords>(entity)
        .unwrap();
    keywords.remove(keyword);

    Ok(())
}

/// `RemoveAllMods`
///
/// ```text
/// Function() native
/// ```
fn remove_all_mods(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `RemoveAllModsFromInventoryItem`
///
/// ```text
/// Function(Form akItem) native
/// ```
fn remove_all_mods_from_inventory_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveMod`
///
/// ```text
/// Function(ObjectMod akMod) native
/// ```
fn remove_mod(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_mod: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveModFromInventoryItem`
///
/// ```text
/// Function(Form akItem, ObjectMod akMod) native
/// ```
fn remove_mod_from_inventory_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item: Object,
    ak_mod: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveDependentAnimatedObjectReference`
///
/// ```text
/// bool Function(ObjectReference akDependent) native
/// ```
fn remove_dependent_animated_object_reference(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_dependent: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `Repair`
///
/// ```text
/// Function() native
/// ```
fn repair(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function(ObjectReference akTarget) native
/// ```
fn reset(caller: Caller<'_, State>, self_ptr: Object, ak_target: Object) -> Result<(), Exception> {
    todo!()
}

/// `ResetKeyword`
///
/// ```text
/// Function(Keyword apKeyword) native
/// ```
fn reset_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let mut keywords = caller
        .data()
        .world()
        .get::<&mut ObjectKeywords>(entity)
        .unwrap();
    keywords.reset(keyword);

    Ok(())
}

/// `RestoreValue`
///
/// ```text
/// Function(ActorValue akAV, float afAmount) native
/// ```
fn restore_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
    af_amount: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ResumeAudio`
///
/// ```text
/// Function() native
/// ```
fn resume_audio(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ReverseConveyorBelt`
///
/// ```text
/// Function(bool abReverse) native
/// ```
fn reverse_conveyor_belt(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_reverse: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `Say`
///
/// ```text
/// Function(Topic akTopicToSay, Actor akActorToSpeakAs, bool abSpeakInPlayersHead, ObjectReference akTarget) native
/// ```
fn say(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_topic_to_say: Object,
    ak_actor_to_speak_as: Object,
    ab_speak_in_players_head: bool,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SayCustom`
///
/// ```text
/// Function(Keyword akKeywordToSay, Actor akActorToSpeakAs, bool abSpeakInPlayersHead, ObjectReference akTarget) native
/// ```
fn say_custom(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword_to_say: Object,
    ak_actor_to_speak_as: Object,
    ab_speak_in_players_head: bool,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SendStealAlarm`
///
/// ```text
/// Function(Actor akThief) native
/// ```
fn send_steal_alarm(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_thief: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetActivateTextOverride`
///
/// ```text
/// Function(Message akText) native
/// ```
fn set_activate_text_override(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_text: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetActorCause`
///
/// ```text
/// Function(Actor akActor) native
/// ```
fn set_actor_cause(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetActorOwner`
///
/// ```text
/// Function(ActorBase akActorBase, bool abNoCrime) native
/// ```
fn set_actor_owner(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor_base: Object,
    ab_no_crime: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetActorRefOwner`
///
/// ```text
/// Function(Actor akActor, bool abNoCrime) native
/// ```
fn set_actor_ref_owner(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
    ab_no_crime: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAngle`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn set_angle(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAnimationVariableBool`
///
/// ```text
/// Function(string arVariableName, bool abNewValue) native
/// ```
fn set_animation_variable_bool(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
    new_value: bool,
) -> Result<(), Exception> {
    let variable_name = unsafe { caller.string(variable_name) };

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(mut driver) = caller.data().world().get::<&mut AnimationDriver>(entity) else {
        return Ok(());
    };

    if let Some(Value::Bool(value)) = driver.get_variable_mut(variable_name) {
        *value = new_value;
    }

    Ok(())
}

/// `SetAnimationVariableInt`
///
/// ```text
/// Function(string arVariableName, int aiNewValue) native
/// ```
fn set_animation_variable_int(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
    new_value: i32,
) -> Result<(), Exception> {
    let variable_name = unsafe { caller.string(variable_name) };

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(mut driver) = caller.data().world().get::<&mut AnimationDriver>(entity) else {
        return Ok(());
    };

    if let Some(Value::I32(value)) = driver.get_variable_mut(variable_name) {
        *value = new_value;
    }

    Ok(())
}

/// `SetAnimationVariableFloat`
///
/// ```text
/// Function(string arVariableName, float afNewValue) native
/// ```
fn set_animation_variable_float(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
    new_value: f32,
) -> Result<(), Exception> {
    let variable_name = unsafe { caller.string(variable_name) };

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(mut driver) = caller.data().world().get::<&mut AnimationDriver>(entity) else {
        return Ok(());
    };

    if let Some(Value::F32(value)) = driver.get_variable_mut(variable_name) {
        *value = new_value;
    }

    Ok(())
}

/// `SetAttractionActive`
///
/// ```text
/// Function(Keyword apKeyword, bool abActive) native
/// ```
fn set_attraction_active(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
    ab_active: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetConveyorBeltVelocity`
///
/// ```text
/// Function(float afLinVelX, float afLinVelY, float afLinVelZ) native
/// ```
fn set_conveyor_belt_velocity(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_lin_vel_x: f32,
    af_lin_vel_y: f32,
    af_lin_vel_z: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetDestroyed`
///
/// ```text
/// Function(bool abDestroyed) native
/// ```
fn set_destroyed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_destroyed: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetDirectAtTarget`
///
/// ```text
/// Function(ObjectReference akTarget) native
/// ```
fn set_direct_at_target(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetFactionOwner`
///
/// ```text
/// Function(Faction akFaction, bool abNoCrime) native
/// ```
fn set_faction_owner(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_faction: Object,
    ab_no_crime: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetHarvested`
///
/// ```text
/// Function(bool abHarvested) native
/// ```
fn set_harvested(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_harvested: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetLinkedRef`
///
/// ```text
/// Function(ObjectReference akLinkedRef, Keyword apKeyword) native
/// ```
fn set_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    linked_ref: Object,
    keyword: Object,
) -> Result<(), Exception> {
    let keyword = (!keyword.is_null())
        .then(|| {
            caller
                .data()
                .objects
                .get_by_addr(keyword)
                .and_then(|s| s.record())
        })
        .ok_or(Exception::Error)?;

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let mut linked_refs = caller
        .data()
        .world()
        .get::<&mut LinkedRefs>(entity)
        .unwrap();

    if linked_ref.is_null() {
        linked_refs.remove(keyword);
    } else {
        let linked_ref = caller.data().entity(linked_ref).ok_or(Exception::Error)?;
        linked_refs.set(keyword, linked_ref);
    }

    Ok(())
}

/// `SetLockLevel`
///
/// ```text
/// Function(int aiLockLevel) native
/// ```
fn set_lock_level(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_lock_level: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetLocRefType`
///
/// ```text
/// Function(Location akLoc, LocationRefType akRefType) native
/// ```
fn set_loc_ref_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_loc: Object,
    ak_ref_type: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetMotionType`
///
/// ```text
/// Function(int aeMotionType, bool abAllowActivate) native
/// ```
fn set_motion_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ae_motion_type: i32,
    ab_allow_activate: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetNoFavorAllowed`
///
/// ```text
/// Function(bool abNoFavor) native
/// ```
fn set_no_favor_allowed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_no_favor: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetOpen`
///
/// ```text
/// Function(bool abOpen) native
/// ```
fn set_open(caller: Caller<'_, State>, self_ptr: Object, ab_open: bool) -> Result<(), Exception> {
    todo!()
}

/// `SetPersistLoc`
///
/// ```text
/// Function(Location akLoc) native
/// ```
fn set_persist_loc(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_loc: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPlayerHasTaken`
///
/// ```text
/// Function(bool abTaken) native
/// ```
fn set_player_has_taken(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_taken: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPosition`
///
/// ```text
/// Function(float afX, float afY, float afZ) native
/// ```
fn set_position(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetRadioOn`
///
/// ```text
/// Function(bool abOn) native
/// ```
fn set_radio_on(caller: Caller<'_, State>, self_ptr: Object, ab_on: bool) -> Result<(), Exception> {
    todo!()
}

/// `SetRadioFrequency`
///
/// ```text
/// Function(float afFrequency) native
/// ```
fn set_radio_frequency(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_frequency: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetRadioVolume`
///
/// ```text
/// Function(float afVolume) native
/// ```
fn set_radio_volume(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_volume: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetScale`
///
/// ```text
/// Function(float afScale) native
/// ```
fn set_scale(caller: Caller<'_, State>, self_ptr: Object, af_scale: f32) -> Result<(), Exception> {
    todo!()
}

/// `SetValue`
///
/// ```text
/// Function(ActorValue akAV, float afValue) native
/// ```
fn set_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_av: Object,
    af_value: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `TranslateTo`
///
/// ```text
/// Function(float afX, float afY, float afZ, float afXAngle, float afYAngle, float afZAngle, float afSpeed, float afMaxRotationSpeed) native
/// ```
fn translate_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
    af_speed: f32,
    af_max_rotation_speed: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SplineTranslateTo`
///
/// ```text
/// Function(float afX, float afY, float afZ, float afXAngle, float afYAngle, float afZAngle, float afTangentMagnitude, float afSpeed, float afMaxRotationSpeed) native
/// ```
fn spline_translate_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
    af_tangent_magnitude: f32,
    af_speed: f32,
    af_max_rotation_speed: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SplineTranslateToRefNode`
///
/// ```text
/// Function(ObjectReference arTarget, string arNodeName, float afTangentMagnitude, float afSpeed, float afMaxRotationSpeed) native
/// ```
fn spline_translate_to_ref_node(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ar_target: Object,
    ar_node_name: isize,
    af_tangent_magnitude: f32,
    af_speed: f32,
    af_max_rotation_speed: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `StartWorkshop`
///
/// ```text
/// Function(bool abStart) native
/// ```
fn start_workshop(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_start: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `StopTranslation`
///
/// ```text
/// Function() native
/// ```
fn stop_translation(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `StoreInWorkshop`
///
/// ```text
/// Function(Form akBaseItem, int aiCount=1) native
/// ```
fn store_in_workshop(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_base_item: Object,
    ai_count_1: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `TetherToHorse`
///
/// ```text
/// Function(ObjectReference akHorse) native
/// ```
fn tether_to_horse(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_horse: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `WaitForAnimationEvent`
///
/// ```text
/// bool Function(string asEventName) native
/// ```
fn wait_for_animation_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_event_name: isize,
) -> Result<bool, Exception> {
    todo!()
}

/// `WaitFor3DLoad`
///
/// ```text
/// bool Function() native
/// ```
fn wait_for3_d_load(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `WaitForWorkshopResourceRecalc`
///
/// ```text
/// Function() native
/// ```
fn wait_for_workshop_resource_recalc(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `Scrap`
///
/// ```text
/// bool Function(ObjectReference akWorkshop) native
/// ```
fn scrap(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_workshop: Object,
) -> Result<bool, Exception> {
    todo!()
}

fn get_transform(caller: Caller<'_, State>, object: Object) -> Option<Transform> {
    let entity = caller.data().entity(object)?;
    let transform = caller.data().world().get::<&Transform>(entity).ok()?;
    Some(transform.to_nif())
}

#[derive(Clone, Debug)]
struct LinkedRefChainIter<'a> {
    state: &'a State,
    entity: Entity,
    keyword: Option<RecordId>,
    // Keep track of visited entities to avoid cycles.
    visited: HashSet<Entity>,
}

impl<'a> LinkedRefChainIter<'a> {
    fn new(caller: &'a Caller<'_, State>, self_ptr: Object, keyword: Object) -> Option<Self> {
        let state = caller.data();

        let keyword = if keyword.is_null() {
            None
        } else {
            Some(
                state
                    .objects
                    .get_by_addr(keyword)
                    .and_then(|s| s.record())?,
            )
        };

        let entity = state.entity(self_ptr)?;

        Some(Self {
            state,
            keyword,
            entity,
            visited: HashSet::new(),
        })
    }
}

impl<'a> Iterator for LinkedRefChainIter<'a> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        if self.visited.contains(&self.entity) {
            return None;
        }
        self.visited.insert(self.entity);

        let linked_refs = self.state.world().get::<&LinkedRefs>(self.entity).ok()?;
        let entity = linked_refs.get(self.keyword)?;

        let script = self.state.get_script_object(entity);
        let scripts = self.state.objects.get_by_id(script.id())?;
        let script = scripts.get("ObjectReference")?;
        let addr = script.get().object.addr();

        self.entity = entity;
        Some(addr)
    }
}
