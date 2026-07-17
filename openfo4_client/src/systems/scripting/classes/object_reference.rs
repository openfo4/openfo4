use game_common::components::Transform;
use hashbrown::HashSet;
use openfo4_animation::Value;
use openfo4_animation::driver::AnimationDriver;
use openfo4_core::ecs::Entity;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::components::{Disabled, ObjectBounds, ObjectLockLevel, ObjectProperties, TriggerBox};
use crate::state::npc::ObjectKeywords;
use crate::systems::actor_value::ActorValueProperties;
use crate::systems::inventory::Inventory;
use crate::systems::linked_refs::LinkedRefs;
use crate::systems::scripting::classes::{EngineExt, Object, State};
use crate::systems::scripting::effects::{
    AddItemEffect, DisableEntitiesEffect, EnableEntitiesEffect,
};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ObjectReference", "Activate", activate);
    engine.register_if_exists("ObjectReference", "AddDependentAnimatedObjectReference", add_dependent_animated_object_reference);
    engine.register_if_exists("ObjectReference", "AddItem", add_item);
    engine.register_if_exists("ObjectReference", "AddKeyword", add_keyword);
    engine.register_if_exists("ObjectReference", "AddTextReplacementData", add_text_replacement_data);
    engine.register_if_exists("ObjectReference", "AddToMap", add_to_map);
    engine.register_if_exists("ObjectReference", "ApplyConveyorBelt", apply_conveyor_belt);
    engine.register_if_exists("ObjectReference", "ApplyHavokImpulse", apply_havok_impulse);
    engine.register_if_exists("ObjectReference", "ApplyFanMotor", apply_fan_motor);
    engine.register_if_exists("ObjectReference", "FanMotorOn", fan_motor_on);
    engine.register_if_exists("ObjectReference", "IsFanMotorOn", is_fan_motor_on);
    engine.register_if_exists("ObjectReference", "AttachMod", attach_mod);
    engine.register_if_exists("ObjectReference", "AttachModToInventoryItem", attach_mod_to_inventory_item);
    engine.register_if_exists("ObjectReference", "AttachTo", attach_to);
    engine.register_if_exists("ObjectReference", "BlockActivation", block_activation);
    engine.register_if_exists("ObjectReference", "CalculateEncounterLevel", calculate_encounter_level);
    engine.register_if_exists("ObjectReference", "CanFastTravelToMarker", can_fast_travel_to_marker);
    engine.register_if_exists("ObjectReference", "CanProduceForWorkshop", can_produce_for_workshop);
    engine.register_if_exists("ObjectReference", "ClearDestruction", clear_destruction);
    engine.register_if_exists("ObjectReference", "ClearFromOldLocations", clear_from_old_locations);
    engine.register_if_exists("ObjectReference", "ConveyorBeltOn", conveyor_belt_on);
    engine.register_if_exists("ObjectReference", "CountActorsLinkedToMe", count_actors_linked_to_me);
    engine.register_if_exists("ObjectReference", "CountLinkedRefChain", count_linked_ref_chain);
    engine.register_if_exists("ObjectReference", "CountRefsLinkedToMe", count_refs_linked_to_me);
    engine.register_if_exists("ObjectReference", "CreateDetectionEvent", create_detection_event);
    engine.register_if_exists("ObjectReference", "DamageObject", damage_object);
    engine.register_if_exists("ObjectReference", "DamageValue", damage_value);
    engine.register_if_exists("ObjectReference", "Delete", delete);
    engine.register_if_exists("ObjectReference", "Disable", disable);
    engine.register_if_exists("ObjectReference", "DisableLinkChain", disable_link_chain);
    engine.register_if_exists("ObjectReference", "DisableNoWait", disable_no_wait);
    engine.register_if_exists("ObjectReference", "Drop", drop);
    engine.register_if_exists("ObjectReference", "DropFirstObject", drop_first_object);
    engine.register_if_exists("ObjectReference", "DropObject", drop_object);
    engine.register_if_exists("ObjectReference", "Enable", enable);
    engine.register_if_exists("ObjectReference", "EnableFastTravel", enable_fast_travel);
    engine.register_if_exists("ObjectReference", "EnableLinkChain", enable_link_chain);
    engine.register_if_exists("ObjectReference", "EnableNoWait", enable_no_wait);
    engine.register_if_exists("ObjectReference", "FindAllReferencesOfType", find_all_references_of_type);
    engine.register_if_exists("ObjectReference", "FindAllReferencesWithKeyword", find_all_references_with_keyword);
    engine.register_if_exists("ObjectReference", "ForceAddRagdollToWorld", force_add_ragdoll_to_world);
    engine.register_if_exists("ObjectReference", "ForceRemoveRagdollFromWorld", force_remove_ragdoll_from_world);
    engine.register_if_exists("ObjectReference", "GetActorOwner", get_actor_owner);
    engine.register_if_exists("ObjectReference", "GetActorRefOwner", get_actor_ref_owner);
    engine.register_if_exists("ObjectReference", "GetActorsLinkedToMe", get_actors_linked_to_me);
    engine.register_if_exists("ObjectReference", "GetAngleX", get_angle_x);
    engine.register_if_exists("ObjectReference", "GetAngleY", get_angle_y);
    engine.register_if_exists("ObjectReference", "GetAngleZ", get_angle_z);
    engine.register_if_exists("ObjectReference", "GetAnimationVariableBool", get_animation_variable_bool);
    engine.register_if_exists("ObjectReference", "GetAnimationVariableInt", get_animation_variable_int);
    engine.register_if_exists("ObjectReference", "GetAnimationVariableFloat", get_animation_variable_float);
    engine.register_if_exists("ObjectReference", "GetBaseObject", get_base_object);
    engine.register_if_exists("ObjectReference", "GetBaseValue", get_base_value);
    engine.register_if_exists("ObjectReference", "GetCurrentDestructionStage", get_current_destruction_stage);
    engine.register_if_exists("ObjectReference", "GetCurrentLocation", get_current_location);
    engine.register_if_exists("ObjectReference", "GetCurrentScene", get_current_scene);
    engine.register_if_exists("ObjectReference", "GetDistance", get_distance);
    engine.register_if_exists("ObjectReference", "GetEditorLocation", get_editor_location);
    engine.register_if_exists("ObjectReference", "GetEncounterZone", get_encounter_zone);
    engine.register_if_exists("ObjectReference", "GetFactionOwner", get_faction_owner);
    engine.register_if_exists("ObjectReference", "GetHeadingAngle", get_heading_angle);
    engine.register_if_exists("ObjectReference", "GetHeight", get_height);
    engine.register_if_exists("ObjectReference", "GetInventoryValue", get_inventory_value);
    engine.register_if_exists("ObjectReference", "GetItemCount", get_item_count);
    engine.register_if_exists("ObjectReference", "GetComponentCount", get_component_count);
    engine.register_if_exists("ObjectReference", "GetContainer", get_container);
    engine.register_if_exists("ObjectReference", "GetItemHealthPercent", get_item_health_percent);
    engine.register_if_exists("ObjectReference", "GetKey", get_key);
    engine.register_if_exists("ObjectReference", "GetLength", get_length);
    engine.register_if_exists("ObjectReference", "GetLinkedRef", get_linked_ref);
    engine.register_if_exists("ObjectReference", "GetLinkedRefChain", get_linked_ref_chain);
    engine.register_if_exists("ObjectReference", "GetLinkedRefChildren", get_linked_ref_children);
    engine.register_if_exists("ObjectReference", "GetLockLevel", get_lock_level);
    engine.register_if_exists("ObjectReference", "GetLocRefTypes", get_loc_ref_types);
    engine.register_if_exists("ObjectReference", "GetNthLinkedRef", get_nth_linked_ref);
    engine.register_if_exists("ObjectReference", "GetMass", get_mass);
    engine.register_if_exists("ObjectReference", "GetOpenState", get_open_state);
    engine.register_if_exists("ObjectReference", "GetParentCell", get_parent_cell);
    engine.register_if_exists("ObjectReference", "GetPositionX", get_position_x);
    engine.register_if_exists("ObjectReference", "GetPositionY", get_position_y);
    engine.register_if_exists("ObjectReference", "GetPositionZ", get_position_z);
    engine.register_if_exists("ObjectReference", "GetRadioFrequency", get_radio_frequency);
    engine.register_if_exists("ObjectReference", "GetRadioVolume", get_radio_volume);
    engine.register_if_exists("ObjectReference", "GetResourceDamage", get_resource_damage);
    engine.register_if_exists("ObjectReference", "GetSafePosition", get_safe_position);
    engine.register_if_exists("ObjectReference", "GetRefsLinkedToMe", get_refs_linked_to_me);
    engine.register_if_exists("ObjectReference", "GetWorkshopOwnedObjects", get_workshop_owned_objects);
    engine.register_if_exists("ObjectReference", "GetWorkshopResourceDamage", get_workshop_resource_damage);
    engine.register_if_exists("ObjectReference", "GetWorkshopResourceObjects", get_workshop_resource_objects);
    engine.register_if_exists("ObjectReference", "GetScale", get_scale);
    engine.register_if_exists("ObjectReference", "GetTeleportCell", get_teleport_cell);
    engine.register_if_exists("ObjectReference", "GetTransitionCell", get_transition_cell);
    engine.register_if_exists("ObjectReference", "GetTransmitterDistance", get_transmitter_distance);
    engine.register_if_exists("ObjectReference", "GetTriggerObjectCount", get_trigger_object_count);
    engine.register_if_exists("ObjectReference", "GetValue", get_value);
    engine.register_if_exists("ObjectReference", "GetValuePercentage", get_value_percentage);
    engine.register_if_exists("ObjectReference", "GetVoiceType", get_voice_type);
    engine.register_if_exists("ObjectReference", "GetWidth", get_width);
    engine.register_if_exists("ObjectReference", "GetWorldSpace", get_world_space);
    engine.register_if_exists("ObjectReference", "HasActorRefOwner", has_actor_ref_owner);
    engine.register_if_exists("ObjectReference", "HasDirectLOS", has_direct_los);
    engine.register_if_exists("ObjectReference", "HasEffectKeyword", has_effect_keyword);
    engine.register_if_exists("ObjectReference", "HasKeyword", has_keyword);
    engine.register_if_exists("ObjectReference", "HasKeywordInFormList", has_keyword_in_form_list);
    engine.register_if_exists("ObjectReference", "HasLocRefType", has_loc_ref_type);
    engine.register_if_exists("ObjectReference", "HasNode", has_node);
    engine.register_if_exists("ObjectReference", "HasSharedPowerGrid", has_shared_power_grid);
    engine.register_if_exists("ObjectReference", "IgnoreFriendlyHits", ignore_friendly_hits);
    engine.register_if_exists("ObjectReference", "InterruptCast", interrupt_cast);
    engine.register_if_exists("ObjectReference", "IsActivateChild", is_activate_child);
    engine.register_if_exists("ObjectReference", "IsActivationBlocked", is_activation_blocked);
    engine.register_if_exists("ObjectReference", "Is3DLoaded", is3_d_loaded);
    engine.register_if_exists("ObjectReference", "IsConveyorBeltOn", is_conveyor_belt_on);
    engine.register_if_exists("ObjectReference", "IsCreated", is_created);
    engine.register_if_exists("ObjectReference", "IsDeleted", is_deleted);
    engine.register_if_exists("ObjectReference", "IsDestroyed", is_destroyed);
    engine.register_if_exists("ObjectReference", "IsDisabled", is_disabled);
    engine.register_if_exists("ObjectReference", "IsFurnitureInUse", is_furniture_in_use);
    engine.register_if_exists("ObjectReference", "IsFurnitureMarkerInUse", is_furniture_marker_in_use);
    engine.register_if_exists("ObjectReference", "IsIgnoringFriendlyHits", is_ignoring_friendly_hits);
    engine.register_if_exists("ObjectReference", "IsInDialogueWithPlayer", is_in_dialogue_with_player);
    engine.register_if_exists("ObjectReference", "IsLockBroken", is_lock_broken);
    engine.register_if_exists("ObjectReference", "IsLocked", is_locked);
    engine.register_if_exists("ObjectReference", "IsMapMarkerVisible", is_map_marker_visible);
    engine.register_if_exists("ObjectReference", "IsOwnedBy", is_owned_by);
    engine.register_if_exists("ObjectReference", "IsPowered", is_powered);
    engine.register_if_exists("ObjectReference", "IsQuestItem", is_quest_item);
    engine.register_if_exists("ObjectReference", "IsRadioOn", is_radio_on);
    engine.register_if_exists("ObjectReference", "IsRefInTransitionCell", is_ref_in_transition_cell);
    engine.register_if_exists("ObjectReference", "IsTeleportAreaLoaded", is_teleport_area_loaded);
    engine.register_if_exists("ObjectReference", "IsWithinBuildableArea", is_within_buildable_area);
    engine.register_if_exists("ObjectReference", "KnockAreaEffect", knock_area_effect);
    engine.register_if_exists("ObjectReference", "Lock", lock);
    engine.register_if_exists("ObjectReference", "MakeRadioReceiver", make_radio_receiver);
    engine.register_if_exists("ObjectReference", "MakeTransmitterRepeater", make_transmitter_repeater);
    engine.register_if_exists("ObjectReference", "ModValue", mod_value);
    engine.register_if_exists("ObjectReference", "MoveTo", move_to);
    engine.register_if_exists("ObjectReference", "MoveToMyEditorLocation", move_to_my_editor_location);
    engine.register_if_exists("ObjectReference", "MoveToNearestNavmeshLocation", move_to_nearest_navmesh_location);
    engine.register_if_exists("ObjectReference", "MoveToNode", move_to_node);
    engine.register_if_exists("ObjectReference", "OpenWorkshopSettlementMenu", open_workshop_settlement_menu);
    engine.register_if_exists("ObjectReference", "OpenWorkshopSettlementMenuEx", open_workshop_settlement_menu_ex);
    engine.register_if_exists("ObjectReference", "RaidTargetsAvailable", raid_targets_available);
    engine.register_if_exists("ObjectReference", "PlaceAtMe", place_at_me);
    engine.register_if_exists("ObjectReference", "PlaceActorAtMe", place_actor_at_me);
    engine.register_if_exists("ObjectReference", "PlaceAtNode", place_at_node);
    engine.register_if_exists("ObjectReference", "PlayAnimation", play_animation);
    engine.register_if_exists("ObjectReference", "PlayAnimationAndWait", play_animation_and_wait);
    engine.register_if_exists("ObjectReference", "PlayGamebryoAnimation", play_gamebryo_animation);
    engine.register_if_exists("ObjectReference", "PlayImpactEffect", play_impact_effect);
    engine.register_if_exists("ObjectReference", "PlaySyncedAnimationSS", play_synced_animation_ss);
    engine.register_if_exists("ObjectReference", "PlaySyncedAnimationAndWaitSS", play_synced_animation_and_wait_ss);
    engine.register_if_exists("ObjectReference", "PlayTerrainEffect", play_terrain_effect);
    engine.register_if_exists("ObjectReference", "PauseAudio", pause_audio);
    engine.register_if_exists("ObjectReference", "PreloadExteriorCell", preload_exterior_cell);
    engine.register_if_exists("ObjectReference", "PreloadTargetArea", preload_target_area);
    engine.register_if_exists("ObjectReference", "ProcessTrapHit", process_trap_hit);
    engine.register_if_exists("ObjectReference", "PushActorAway", push_actor_away);
    engine.register_if_exists("ObjectReference", "RecalculateResources", recalculate_resources);
    engine.register_if_exists("ObjectReference", "RemoveAllItems", remove_all_items);
    engine.register_if_exists("ObjectReference", "RemoveComponents", remove_components);
    engine.register_if_exists("ObjectReference", "RemoveItem", remove_item);
    engine.register_if_exists("ObjectReference", "RemoveItemByComponent", remove_item_by_component);
    engine.register_if_exists("ObjectReference", "RemoveKeyword", remove_keyword);
    engine.register_if_exists("ObjectReference", "RemoveAllMods", remove_all_mods);
    engine.register_if_exists("ObjectReference", "RemoveAllModsFromInventoryItem", remove_all_mods_from_inventory_item);
    engine.register_if_exists("ObjectReference", "RemoveMod", remove_mod);
    engine.register_if_exists("ObjectReference", "RemoveModFromInventoryItem", remove_mod_from_inventory_item);
    engine.register_if_exists("ObjectReference", "RemoveDependentAnimatedObjectReference", remove_dependent_animated_object_reference);
    engine.register_if_exists("ObjectReference", "Repair", repair);
    engine.register_if_exists("ObjectReference", "Reset", reset);
    engine.register_if_exists("ObjectReference", "ResetKeyword", reset_keyword);
    engine.register_if_exists("ObjectReference", "RestoreValue", restore_value);
    engine.register_if_exists("ObjectReference", "ResumeAudio", resume_audio);
    engine.register_if_exists("ObjectReference", "ReverseConveyorBelt", reverse_conveyor_belt);
    engine.register_if_exists("ObjectReference", "Say", say);
    engine.register_if_exists("ObjectReference", "SayCustom", say_custom);
    engine.register_if_exists("ObjectReference", "SendStealAlarm", send_steal_alarm);
    engine.register_if_exists("ObjectReference", "SetActivateTextOverride", set_activate_text_override);
    engine.register_if_exists("ObjectReference", "SetActorCause", set_actor_cause);
    engine.register_if_exists("ObjectReference", "SetActorOwner", set_actor_owner);
    engine.register_if_exists("ObjectReference", "SetActorRefOwner", set_actor_ref_owner);
    engine.register_if_exists("ObjectReference", "SetAngle", set_angle);
    engine.register_if_exists("ObjectReference", "SetAnimationVariableBool", set_animation_variable_bool);
    engine.register_if_exists("ObjectReference", "SetAnimationVariableInt", set_animation_variable_int);
    engine.register_if_exists("ObjectReference", "SetAnimationVariableFloat", set_animation_variable_float);
    engine.register_if_exists("ObjectReference", "SetAttractionActive", set_attraction_active);
    engine.register_if_exists("ObjectReference", "SetConveyorBeltVelocity", set_conveyor_belt_velocity);
    engine.register_if_exists("ObjectReference", "SetDestroyed", set_destroyed);
    engine.register_if_exists("ObjectReference", "SetDirectAtTarget", set_direct_at_target);
    engine.register_if_exists("ObjectReference", "SetFactionOwner", set_faction_owner);
    engine.register_if_exists("ObjectReference", "SetHarvested", set_harvested);
    engine.register_if_exists("ObjectReference", "SetLinkedRef", set_linked_ref);
    engine.register_if_exists("ObjectReference", "SetLockLevel", set_lock_level);
    engine.register_if_exists("ObjectReference", "SetLocRefType", set_loc_ref_type);
    engine.register_if_exists("ObjectReference", "SetMotionType", set_motion_type);
    engine.register_if_exists("ObjectReference", "SetNoFavorAllowed", set_no_favor_allowed);
    engine.register_if_exists("ObjectReference", "SetOpen", set_open);
    engine.register_if_exists("ObjectReference", "SetPersistLoc", set_persist_loc);
    engine.register_if_exists("ObjectReference", "SetPlayerHasTaken", set_player_has_taken);
    engine.register_if_exists("ObjectReference", "SetPosition", set_position);
    engine.register_if_exists("ObjectReference", "SetRadioOn", set_radio_on);
    engine.register_if_exists("ObjectReference", "SetRadioFrequency", set_radio_frequency);
    engine.register_if_exists("ObjectReference", "SetRadioVolume", set_radio_volume);
    engine.register_if_exists("ObjectReference", "SetScale", set_scale);
    engine.register_if_exists("ObjectReference", "SetValue", set_value);
    engine.register_if_exists("ObjectReference", "TranslateTo", translate_to);
    engine.register_if_exists("ObjectReference", "SplineTranslateTo", spline_translate_to);
    engine.register_if_exists("ObjectReference", "SplineTranslateToRefNode", spline_translate_to_ref_node);
    engine.register_if_exists("ObjectReference", "StartWorkshop", start_workshop);
    engine.register_if_exists("ObjectReference", "StopTranslation", stop_translation);
    engine.register_if_exists("ObjectReference", "StoreInWorkshop", store_in_workshop);
    engine.register_if_exists("ObjectReference", "TetherToHorse", tether_to_horse);
    engine.register_if_exists("ObjectReference", "WaitForAnimationEvent", wait_for_animation_event);
    engine.register_if_exists("ObjectReference", "WaitFor3DLoad", wait_for3_d_load);
    engine.register_if_exists("ObjectReference", "WaitForWorkshopResourceRecalc", wait_for_workshop_resource_recalc);
    engine.register_if_exists("ObjectReference", "Scrap", scrap);
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
) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `AddItem`
///
/// ```text
/// Function(Form akItemToAdd, int aiCount, bool abSilent) native
/// ```
async fn add_item(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    count: i32,
    silent: bool,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;

    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .map(|s| s.record())
        .flatten()
        .ok_or_else(|| todo!())?;

    // Ignore negative item counts.
    let Ok(count) = u64::try_from(count) else {
        return Ok(());
    };

    caller
        .data_mut()
        .run_effect_and_wait(AddItemEffect {
            entity,
            item,
            count,
        })
        .await?;

    Ok(())
}

/// `AddKeyword`
///
/// ```text
/// Function(Keyword apKeyword) native
/// ```
async fn add_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let mut keywords = world.get::<&mut ObjectKeywords>(entity).unwrap();
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
    as_token_label: Object,
    ak_form: Object,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
    a_target: Object,
    a_lin_vel_x: f32,
    a_lin_vel_y: f32,
    a_lin_vel_z: f32,
    ab_on: bool,
    ab_reverse: bool,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
    a_target: Object,
    a_axis_x: f32,
    a_axis_y: f32,
    a_axis_z: f32,
    a_force: f32,
    ab_on: bool,
) -> Result<(), Error> {
    todo!()
}

/// `FanMotorOn`
///
/// ```text
/// Function(bool abOn) native
/// ```
fn fan_motor_on(caller: Caller<'_, State>, self_ptr: Object, ab_on: bool) -> Result<(), Error> {
    todo!()
}

/// `IsFanMotorOn`
///
/// ```text
/// bool Function() native
/// ```
fn is_fan_motor_on(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `AttachTo`
///
/// ```text
/// Function(ObjectReference akParent) native
/// ```
fn attach_to(caller: Caller<'_, State>, self_ptr: Object, ak_parent: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<i32, Error> {
    todo!()
}

/// `CanFastTravelToMarker`
///
/// ```text
/// bool Function() native
/// ```
fn can_fast_travel_to_marker(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `CanProduceForWorkshop`
///
/// ```text
/// bool Function() native
/// ```
fn can_produce_for_workshop(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `ClearDestruction`
///
/// ```text
/// Function() native
/// ```
fn clear_destruction(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `ClearFromOldLocations`
///
/// ```text
/// Function() native
/// ```
fn clear_from_old_locations(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `ConveyorBeltOn`
///
/// ```text
/// Function(bool abOn) native
/// ```
fn conveyor_belt_on(caller: Caller<'_, State>, self_ptr: Object, ab_on: bool) -> Result<(), Error> {
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
) -> Result<i32, Error> {
    todo!()
}

/// `CountLinkedRefChain`
///
/// ```text
/// int Function(keyword apKeyword, int maxExpectedLinkedRefs) native
/// ```
async fn count_linked_ref_chain(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
    max_expected_linked_refs: i32,
) -> Result<i32, Error> {
    let Ok(max_count) = usize::try_from(max_expected_linked_refs) else {
        return Ok(0);
    };

    let mut iter = LinkedRefChainIter::new(&caller, self_ptr, keyword)
        .await
        .ok_or_else(|| todo!())?;

    let mut count = 0;
    while let Some(_) = iter.next().await
        && count != max_count
    {
        count += 1;
    }

    Ok(count.try_into().unwrap_or(i32::MAX))
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
) -> Result<i32, Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `DamageObject`
///
/// ```text
/// Function(float afDamage) native
/// ```
fn damage_object(caller: Caller<'_, State>, self_ptr: Object, af_damage: f32) -> Result<(), Error> {
    todo!()
}

/// `DamageValue`
///
/// ```text
/// Function(ActorValue akAV, float afDamage) native
/// ```
async fn damage_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    damage: f32,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let actor_value = caller
        .data()
        .objects
        .get_by_addr(actor_value)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(mut props) = world.get::<&mut ActorValueProperties>(entity) else {
        return Ok(());
    };

    let Some(mut value) = props.get(actor_value) else {
        return Ok(());
    };

    // It looks like damage is clamped at 0.
    value.damaged = f32::clamp(value.damaged - damage, 0.0, value.value);

    props.set(actor_value, value);
    Ok(())
}

/// `Delete`
///
/// ```text
/// Function() native
/// ```
fn delete(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Disable`
///
/// ```text
/// Function(bool abFadeOut) native
/// ```
async fn disable(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    fade_out: bool,
) -> Result<(), Error> {
    let entity = caller.data().entity(self_ptr).await.unwrap();
    caller
        .data_mut()
        .run_effect_and_wait(DisableEntitiesEffect {
            entities: vec![entity],
            fade: fade_out,
            wait_until_faded: true,
        })
        .await?;
    Ok(())
}

/// `DisableLinkChain`
///
/// ```text
/// Function(Keyword apKeyword, bool abFadeOut) native
/// ```
async fn disable_link_chain(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
    fade_out: bool,
) -> Result<(), Error> {
    let Some(mut objects) = LinkedRefChainIter::new(&caller, self_ptr, keyword).await else {
        return Ok(());
    };

    let mut entities = Vec::new();
    while let Some(object) = objects.next().await {
        let entity = caller.data().entity(object).await.unwrap();
        entities.push(entity);
    }

    caller
        .data_mut()
        .run_effect_and_wait(DisableEntitiesEffect {
            entities,
            fade: fade_out,
            wait_until_faded: false,
        })
        .await?;
    Ok(())
}

/// `DisableNoWait`
///
/// ```text
/// Function(bool abFadeOut) native
/// ```
async fn disable_no_wait(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    fade_out: bool,
) -> Result<(), Error> {
    let entity = caller.data().entity(self_ptr).await.unwrap();
    caller
        .data_mut()
        .run_effect_and_wait(DisableEntitiesEffect {
            entities: vec![entity],
            fade: fade_out,
            wait_until_faded: false,
        })
        .await?;
    Ok(())
}

/// `Drop`
///
/// ```text
/// Function(bool abSilent) native
/// ```
fn drop(caller: Caller<'_, State>, self_ptr: Object, ab_silent: bool) -> Result<(), Error> {
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
    todo!()
}

/// `Enable`
///
/// ```text
/// Function(bool abFadeIn) native
/// ```
async fn enable(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    fade_in: bool,
) -> Result<(), Error> {
    let entity = caller.data().entity(self_ptr).await.unwrap();
    caller
        .data_mut()
        .run_effect_and_wait(EnableEntitiesEffect {
            entities: vec![entity],
            fade: fade_in,
            wait_until_faded: true,
        })
        .await?;
    Ok(())
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
) -> Result<(), Error> {
    todo!()
}

/// `EnableLinkChain`
///
/// ```text
/// Function(Keyword apKeyword, bool abFadeIn) native
/// ```
async fn enable_link_chain(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
    fade_in: bool,
) -> Result<(), Error> {
    let Some(mut objects) = LinkedRefChainIter::new(&caller, self_ptr, keyword).await else {
        return Ok(());
    };

    let mut entities = Vec::new();
    while let Some(object) = objects.next().await {
        let entity = caller.data().entity(object).await.unwrap();
        entities.push(entity);
    }

    caller
        .data_mut()
        .run_effect_and_wait(EnableEntitiesEffect {
            entities,
            fade: fade_in,
            wait_until_faded: false,
        })
        .await?;

    Ok(())
}

/// `EnableNoWait`
///
/// ```text
/// Function(bool abFadeIn) native
/// ```
async fn enable_no_wait(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    fade_in: bool,
) -> Result<(), Error> {
    let entity = caller.data().entity(self_ptr).await.unwrap();
    caller
        .data_mut()
        .run_effect_and_wait(EnableEntitiesEffect {
            entities: vec![entity],
            fade: fade_in,
            wait_until_faded: false,
        })
        .await?;
    Ok(())
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
    todo!()
}

/// `ForceAddRagdollToWorld`
///
/// ```text
/// Function() native
/// ```
fn force_add_ragdoll_to_world(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `GetActorOwner`
///
/// ```text
/// ActorBase Function() native
/// ```
fn get_actor_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetActorRefOwner`
///
/// ```text
/// Actor Function() native
/// ```
fn get_actor_ref_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
    todo!()
}

/// `GetAngleX`
///
/// ```text
/// float Function() native
/// ```
fn get_angle_x(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetAngleY`
///
/// ```text
/// float Function() native
/// ```
fn get_angle_y(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetAngleZ`
///
/// ```text
/// float Function() native
/// ```
fn get_angle_z(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetAnimationVariableBool`
///
/// ```text
/// bool Function(string arVariableName) native
/// ```
async fn get_animation_variable_bool(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
) -> Result<bool, Error> {
    let name = caller.string(variable_name);

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(driver) = world.get::<&AnimationDriver>(entity) else {
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
async fn get_animation_variable_int(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
) -> Result<i32, Error> {
    let name = caller.string(variable_name);

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(driver) = world.get::<&AnimationDriver>(entity) else {
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
async fn get_animation_variable_float(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
) -> Result<f32, Error> {
    let name = caller.string(variable_name);

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(driver) = world.get::<&AnimationDriver>(entity) else {
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
fn get_base_object(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
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
) -> Result<f32, Error> {
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
) -> Result<i32, Error> {
    todo!()
}

/// `GetCurrentLocation`
///
/// ```text
/// Location Function() native
/// ```
fn get_current_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetCurrentScene`
///
/// ```text
/// Scene Function() native
/// ```
fn get_current_scene(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetDistance`
///
/// ```text
/// float Function(ObjectReference akOther) native
/// ```
async fn get_distance(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<f32, Error> {
    let this = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let other = caller.data().entity(other).await.ok_or_else(|| todo!())?;

    let world = caller.data().world().await;

    // Entities are guaranteed to exist, otherwise `entity()` would return `None`.
    let this_transform = world.get::<&Transform>(this).unwrap();
    let other_transform = world.get::<&Transform>(other).unwrap();

    let a = this_transform.to_nif();
    let b = other_transform.to_nif();

    Ok(a.translation.distance(b.translation))
}

/// `GetEditorLocation`
///
/// ```text
/// Location Function() native
/// ```
fn get_editor_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetEncounterZone`
///
/// ```text
/// EncounterZone Function() native
/// ```
fn get_encounter_zone(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetFactionOwner`
///
/// ```text
/// Faction Function() native
/// ```
fn get_faction_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
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
) -> Result<f32, Error> {
    todo!()
}

/// `GetHeight`
///
/// ```text
/// float Function() native
/// ```
async fn get_height(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let bounds = get_bounds(caller, self_ptr).await.ok_or_else(|| todo!())?;
    Ok(bounds.min.z.abs_diff(bounds.max.z) as f32)
}

/// `GetInventoryValue`
///
/// ```text
/// int Function() native
/// ```
fn get_inventory_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetItemCount`
///
/// ```text
/// int Function(Form akItem) native
/// ```
async fn get_item_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
) -> Result<i32, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(inventory) = world.get::<&Inventory>(entity) else {
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
            .ok_or_else(|| todo!())?;
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
) -> Result<i32, Error> {
    todo!()
}

/// `GetContainer`
///
/// ```text
/// ObjectReference Function() native
/// ```
fn get_container(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetItemHealthPercent`
///
/// ```text
/// float Function() native
/// ```
fn get_item_health_percent(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetKey`
///
/// ```text
/// Key Function() native
/// ```
async fn get_key(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;

    let key = {
        let world = caller.data().world().await;
        let Ok(props) = world.get::<&ObjectProperties>(entity) else {
            return Ok(Object::NULL);
        };

        let Some(key) = props.lock.and_then(|lock| lock.key) else {
            return Ok(Object::NULL);
        };

        key
    };

    let scripts = caller.data_mut().get_or_create_record(key).await?;
    let script = scripts.get("Key").ok_or_else(|| todo!())?;

    Ok(script.get().object.addr())
}

/// `GetLength`
///
/// ```text
/// float Function() native
/// ```
async fn get_length(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let bounds = get_bounds(caller, self_ptr).await.ok_or_else(|| todo!())?;
    Ok(bounds.min.y.abs_diff(bounds.max.y) as f32)
}

/// `GetLinkedRef`
///
/// ```text
/// ObjectReference Function(Keyword apKeyword) native
/// ```
async fn get_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Object, Error> {
    let mut iter = LinkedRefChainIter::new(&caller, self_ptr, keyword)
        .await
        .ok_or_else(|| todo!())?;
    let addr = iter.next().await.unwrap_or(Object::NULL);
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
    todo!()
}

/// `GetLockLevel`
///
/// ```text
/// int Function() native
/// ```
async fn get_lock_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(props) = world.get::<&ObjectProperties>(entity) else {
        return Ok(0);
    };

    let Some(level) = props.lock.and_then(|lock| lock.level) else {
        return Ok(0);
    };

    match level {
        ObjectLockLevel::Noivce => Ok(25),
        ObjectLockLevel::Advanced => Ok(50),
        ObjectLockLevel::Expert => Ok(75),
        ObjectLockLevel::Master => Ok(100),
        ObjectLockLevel::Barred => Ok(251),
        ObjectLockLevel::Chained => Ok(252),
        ObjectLockLevel::RequiresTerminal => Ok(253),
        ObjectLockLevel::Inaccessible => Ok(254),
        ObjectLockLevel::RequiresKey => Ok(255),
    }
}

/// `GetLocRefTypes`
///
/// ```text
/// LocationRefType[] Function() native
/// ```
fn get_loc_ref_types(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetNthLinkedRef`
///
/// ```text
/// ObjectReference Function(int aiLinkedRef, Keyword apKeyword) native
/// ```
async fn get_nth_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    index: i32,
    keyword: Object,
) -> Result<Object, Error> {
    let index = usize::try_from(index).map_err(|_| todo!())?;

    let mut iter = LinkedRefChainIter::new(&caller, self_ptr, keyword)
        .await
        .ok_or_else(|| todo!())?;

    let mut i = 0;
    while let Some(object) = iter.next().await {
        if i == index {
            return Ok(object);
        }

        i += 1;
    }

    Ok(Object::NULL)
}

/// `GetMass`
///
/// ```text
/// float Function() native
/// ```
fn get_mass(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetOpenState`
///
/// ```text
/// int Function() native
/// ```
fn get_open_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetParentCell`
///
/// ```text
/// Cell Function() native
/// ```
fn get_parent_cell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetPositionX`
///
/// ```text
/// float Function() native
/// ```
async fn get_position_x(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let transform = get_transform(caller, self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    Ok(transform.translation.x)
}

/// `GetPositionY`
///
/// ```text
/// float Function() native
/// ```
async fn get_position_y(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let transform = get_transform(caller, self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    Ok(transform.translation.y)
}

/// `GetPositionZ`
///
/// ```text
/// float Function() native
/// ```
async fn get_position_z(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let transform = get_transform(caller, self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    Ok(transform.translation.z)
}

/// `GetRadioFrequency`
///
/// ```text
/// float Function() native
/// ```
fn get_radio_frequency(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetRadioVolume`
///
/// ```text
/// float Function() native
/// ```
fn get_radio_volume(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
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
) -> Result<f32, Error> {
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
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
) -> Result<f32, Error> {
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
) -> Result<Object, Error> {
    todo!()
}

/// `GetScale`
///
/// ```text
/// float Function() native
/// ```
async fn get_scale(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let transform = get_transform(caller, self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    // Assume we have uniform scale.
    Ok(transform.scale.x)
}

/// `GetTeleportCell`
///
/// ```text
/// Cell Function() native
/// ```
fn get_teleport_cell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetTransitionCell`
///
/// ```text
/// Cell Function() native
/// ```
fn get_transition_cell(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetTransmitterDistance`
///
/// ```text
/// float Function() native
/// ```
fn get_transmitter_distance(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetTriggerObjectCount`
///
/// ```text
/// int Function() native
/// ```
async fn get_trigger_object_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let tbox = world.get::<&TriggerBox>(entity).map_err(|_| todo!())?;

    let count = tbox.entities_inside.len().try_into().unwrap_or(i32::MAX);
    Ok(count)
}

/// `GetValue`
///
/// ```text
/// float Function(ActorValue akAV) native
/// ```
async fn get_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
) -> Result<f32, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let actor_value = caller
        .data()
        .objects
        .get_by_addr(actor_value)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(props) = world.get::<&ActorValueProperties>(entity) else {
        // Returns 0.0 when the actor value is not present.
        return Ok(0.0);
    };

    let Some(value) = props.get(actor_value) else {
        // Returns 0.0 when the actor value is not present.
        return Ok(0.0);
    };

    // `GetValue` returns the damaged value, not the "full" value.
    Ok(value.damaged())
}

/// `GetValuePercentage`
///
/// ```text
/// float Function(ActorValue akAV) native
/// ```
async fn get_value_percentage(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
) -> Result<f32, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let actor_value = caller
        .data()
        .objects
        .get_by_addr(actor_value)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(props) = world.get::<&ActorValueProperties>(entity) else {
        return Ok(0.0);
    };

    let Some(value) = props.get(actor_value) else {
        return Ok(0.0);
    };

    let max = value.value;
    let cur = value.damaged();
    let percentage = cur / max;

    if percentage.is_nan() {
        // Probably divide by zero.
        Ok(1.0)
    } else {
        // Clamp just in case.
        Ok(f32::clamp(percentage, 0.0, 1.0))
    }
}

/// `GetVoiceType`
///
/// ```text
/// VoiceType Function() native
/// ```
fn get_voice_type(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetWidth`
///
/// ```text
/// float Function() native
/// ```
async fn get_width(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Error> {
    let bounds = get_bounds(caller, self_ptr).await.ok_or_else(|| todo!())?;
    Ok(bounds.min.x.abs_diff(bounds.max.x) as f32)
}

/// `GetWorldSpace`
///
/// ```text
/// WorldSpace Function() native
/// ```
fn get_world_space(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `HasActorRefOwner`
///
/// ```text
/// bool Function() native
/// ```
fn has_actor_ref_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
    as_source_node: Object,
    as_target_node: Object,
) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `HasKeyword`
///
/// ```text
/// bool Function(Keyword apKeyword) native
/// ```
async fn has_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<bool, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let keywords = world.get::<&ObjectKeywords>(entity).unwrap();
    Ok(keywords.contains(keyword))
}

/// `HasKeywordInFormList`
///
/// ```text
/// bool Function(FormList akKeywordList) native
/// ```
async fn has_keyword_in_form_list(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword_list: Object,
) -> Result<bool, Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let form_list = caller
        .data()
        .objects
        .get_by_addr(keyword_list)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let keywords = {
        let world = caller.data().world().await;
        let keywords = world.get::<&ObjectKeywords>(entity).unwrap();
        // FIXME: Avoid the clone here.
        keywords.clone()
    };

    let globals = caller.data().globals().await;
    let form_list = globals.form_lists.get(&form_list).ok_or_else(|| todo!())?;

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
) -> Result<bool, Error> {
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
    as_node_name: Object,
) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `InterruptCast`
///
/// ```text
/// Function() native
/// ```
fn interrupt_cast(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `IsActivationBlocked`
///
/// ```text
/// bool Function() native
/// ```
fn is_activation_blocked(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `Is3DLoaded`
///
/// ```text
/// bool Function() native
/// ```
fn is3_d_loaded(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsConveyorBeltOn`
///
/// ```text
/// bool Function() native
/// ```
fn is_conveyor_belt_on(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsCreated`
///
/// ```text
/// bool Function() native
/// ```
fn is_created(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    let script = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .ok_or_else(|| todo!())?;

    // Dynamically created objects have no RecordId.
    Ok(script.record().is_none())
}

/// `IsDeleted`
///
/// ```text
/// bool Function() native
/// ```
fn is_deleted(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsDestroyed`
///
/// ```text
/// bool Function() native
/// ```
fn is_destroyed(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsDisabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_disabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    let entity = caller.data().entity(self_ptr).await.unwrap();
    let world = caller.data().world().await;
    let entity = world.entity(entity).unwrap();
    Ok(entity.has::<Disabled>())
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
) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `IsIgnoringFriendlyHits`
///
/// ```text
/// bool Function() native
/// ```
fn is_ignoring_friendly_hits(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsInDialogueWithPlayer`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_dialogue_with_player(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsLockBroken`
///
/// ```text
/// bool Function() native
/// ```
fn is_lock_broken(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsLocked`
///
/// ```text
/// bool Function() native
/// ```
fn is_locked(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsMapMarkerVisible`
///
/// ```text
/// bool Function() native
/// ```
fn is_map_marker_visible(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `IsPowered`
///
/// ```text
/// bool Function() native
/// ```
fn is_powered(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsQuestItem`
///
/// ```text
/// bool Function() native
/// ```
fn is_quest_item(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsRadioOn`
///
/// ```text
/// bool Function() native
/// ```
fn is_radio_on(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `IsTeleportAreaLoaded`
///
/// ```text
/// bool Function() native
/// ```
fn is_teleport_area_loaded(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
) -> Result<bool, Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `ModValue`
///
/// ```text
/// Function(ActorValue akAV, float afAmount) native
/// ```
async fn mod_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    amount: f32,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let actor_value = caller
        .data()
        .objects
        .get_by_addr(actor_value)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(mut props) = world.get::<&mut ActorValueProperties>(entity) else {
        return Ok(());
    };

    // The current damage value is retained.
    let mut value = props.get(actor_value).unwrap_or_default();
    value.value += amount;
    value.damaged += amount;

    props.set(actor_value, value);
    Ok(())
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
) -> Result<(), Error> {
    todo!()
}

/// `MoveToMyEditorLocation`
///
/// ```text
/// Function() native
/// ```
fn move_to_my_editor_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
    as_node_name: Object,
    as_match_node_name: Object,
) -> Result<(), Error> {
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
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
) -> Result<i32, Error> {
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
) -> Result<Object, Error> {
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
) -> Result<Object, Error> {
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
    as_node_name: Object,
    ak_form_to_place: Object,
    ai_count: i32,
    ab_force_persist: bool,
    ab_initially_disabled: bool,
    ab_delete_when_able: bool,
    ab_attach: bool,
) -> Result<Object, Error> {
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
    as_animation: Object,
) -> Result<bool, Error> {
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
    as_animation: Object,
    as_event_name: Object,
) -> Result<bool, Error> {
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
    as_animation: Object,
    ab_start_over: bool,
    af_ease_in_time: f32,
) -> Result<bool, Error> {
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
    as_node_name: Object,
    af_pick_dir_x: f32,
    af_pick_dir_y: f32,
    af_pick_dir_z: f32,
    af_pick_length: f32,
    ab_apply_node_rotation: bool,
    ab_use_node_local_rotation: bool,
) -> Result<bool, Error> {
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
    as_animation1: Object,
    ak_obj2: Object,
    as_animation2: Object,
) -> Result<bool, Error> {
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
    as_animation1: Object,
    as_event1: Object,
    ak_obj2: Object,
    as_animation2: Object,
    as_event2: Object,
) -> Result<bool, Error> {
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
    as_effect_model_name: Object,
    as_attach_bone_name: Object,
) -> Result<(), Error> {
    todo!()
}

/// `PauseAudio`
///
/// ```text
/// Function() native
/// ```
fn pause_audio(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `PreloadExteriorCell`
///
/// ```text
/// Function() native
/// ```
fn preload_exterior_cell(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    Ok(())
}

/// `PreloadTargetArea`
///
/// ```text
/// Function() native
/// ```
fn preload_target_area(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `RecalculateResources`
///
/// ```text
/// Function() native
/// ```
fn recalculate_resources(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    Ok(())
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `RemoveKeyword`
///
/// ```text
/// Function(Keyword apKeyword) native
/// ```
async fn remove_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let mut keywords = world.get::<&mut ObjectKeywords>(entity).unwrap();
    keywords.remove(keyword);

    Ok(())
}

/// `RemoveAllMods`
///
/// ```text
/// Function() native
/// ```
fn remove_all_mods(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `RemoveMod`
///
/// ```text
/// Function(ObjectMod akMod) native
/// ```
fn remove_mod(caller: Caller<'_, State>, self_ptr: Object, ak_mod: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `Repair`
///
/// ```text
/// Function() native
/// ```
fn repair(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function(ObjectReference akTarget) native
/// ```
fn reset(caller: Caller<'_, State>, self_ptr: Object, ak_target: Object) -> Result<(), Error> {
    todo!()
}

/// `ResetKeyword`
///
/// ```text
/// Function(Keyword apKeyword) native
/// ```
async fn reset_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let mut keywords = world.get::<&mut ObjectKeywords>(entity).unwrap();
    keywords.reset(keyword);

    Ok(())
}

/// `RestoreValue`
///
/// ```text
/// Function(ActorValue akAV, float afAmount) native
/// ```
async fn restore_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    amount: f32,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let actor_value = caller
        .data()
        .objects
        .get_by_addr(actor_value)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(mut props) = world.get::<&mut ActorValueProperties>(entity) else {
        return Ok(());
    };

    let Some(mut value) = props.get(actor_value) else {
        return Ok(());
    };

    value.damaged = f32::clamp(value.damaged + amount, 0.0, value.value);

    props.set(actor_value, value);
    Ok(())
}

/// `ResumeAudio`
///
/// ```text
/// Function() native
/// ```
fn resume_audio(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `SetAnimationVariableBool`
///
/// ```text
/// Function(string arVariableName, bool abNewValue) native
/// ```
async fn set_animation_variable_bool(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
    new_value: bool,
) -> Result<(), Error> {
    let variable_name = caller.string(variable_name);

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(mut driver) = world.get::<&mut AnimationDriver>(entity) else {
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
async fn set_animation_variable_int(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
    new_value: i32,
) -> Result<(), Error> {
    let variable_name = caller.string(variable_name);

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(mut driver) = world.get::<&mut AnimationDriver>(entity) else {
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
async fn set_animation_variable_float(
    caller: Caller<'_, State>,
    self_ptr: Object,
    variable_name: Object,
    new_value: f32,
) -> Result<(), Error> {
    let variable_name = caller.string(variable_name);

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(mut driver) = world.get::<&mut AnimationDriver>(entity) else {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `SetLinkedRef`
///
/// ```text
/// Function(ObjectReference akLinkedRef, Keyword apKeyword) native
/// ```
async fn set_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    linked_ref: Object,
    keyword: Object,
) -> Result<(), Error> {
    let keyword = (!keyword.is_null())
        .then(|| {
            caller
                .data()
                .objects
                .get_by_addr(keyword)
                .and_then(|s| s.record())
        })
        .ok_or_else(|| todo!())?;

    let linked_ref_entity = caller.data().entity(linked_ref).await;

    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let mut linked_refs = world.get::<&mut LinkedRefs>(entity).unwrap();

    if linked_ref.is_null() {
        linked_refs.remove(keyword);
    } else {
        let linked_ref = linked_ref_entity.ok_or_else(|| todo!())?;
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `SetOpen`
///
/// ```text
/// Function(bool abOpen) native
/// ```
fn set_open(caller: Caller<'_, State>, self_ptr: Object, ab_open: bool) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `SetRadioOn`
///
/// ```text
/// Function(bool abOn) native
/// ```
fn set_radio_on(caller: Caller<'_, State>, self_ptr: Object, ab_on: bool) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `SetScale`
///
/// ```text
/// Function(float afScale) native
/// ```
fn set_scale(caller: Caller<'_, State>, self_ptr: Object, af_scale: f32) -> Result<(), Error> {
    todo!()
}

/// `SetValue`
///
/// ```text
/// Function(ActorValue akAV, float afValue) native
/// ```
async fn set_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    new_value: f32,
) -> Result<(), Error> {
    let entity = caller
        .data()
        .entity(self_ptr)
        .await
        .ok_or_else(|| todo!())?;
    let actor_value = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(mut props) = world.get::<&mut ActorValueProperties>(entity) else {
        return Ok(());
    };

    // The current damage value is retained.
    let mut value = props.get(actor_value).unwrap_or_default();
    let damage = value.damage();

    value.value = new_value;
    value.damaged = new_value - damage;

    props.set(actor_value, value);
    Ok(())
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
    ar_node_name: Object,
    af_tangent_magnitude: f32,
    af_speed: f32,
    af_max_rotation_speed: f32,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `StopTranslation`
///
/// ```text
/// Function() native
/// ```
fn stop_translation(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
    as_event_name: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `WaitFor3DLoad`
///
/// ```text
/// bool Function() native
/// ```
fn wait_for3_d_load(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
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
) -> Result<(), Error> {
    todo!()
}

/// `Scrap`
///
/// ```text
/// bool Function(ObjectReference akWorkshop) native
/// ```
fn scrap(caller: Caller<'_, State>, self_ptr: Object, ak_workshop: Object) -> Result<bool, Error> {
    todo!()
}

async fn get_transform(caller: Caller<'_, State>, object: Object) -> Option<Transform> {
    let entity = caller.data().entity(object).await?;
    let world = caller.data().world().await;
    let transform = world.get::<&Transform>(entity).ok()?;
    Some(transform.to_nif())
}

async fn get_bounds(caller: Caller<'_, State>, object: Object) -> Option<ObjectBounds> {
    let entity = caller.data().entity(object).await?;
    let world = caller.data().world().await;
    let bounds = world.get::<&ObjectBounds>(entity).ok()?;
    Some(*bounds)
}

struct LinkedRefChainIter<'a> {
    state: &'a State,
    entity: Entity,
    keyword: Option<RecordId>,
    // Keep track of visited entities to avoid cycles.
    visited: HashSet<Entity>,
}

impl<'a> LinkedRefChainIter<'a> {
    async fn new(caller: &'a Caller<'_, State>, self_ptr: Object, keyword: Object) -> Option<Self> {
        let state = caller.data();

        let entity = state.entity(self_ptr).await?;

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

        Some(Self {
            state,
            keyword,
            entity,
            visited: HashSet::new(),
        })
    }

    async fn next(&mut self) -> Option<Object> {
        if self.visited.contains(&self.entity) {
            return None;
        }
        self.visited.insert(self.entity);

        let entity = {
            let world = self.state.world().await;
            let linked_refs = world.get::<&LinkedRefs>(self.entity).ok()?;
            let entity = linked_refs.get(self.keyword)?;

            entity
        };

        let scripts = match self.state.get_or_create_entity(entity).await {
            Ok(scripts) => scripts,
            Err(err) => {
                todo!("{}", err);
            }
        };

        let script = scripts.get("ObjectReference")?;
        let addr = script.get().object.addr();

        self.entity = entity;
        Some(addr)
    }
}
