use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::components::{ActorProperties, ActorProtection, Player};
use crate::state::hardcoded::fallout4::GOLD;
use crate::systems::inventory::{Inventory, ItemData, WeaponItemData};
use crate::systems::scripting::classes::{Object, State};
use crate::systems::scripting::effects::{EquipItemEffect, UnequipItemEffect};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Actor", "AddPerk", add_perk);
    engine.register_native_function("Actor", "AddSpell", add_spell);
    engine.register_native_function("Actor", "AllowBleedoutDialogue", allow_bleedout_dialogue);
    engine.register_native_function("Actor", "AllowPCDialogue", allow_pc_dialogue);
    engine.register_native_function("Actor", "AttachAshPile", attach_ash_pile);
    engine.register_native_function("Actor", "AttemptAnimationSetSwitch", attempt_animation_set_switch);
    engine.register_native_function("Actor", "CanFlyHere", can_fly_here);
    engine.register_native_function("Actor", "ChangeAnimArchetype", change_anim_archetype);
    engine.register_native_function("Actor", "ChangeAnimFlavor", change_anim_flavor);
    engine.register_native_function("Actor", "ChangeHeadPart", change_head_part);
    engine.register_native_function("Actor", "ClearArrested", clear_arrested);
    engine.register_native_function("Actor", "ClearExpressionOverride", clear_expression_override);
    engine.register_native_function("Actor", "ClearExtraArrows", clear_extra_arrows);
    engine.register_native_function("Actor", "ClearLookAt", clear_look_at);
    engine.register_native_function("Actor", "Dismount", dismount);
    engine.register_native_function("Actor", "DispelAllSpells", dispel_all_spells);
    engine.register_native_function("Actor", "DispelSpell", dispel_spell);
    engine.register_native_function("Actor", "DoCombatSpellApply", do_combat_spell_apply);
    engine.register_native_function("Actor", "EnableAI", enable_ai);
    engine.register_native_function("Actor", "EndDeferredKill", end_deferred_kill);
    engine.register_native_function("Actor", "EquipItem", equip_item);
    engine.register_native_function("Actor", "EquipSpell", equip_spell);
    engine.register_native_function("Actor", "EvaluatePackage", evaluate_package);
    engine.register_native_function("Actor", "GetBribeAmount", get_bribe_amount);
    engine.register_native_function("Actor", "GetAllCombatTargets", get_all_combat_targets);
    engine.register_native_function("Actor", "GetCrimeFaction", get_crime_faction);
    engine.register_native_function("Actor", "GetCombatState", get_combat_state);
    engine.register_native_function("Actor", "GetCombatTarget", get_combat_target);
    engine.register_native_function("Actor", "GetCurrentPackage", get_current_package);
    engine.register_native_function("Actor", "GetDialogueTarget", get_dialogue_target);
    engine.register_native_function("Actor", "GetEquippedItemType", get_equipped_item_type);
    engine.register_native_function("Actor", "GetEquippedWeapon", get_equipped_weapon);
    engine.register_native_function("Actor", "GetEquippedShield", get_equipped_shield);
    engine.register_native_function("Actor", "GetEquippedSpell", get_equipped_spell);
    engine.register_native_function("Actor", "GetFactionRank", get_faction_rank);
    engine.register_native_function("Actor", "GetFactionReaction", get_faction_reaction);
    engine.register_native_function("Actor", "GetFlyingState", get_flying_state);
    engine.register_native_function("Actor", "GetForcedLandingMarker", get_forced_landing_marker);
    engine.register_native_function("Actor", "GetGoldAmount", get_gold_amount);
    engine.register_native_function("Actor", "GetHighestRelationshipRank", get_highest_relationship_rank);
    engine.register_native_function("Actor", "GetKiller", get_killer);
    engine.register_native_function("Actor", "GetLevel", get_level);
    engine.register_native_function("Actor", "GetLightLevel", get_light_level);
    engine.register_native_function("Actor", "GetLowestRelationshipRank", get_lowest_relationship_rank);
    engine.register_native_function("Actor", "GetLeveledActorBase", get_leveled_actor_base);
    engine.register_native_function("Actor", "GetNoBleedoutRecovery", get_no_bleedout_recovery);
    engine.register_native_function("Actor", "GetPlayerControls", get_player_controls);
    engine.register_native_function("Actor", "GetRace", get_race);
    engine.register_native_function("Actor", "GetRelationshipRank", get_relationship_rank);
    engine.register_native_function("Actor", "GetSitState", get_sit_state);
    engine.register_native_function("Actor", "GetSleepState", get_sleep_state);
    engine.register_native_function("Actor", "HasAssociation", has_association);
    engine.register_native_function("Actor", "HasFamilyRelationship", has_family_relationship);
    engine.register_native_function("Actor", "HasDetectionLOS", has_detection_los);
    engine.register_native_function("Actor", "HasMagicEffect", has_magic_effect);
    engine.register_native_function("Actor", "HasMagicEffectWithKeyword", has_magic_effect_with_keyword);
    engine.register_native_function("Actor", "HasParentRelationship", has_parent_relationship);
    engine.register_native_function("Actor", "HasPerk", has_perk);
    engine.register_native_function("Actor", "HasSpell", has_spell);
    engine.register_native_function("Actor", "IsAIEnabled", is_ai_enabled);
    engine.register_native_function("Actor", "IsAlarmed", is_alarmed);
    engine.register_native_function("Actor", "IsAlerted", is_alerted);
    engine.register_native_function("Actor", "IsAllowedToFly", is_allowed_to_fly);
    engine.register_native_function("Actor", "IsArrested", is_arrested);
    engine.register_native_function("Actor", "IsArrestingTarget", is_arresting_target);
    engine.register_native_function("Actor", "IsBeingRidden", is_being_ridden);
    engine.register_native_function("Actor", "IsBeingRiddenBy", is_being_ridden_by);
    engine.register_native_function("Actor", "IsBleedingOut", is_bleeding_out);
    engine.register_native_function("Actor", "IsBribed", is_bribed);
    engine.register_native_function("Actor", "IsChild", is_child);
    engine.register_native_function("Actor", "IsCommandedActor", is_commanded_actor);
    engine.register_native_function("Actor", "IsDead", is_dead);
    engine.register_native_function("Actor", "IsDetectedBy", is_detected_by);
    engine.register_native_function("Actor", "IsDismembered", is_dismembered);
    engine.register_native_function("Actor", "IsDoingFavor", is_doing_favor);
    engine.register_native_function("Actor", "IsEquipped", is_equipped);
    engine.register_native_function("Actor", "IsEssential", is_essential);
    engine.register_native_function("Actor", "IsFlying", is_flying);
    engine.register_native_function("Actor", "IsGuard", is_guard);
    engine.register_native_function("Actor", "IsGhost", is_ghost);
    engine.register_native_function("Actor", "IsHostileToActor", is_hostile_to_actor);
    engine.register_native_function("Actor", "IsInCombat", is_in_combat);
    engine.register_native_function("Actor", "IsInFaction", is_in_faction);
    engine.register_native_function("Actor", "IsInIronSights", is_in_iron_sights);
    engine.register_native_function("Actor", "IsInKillMove", is_in_kill_move);
    engine.register_native_function("Actor", "IsInScene", is_in_scene);
    engine.register_native_function("Actor", "IsIntimidated", is_intimidated);
    engine.register_native_function("Actor", "IsOnMount", is_on_mount);
    engine.register_native_function("Actor", "IsOverEncumbered", is_over_encumbered);
    engine.register_native_function("Actor", "IsPlayersLastRiddenHorse", is_players_last_ridden_horse);
    engine.register_native_function("Actor", "IsPlayerTeammate", is_player_teammate);
    engine.register_native_function("Actor", "IsRunning", is_running);
    engine.register_native_function("Actor", "IsSeatOccupied", is_seat_occupied);
    engine.register_native_function("Actor", "IsSneaking", is_sneaking);
    engine.register_native_function("Actor", "IsSprinting", is_sprinting);
    engine.register_native_function("Actor", "IsTalking", is_talking);
    engine.register_native_function("Actor", "IsTrespassing", is_trespassing);
    engine.register_native_function("Actor", "IsUnconscious", is_unconscious);
    engine.register_native_function("Actor", "IsWeaponDrawn", is_weapon_drawn);
    engine.register_native_function("Actor", "Kill", kill);
    engine.register_native_function("Actor", "KillSilent", kill_silent);
    engine.register_native_function("Actor", "Dismember", dismember);
    engine.register_native_function("Actor", "MarkItemAsFavorite", mark_item_as_favorite);
    engine.register_native_function("Actor", "ModFactionRank", mod_faction_rank);
    engine.register_native_function("Actor", "MoveToPackageLocation", move_to_package_location);
    engine.register_native_function("Actor", "OpenInventory", open_inventory);
    engine.register_native_function("Actor", "PathToReference", path_to_reference);
    engine.register_native_function("Actor", "PlayIdle", play_idle);
    engine.register_native_function("Actor", "PlayIdleAction", play_idle_action);
    engine.register_native_function("Actor", "PlayIdleWithTarget", play_idle_with_target);
    engine.register_native_function("Actor", "PlaySubGraphAnimation", play_sub_graph_animation);
    engine.register_native_function("Actor", "RemoveFromFaction", remove_from_faction);
    engine.register_native_function("Actor", "RemoveFromAllFactions", remove_from_all_factions);
    engine.register_native_function("Actor", "RemovePerk", remove_perk);
    engine.register_native_function("Actor", "RemoveSpell", remove_spell);
    engine.register_native_function("Actor", "ResetHealthAndLimbs", reset_health_and_limbs);
    engine.register_native_function("Actor", "Resurrect", resurrect);
    engine.register_native_function("Actor", "SendAssaultAlarm", send_assault_alarm);
    engine.register_native_function("Actor", "SendTrespassAlarm", send_trespass_alarm);
    engine.register_native_function("Actor", "SetAlert", set_alert);
    engine.register_native_function("Actor", "SetAllowFlying", set_allow_flying);
    engine.register_native_function("Actor", "SetAlpha", set_alpha);
    engine.register_native_function("Actor", "SetAttackActorOnSight", set_attack_actor_on_sight);
    engine.register_native_function("Actor", "SetAvoidPlayer", set_avoid_player);
    engine.register_native_function("Actor", "SetCommandState", set_command_state);
    engine.register_native_function("Actor", "SetBribed", set_bribed);
    engine.register_native_function("Actor", "SetCanDoCommand", set_can_do_command);
    engine.register_native_function("Actor", "SetCombatStyle", set_combat_style);
    engine.register_native_function("Actor", "SetCrimeFaction", set_crime_faction);
    engine.register_native_function("Actor", "SetCriticalStage", set_critical_stage);
    engine.register_native_function("Actor", "SetDoingFavor", set_doing_favor);
    engine.register_native_function("Actor", "ChangeAnimFaceArchetype", change_anim_face_archetype);
    engine.register_native_function("Actor", "SetEyeTexture", set_eye_texture);
    engine.register_native_function("Actor", "SetEssential", set_essential);
    engine.register_native_function("Actor", "SetFactionRank", set_faction_rank);
    engine.register_native_function("Actor", "SetForcedLandingMarker", set_forced_landing_marker);
    engine.register_native_function("Actor", "SetGhost", set_ghost);
    engine.register_native_function("Actor", "SetHasCharGenSkeleton", set_has_char_gen_skeleton);
    engine.register_native_function("Actor", "SetHeadTracking", set_head_tracking);
    engine.register_native_function("Actor", "SetIntimidated", set_intimidated);
    engine.register_native_function("Actor", "SetLookAt", set_look_at);
    engine.register_native_function("Actor", "SetNoBleedoutRecovery", set_no_bleedout_recovery);
    engine.register_native_function("Actor", "SetNotShowOnStealthMeter", set_not_show_on_stealth_meter);
    engine.register_native_function("Actor", "SetOutfit", set_outfit);
    engine.register_native_function("Actor", "SetOverrideVoiceType", set_override_voice_type);
    engine.register_native_function("Actor", "SetPlayerControls", set_player_controls);
    engine.register_native_function("Actor", "SetPlayerResistingArrest", set_player_resisting_arrest);
    engine.register_native_function("Actor", "SetPlayerTeammate", set_player_teammate);
    engine.register_native_function("Actor", "SetProtected", set_protected);
    engine.register_native_function("Actor", "SetRace", set_race);
    engine.register_native_function("Actor", "SetRelationshipRank", set_relationship_rank);
    engine.register_native_function("Actor", "SetRestrained", set_restrained);
    engine.register_native_function("Actor", "SetSubGraphFloatVariable", set_sub_graph_float_variable);
    engine.register_native_function("Actor", "SetUnconscious", set_unconscious);
    engine.register_native_function("Actor", "SetVehicle", set_vehicle);
    engine.register_native_function("Actor", "ShowBarterMenu", show_barter_menu);
    engine.register_native_function("Actor", "StartCannibal", start_cannibal);
    engine.register_native_function("Actor", "StartCombat", start_combat);
    engine.register_native_function("Actor", "StartDeferredKill", start_deferred_kill);
    engine.register_native_function("Actor", "StartVampireFeed", start_vampire_feed);
    engine.register_native_function("Actor", "StartFrenzyAttack", start_frenzy_attack);
    engine.register_native_function("Actor", "StopCombat", stop_combat);
    engine.register_native_function("Actor", "StopCombatAlarm", stop_combat_alarm);
    engine.register_native_function("Actor", "SwitchToPowerArmor", switch_to_power_armor);
    engine.register_native_function("Actor", "SnapIntoInteraction", snap_into_interaction);
    engine.register_native_function("Actor", "TrapSoul", trap_soul);
    engine.register_native_function("Actor", "UnequipAll", unequip_all);
    engine.register_native_function("Actor", "UnequipItem", unequip_item);
    engine.register_native_function("Actor", "UnequipItemSlot", unequip_item_slot);
    engine.register_native_function("Actor", "UnequipSpell", unequip_spell);
    engine.register_native_function("Actor", "UnLockOwnedDoorsInCell", un_lock_owned_doors_in_cell);
    engine.register_native_function("Actor", "WillIntimidateSucceed", will_intimidate_succeed);
    engine.register_native_function("Actor", "WornHasKeyword", worn_has_keyword);
    engine.register_native_function("Actor", "WouldBeStealing", would_be_stealing);
    engine.register_native_function("Actor", "WouldRefuseCommand", would_refuse_command);
    engine.register_native_function("Actor", "StartSneaking", start_sneaking);
    engine.register_native_function("Actor", "DrawWeapon", draw_weapon);
    engine.register_native_function("Actor", "DogPlaceInMouth", dog_place_in_mouth);
    engine.register_native_function("Actor", "DogDropItems", dog_drop_items);
    engine.register_native_function("Actor", "ForceMovementDirection", force_movement_direction);
    engine.register_native_function("Actor", "ForceMovementSpeed", force_movement_speed);
    engine.register_native_function("Actor", "ForceMovementRotationSpeed", force_movement_rotation_speed);
    engine.register_native_function("Actor", "ForceMovementDirectionRamp", force_movement_direction_ramp);
    engine.register_native_function("Actor", "ForceMovementSpeedRamp", force_movement_speed_ramp);
    engine.register_native_function("Actor", "ForceMovementRotationSpeedRamp", force_movement_rotation_speed_ramp);
    engine.register_native_function("Actor", "ForceTargetDirection", force_target_direction);
    engine.register_native_function("Actor", "ForceTargetSpeed", force_target_speed);
    engine.register_native_function("Actor", "ForceTargetAngle", force_target_angle);
    engine.register_native_function("Actor", "ClearForcedMovement", clear_forced_movement);
    engine.register_native_function("Actor", "CanMoveVertical", can_move_vertical);
    engine.register_native_function("Actor", "CanStrafe", can_strafe);
    engine.register_native_function("Actor", "GetWornItem", get_worn_item);
    engine.register_native_function("Actor", "GetWornItemMods", get_worn_item_mods);
    engine.register_native_function("Actor", "QueueUpdate", queue_update);
    engine.register_native_function("Actor", "GetFurnitureReference", get_furniture_reference);
    engine.register_native_function("Actor", "IsProtected", is_protected);
}

/// `AddPerk`
///
/// ```text
/// Function(Perk akPerk, bool abNotify=false) native
/// ```
fn add_perk(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_perk: Object,
    ab_notify_false: bool,
) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr);

    // todo!()
    Ok(())
}

/// `AddSpell`
///
/// ```text
/// bool Function(Spell akSpell, bool abVerbose=true) native
/// ```
fn add_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_spell: Object,
    ab_verbose_true: bool,
) -> Result<bool, Exception> {
    // todo!()
    Ok(true)
}

/// `AllowBleedoutDialogue`
///
/// ```text
/// Function(bool abCanTalk) native
/// ```
fn allow_bleedout_dialogue(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_can_talk: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `AllowPCDialogue`
///
/// ```text
/// Function(bool abTalk) native
/// ```
fn allow_pc_dialogue(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_talk: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `AttachAshPile`
///
/// ```text
/// Function(Form akAshPileBase) native
/// ```
fn attach_ash_pile(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ash_pile_base: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `AttemptAnimationSetSwitch`
///
/// ```text
/// Function() native
/// ```
fn attempt_animation_set_switch(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `CanFlyHere`
///
/// ```text
/// bool Function() native
/// ```
fn can_fly_here(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `ChangeAnimArchetype`
///
/// ```text
/// bool Function(keyword apKeyword) native
/// ```
fn change_anim_archetype(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `ChangeAnimFlavor`
///
/// ```text
/// bool Function(keyword apKeyword) native
/// ```
fn change_anim_flavor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `ChangeHeadPart`
///
/// ```text
/// Function(headpart apHeadPart, bool abRemovePart, bool abRemoveExtraParts) native
/// ```
fn change_head_part(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_head_part: Object,
    ab_remove_part: bool,
    ab_remove_extra_parts: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `ClearArrested`
///
/// ```text
/// Function() native
/// ```
fn clear_arrested(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ClearExpressionOverride`
///
/// ```text
/// Function() native
/// ```
fn clear_expression_override(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ClearExtraArrows`
///
/// ```text
/// Function() native
/// ```
fn clear_extra_arrows(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ClearLookAt`
///
/// ```text
/// Function() native
/// ```
fn clear_look_at(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Dismount`
///
/// ```text
/// bool Function() native
/// ```
fn dismount(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `DispelAllSpells`
///
/// ```text
/// Function() native
/// ```
fn dispel_all_spells(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DispelSpell`
///
/// ```text
/// bool Function(Spell akSpell) native
/// ```
fn dispel_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_spell: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `DoCombatSpellApply`
///
/// ```text
/// Function(Spell akSpell, ObjectReference akTarget) native
/// ```
fn do_combat_spell_apply(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_spell: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableAI`
///
/// ```text
/// Function(bool abEnable, bool abPauseVoice) native
/// ```
fn enable_ai(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
    ab_pause_voice: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EndDeferredKill`
///
/// ```text
/// Function() native
/// ```
fn end_deferred_kill(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `EquipItem`
///
/// ```text
/// Function(Form akItem, bool abPreventRemoval, bool abSilent) native
/// ```
fn equip_item(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    prevent_removal: bool,
    silent: bool,
) -> Result<(), Exception> {
    let actor = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    caller
        .data_mut()
        .run_effect_and_wait(EquipItemEffect { actor, item })?;

    Ok(())
}

/// `EquipSpell`
///
/// ```text
/// Function(Spell akSpell, int aiSource) native
/// ```
fn equip_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_spell: Object,
    ai_source: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `EvaluatePackage`
///
/// ```text
/// Function(bool abResetAI) native
/// ```
fn evaluate_package(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_reset_ai: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `GetBribeAmount`
///
/// ```text
/// int Function() native
/// ```
fn get_bribe_amount(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetAllCombatTargets`
///
/// ```text
/// Actor[] Function() native
/// ```
fn get_all_combat_targets(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetCrimeFaction`
///
/// ```text
/// Faction Function() native
/// ```
fn get_crime_faction(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCombatState`
///
/// ```text
/// int Function() native
/// ```
fn get_combat_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetCombatTarget`
///
/// ```text
/// Actor Function() native
/// ```
fn get_combat_target(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCurrentPackage`
///
/// ```text
/// Package Function() native
/// ```
fn get_current_package(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetDialogueTarget`
///
/// ```text
/// Actor Function() native
/// ```
fn get_dialogue_target(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetEquippedItemType`
///
/// ```text
/// int Function(int aiEquipIndex) native
/// ```
fn get_equipped_item_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_equip_index: i32,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetEquippedWeapon`
///
/// ```text
/// Weapon Function(int aiEquipIndex) native
/// ```
fn get_equipped_weapon(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_equip_index: i32,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetEquippedShield`
///
/// ```text
/// Armor Function() native
/// ```
fn get_equipped_shield(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetEquippedSpell`
///
/// ```text
/// Spell Function(int aiSource) native
/// ```
fn get_equipped_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_source: i32,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetFactionRank`
///
/// ```text
/// int Function(Faction akFaction) native
/// ```
fn get_faction_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<i32, Exception> {
    let Some(faction) = caller
        .data()
        .objects
        .get_by_addr(faction)
        .and_then(|s| s.record())
    else {
        debug_assert!(faction.is_null(),);

        // Returns -2 if actor is not in faction.
        return Ok(-2);
    };

    let entity = caller.data().entity(self_ptr).unwrap();
    let props = caller
        .data()
        .world()
        .get::<&ActorProperties>(entity)
        .unwrap();

    match props.factions.iter().find(|f| f.faction == faction) {
        Some(faction) => Ok(faction.rank.into()),
        // Returns -2 if actor is not in faction.
        None => Ok(-2),
    }
}

/// `GetFactionReaction`
///
/// ```text
/// int Function(Actor akOther) native
/// ```
fn get_faction_reaction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetFlyingState`
///
/// ```text
/// int Function() native
/// ```
fn get_flying_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetForcedLandingMarker`
///
/// ```text
/// ObjectReference Function() native
/// ```
fn get_forced_landing_marker(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetGoldAmount`
///
/// ```text
/// int Function() native
/// ```
fn get_gold_amount(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    with_inventory(caller, self_ptr, |inventory| {
        let mut sum: i32 = 0;
        for (_, item) in inventory.get_by_form(GOLD) {
            sum = sum.saturating_add(item.count.try_into().unwrap_or(i32::MAX));
        }

        sum
    })
}

/// `GetHighestRelationshipRank`
///
/// ```text
/// int Function() native
/// ```
fn get_highest_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetKiller`
///
/// ```text
/// Actor Function() native
/// ```
fn get_killer(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetLevel`
///
/// ```text
/// int Function() native
/// ```
fn get_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;

    let Ok(props) = caller.data().world().get::<&ActorProperties>(entity) else {
        return Ok(0);
    };

    Ok(props.level.try_into().unwrap_or(i32::MAX))
}

/// `GetLightLevel`
///
/// ```text
/// float Function() native
/// ```
fn get_light_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<f32, Exception> {
    todo!()
}

/// `GetLowestRelationshipRank`
///
/// ```text
/// int Function() native
/// ```
fn get_lowest_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetLeveledActorBase`
///
/// ```text
/// ActorBase Function() native
/// ```
fn get_leveled_actor_base(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetNoBleedoutRecovery`
///
/// ```text
/// bool Function() native
/// ```
fn get_no_bleedout_recovery(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(props) = caller.data().world().get::<&ActorProperties>(entity) else {
        return Ok(false);
    };

    // Note that this function returns whether bleedout recovery is disabled.
    Ok(!props.bleedout_recovery)
}

/// `GetPlayerControls`
///
/// ```text
/// bool Function() native
/// ```
fn get_player_controls(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let is_player = caller
        .data()
        .world()
        .entity(entity)
        .is_ok_and(|e| e.has::<Player>());
    Ok(is_player)
}

/// `GetRace`
///
/// ```text
/// Race Function() native
/// ```
fn get_race(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetRelationshipRank`
///
/// ```text
/// int Function(Actor akOther) native
/// ```
fn get_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetSitState`
///
/// ```text
/// int Function() native
/// ```
fn get_sit_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetSleepState`
///
/// ```text
/// int Function() native
/// ```
fn get_sleep_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `HasAssociation`
///
/// ```text
/// bool Function(AssociationType akAssociation, Actor akOther) native
/// ```
fn has_association(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_association: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasFamilyRelationship`
///
/// ```text
/// bool Function(Actor akOther) native
/// ```
fn has_family_relationship(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasDetectionLOS`
///
/// ```text
/// bool Function(ObjectReference akOther) native
/// ```
fn has_detection_los(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasMagicEffect`
///
/// ```text
/// bool Function(MagicEffect akEffect) native
/// ```
fn has_magic_effect(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_effect: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasMagicEffectWithKeyword`
///
/// ```text
/// bool Function(Keyword akKeyword) native
/// ```
fn has_magic_effect_with_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasParentRelationship`
///
/// ```text
/// bool Function(Actor akOther) native
/// ```
fn has_parent_relationship(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasPerk`
///
/// ```text
/// bool Function(Perk akPerk) native
/// ```
fn has_perk(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_perk: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasSpell`
///
/// ```text
/// bool Function(Form akForm) native
/// ```
fn has_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_form: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsAIEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_ai_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsAlarmed`
///
/// ```text
/// bool Function() native
/// ```
fn is_alarmed(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsAlerted`
///
/// ```text
/// bool Function() native
/// ```
fn is_alerted(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsAllowedToFly`
///
/// ```text
/// bool Function() native
/// ```
fn is_allowed_to_fly(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsArrested`
///
/// ```text
/// bool Function() native
/// ```
fn is_arrested(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsArrestingTarget`
///
/// ```text
/// bool Function() native
/// ```
fn is_arresting_target(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsBeingRidden`
///
/// ```text
/// bool Function() native
/// ```
fn is_being_ridden(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsBeingRiddenBy`
///
/// ```text
/// bool Function(Actor akActor) native
/// ```
fn is_being_ridden_by(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsBleedingOut`
///
/// ```text
/// bool Function() native
/// ```
fn is_bleeding_out(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsBribed`
///
/// ```text
/// bool Function() native
/// ```
fn is_bribed(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsChild`
///
/// ```text
/// bool Function() native
/// ```
fn is_child(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    with_actor_props(caller, self_ptr, |actor| actor.is_child)
}

/// `IsCommandedActor`
///
/// ```text
/// bool Function() native
/// ```
fn is_commanded_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsDead`
///
/// ```text
/// bool Function() native
/// ```
fn is_dead(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    with_actor_props(caller, self_ptr, |actor| actor.is_dead)
}

/// `IsDetectedBy`
///
/// ```text
/// bool Function(Actor akOther) native
/// ```
fn is_detected_by(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsDismembered`
///
/// ```text
/// bool Function(string asBodyPart) native
/// ```
fn is_dismembered(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_body_part: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsDoingFavor`
///
/// ```text
/// bool Function() native
/// ```
fn is_doing_favor(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsEquipped`
///
/// ```text
/// bool Function(Form akItem) native
/// ```
fn is_equipped(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
) -> Result<bool, Exception> {
    let actor = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let Ok(inventory) = caller.data().world().get::<&Inventory>(actor) else {
        return Ok(false);
    };

    for (_, stack) in inventory.get_by_form(item) {
        if stack.item == item && stack.equipped {
            return Ok(true);
        }
    }

    Ok(false)
}

/// `IsEssential`
///
/// ```text
/// bool Function() native
/// ```
fn is_essential(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    with_actor_props(caller, self_ptr, |actor| {
        actor.protection == ActorProtection::Essential
    })
}

/// `IsFlying`
///
/// ```text
/// bool Function() native
/// ```
fn is_flying(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsGuard`
///
/// ```text
/// bool Function() native
/// ```
fn is_guard(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsGhost`
///
/// ```text
/// bool Function() native
/// ```
fn is_ghost(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsHostileToActor`
///
/// ```text
/// bool Function(Actor akActor) native
/// ```
fn is_hostile_to_actor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsInCombat`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_combat(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInFaction`
///
/// ```text
/// bool Function(Faction akFaction) native
/// ```
fn is_in_faction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<bool, Exception> {
    let Some(faction) = caller
        .data()
        .objects
        .get_by_addr(faction)
        .and_then(|s| s.record())
    else {
        debug_assert!(faction.is_null());
        return Ok(false);
    };

    let entity = caller.data().entity(self_ptr).unwrap();
    let props = caller
        .data()
        .world()
        .get::<&ActorProperties>(entity)
        .unwrap();

    Ok(props
        .factions
        .iter()
        .find(|f| f.faction == faction)
        .is_some())
}

/// `IsInIronSights`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_iron_sights(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInKillMove`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_kill_move(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInScene`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_scene(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsIntimidated`
///
/// ```text
/// bool Function() native
/// ```
fn is_intimidated(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsOnMount`
///
/// ```text
/// bool Function() native
/// ```
fn is_on_mount(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsOverEncumbered`
///
/// ```text
/// bool Function() native
/// ```
fn is_over_encumbered(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsPlayersLastRiddenHorse`
///
/// ```text
/// bool Function() native
/// ```
fn is_players_last_ridden_horse(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsPlayerTeammate`
///
/// ```text
/// bool Function() native
/// ```
fn is_player_teammate(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsRunning`
///
/// ```text
/// bool Function() native
/// ```
fn is_running(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsSeatOccupied`
///
/// ```text
/// bool Function(keyword apKeyword) native
/// ```
fn is_seat_occupied(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsSneaking`
///
/// ```text
/// bool Function() native
/// ```
fn is_sneaking(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsSprinting`
///
/// ```text
/// bool Function() native
/// ```
fn is_sprinting(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsTalking`
///
/// ```text
/// bool Function() native
/// ```
fn is_talking(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsTrespassing`
///
/// ```text
/// bool Function() native
/// ```
fn is_trespassing(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsUnconscious`
///
/// ```text
/// bool Function() native
/// ```
fn is_unconscious(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsWeaponDrawn`
///
/// ```text
/// bool Function() native
/// ```
fn is_weapon_drawn(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    with_inventory(caller, self_ptr, |inventory| {
        inventory.items().any(|(_, item)| {
            item.equipped
                && matches!(
                    &item.data,
                    ItemData::Weapon(WeaponItemData { drawn: true, .. }),
                )
        })
    })
}

/// `Kill`
///
/// ```text
/// Function(Actor akKiller) native
/// ```
fn kill(caller: Caller<'_, State>, self_ptr: Object, ak_killer: Object) -> Result<(), Exception> {
    todo!()
}

/// `KillSilent`
///
/// ```text
/// Function(Actor akKiller) native
/// ```
fn kill_silent(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_killer: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `Dismember`
///
/// ```text
/// Function(string asBodyPart, bool abForceExplode, bool abForceDismember, bool abForceBloodyMess) native
/// ```
fn dismember(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_body_part: Object,
    ab_force_explode: bool,
    ab_force_dismember: bool,
    ab_force_bloody_mess: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `MarkItemAsFavorite`
///
/// ```text
/// Function(Form akItem, int aiSlot=-1) native
/// ```
fn mark_item_as_favorite(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item: Object,
    ai_slot_1: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `ModFactionRank`
///
/// ```text
/// Function(Faction akFaction, int aiMod) native
/// ```
fn mod_faction_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_faction: Object,
    ai_mod: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `MoveToPackageLocation`
///
/// ```text
/// Function() native
/// ```
fn move_to_package_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `OpenInventory`
///
/// ```text
/// Function(bool abForceOpen) native
/// ```
fn open_inventory(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_force_open: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `PathToReference`
///
/// ```text
/// bool Function(ObjectReference aTarget, float afWalkRunPercent) native
/// ```
fn path_to_reference(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_target: Object,
    af_walk_run_percent: f32,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayIdle`
///
/// ```text
/// bool Function(Idle akIdle) native
/// ```
fn play_idle(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_idle: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayIdleAction`
///
/// ```text
/// bool Function(Action aAction, ObjectReference aTarget) native
/// ```
fn play_idle_action(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_action: Object,
    a_target: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlayIdleWithTarget`
///
/// ```text
/// bool Function(Idle akIdle, ObjectReference akTarget) native
/// ```
fn play_idle_with_target(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_idle: Object,
    ak_target: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `PlaySubGraphAnimation`
///
/// ```text
/// Function(string asEventName) native
/// ```
fn play_sub_graph_animation(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_event_name: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveFromFaction`
///
/// ```text
/// Function(Faction akFaction) native
/// ```
fn remove_from_faction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<(), Exception> {
    let Some(faction) = caller
        .data()
        .objects
        .get_by_addr(faction)
        .and_then(|s| s.record())
    else {
        debug_assert!(faction.is_null(),);
        return Ok(());
    };

    let entity = caller.data().entity(self_ptr).unwrap();
    let mut props = caller
        .data()
        .world()
        .get::<&mut ActorProperties>(entity)
        .unwrap();

    props.factions.retain(|v| v.faction != faction);
    Ok(())
}

/// `RemoveFromAllFactions`
///
/// ```text
/// Function() native
/// ```
fn remove_from_all_factions(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr).unwrap();
    let mut props = caller
        .data()
        .world()
        .get::<&mut ActorProperties>(entity)
        .unwrap();

    props.factions.clear();
    Ok(())
}

/// `RemovePerk`
///
/// ```text
/// Function(Perk akPerk) native
/// ```
fn remove_perk(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_perk: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RemoveSpell`
///
/// ```text
/// bool Function(Spell akSpell) native
/// ```
fn remove_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_spell: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `ResetHealthAndLimbs`
///
/// ```text
/// Function() native
/// ```
fn reset_health_and_limbs(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Resurrect`
///
/// ```text
/// Function() native
/// ```
fn resurrect(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `SendAssaultAlarm`
///
/// ```text
/// Function() native
/// ```
fn send_assault_alarm(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `SendTrespassAlarm`
///
/// ```text
/// Function(Actor akCriminal) native
/// ```
fn send_trespass_alarm(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_criminal: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAlert`
///
/// ```text
/// Function(bool abAlerted) native
/// ```
fn set_alert(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_alerted: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAllowFlying`
///
/// ```text
/// Function(bool abAllowed, bool abAllowCrash, bool abAllowSearch) native
/// ```
fn set_allow_flying(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_allowed: bool,
    ab_allow_crash: bool,
    ab_allow_search: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAlpha`
///
/// ```text
/// Function(float afTargetAlpha, bool abFade) native
/// ```
fn set_alpha(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_target_alpha: f32,
    ab_fade: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAttackActorOnSight`
///
/// ```text
/// Function(bool abAttackOnSight) native
/// ```
fn set_attack_actor_on_sight(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_attack_on_sight: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAvoidPlayer`
///
/// ```text
/// Function(bool abAvoid) native
/// ```
fn set_avoid_player(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_avoid: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCommandState`
///
/// ```text
/// Function(bool abStartCommandMode) native
/// ```
fn set_command_state(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_start_command_mode: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetBribed`
///
/// ```text
/// Function(bool abBribe) native
/// ```
fn set_bribed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_bribe: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCanDoCommand`
///
/// ```text
/// Function(bool abCanCommand=) native
/// ```
fn set_can_do_command(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_can_command: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCombatStyle`
///
/// ```text
/// Function(CombatStyle akCombatStyle) native
/// ```
fn set_combat_style(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_combat_style: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCrimeFaction`
///
/// ```text
/// Function(Faction akFaction) native
/// ```
fn set_crime_faction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_faction: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCriticalStage`
///
/// ```text
/// Function(int aiStage) native
/// ```
fn set_critical_stage(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_stage: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetDoingFavor`
///
/// ```text
/// Function(bool abDoingFavor, bool abWorkShopMode=false) native
/// ```
fn set_doing_favor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_doing_favor: bool,
    ab_work_shop_mode_false: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `ChangeAnimFaceArchetype`
///
/// ```text
/// Function(keyword apKeyword) native
/// ```
fn change_anim_face_archetype(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_keyword: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetEyeTexture`
///
/// ```text
/// Function(TextureSet akNewTexture) native
/// ```
fn set_eye_texture(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_new_texture: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetEssential`
///
/// ```text
/// Function(bool abEssential) native
/// ```
fn set_essential(
    caller: Caller<'_, State>,
    self_ptr: Object,
    essential: bool,
) -> Result<(), Exception> {
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let mut props = caller
        .data()
        .world()
        .get::<&mut ActorProperties>(entity)
        .map_err(|_| Exception::Error)?;

    if essential {
        // Overwrite the protected flag if set.
        props.protection = ActorProtection::Essential;
    } else {
        // If called with essential = false, don't drop the protected
        // flag if it is currently set.
        if props.protection == ActorProtection::Essential {
            props.protection = ActorProtection::None
        }
    }

    Ok(())
}

/// `SetFactionRank`
///
/// ```text
/// Function(Faction akFaction, int aiRank) native
/// ```
fn set_faction_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
    ai_rank: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetForcedLandingMarker`
///
/// ```text
/// Function(ObjectReference aMarker) native
/// ```
fn set_forced_landing_marker(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_marker: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetGhost`
///
/// ```text
/// Function(bool abIsGhost) native
/// ```
fn set_ghost(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_is_ghost: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetHasCharGenSkeleton`
///
/// ```text
/// Function(bool abCharGen) native
/// ```
fn set_has_char_gen_skeleton(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_char_gen: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetHeadTracking`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn set_head_tracking(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetIntimidated`
///
/// ```text
/// Function(bool abIntimidate) native
/// ```
fn set_intimidated(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_intimidate: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetLookAt`
///
/// ```text
/// Function(ObjectReference akTarget, bool abPathingLookAt) native
/// ```
fn set_look_at(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    ab_pathing_look_at: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetNoBleedoutRecovery`
///
/// ```text
/// Function(bool abAllowed) native
/// ```
fn set_no_bleedout_recovery(
    caller: Caller<'_, State>,
    self_ptr: Object,
    not_allowed: bool,
) -> Result<(), Exception> {
    // The function sets whether bleedout recovery is disabled.
    let allowed = !not_allowed;

    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let Ok(mut props) = caller.data().world().get::<&mut ActorProperties>(entity) else {
        return Ok(());
    };

    props.bleedout_recovery = allowed;
    Ok(())
}

/// `SetNotShowOnStealthMeter`
///
/// ```text
/// Function(bool abNotShow) native
/// ```
fn set_not_show_on_stealth_meter(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_not_show: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetOutfit`
///
/// ```text
/// Function(Outfit akOutfit, bool abSleepOutfit) native
/// ```
fn set_outfit(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_outfit: Object,
    ab_sleep_outfit: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetOverrideVoiceType`
///
/// ```text
/// Function(VoiceType akVoiceType) native
/// ```
fn set_override_voice_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_voice_type: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPlayerControls`
///
/// ```text
/// Function(bool abControls) native
/// ```
fn set_player_controls(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_controls: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPlayerResistingArrest`
///
/// ```text
/// Function() native
/// ```
fn set_player_resisting_arrest(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPlayerTeammate`
///
/// ```text
/// Function(bool abTeammate, bool abCanDoFavor=true, bool abGivePlayerXP=false) native
/// ```
fn set_player_teammate(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_teammate: bool,
    ab_can_do_favor_true: bool,
    ab_give_player_xp_false: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetProtected`
///
/// ```text
/// Function(bool abProtected) native
/// ```
fn set_protected(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_protected: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetRace`
///
/// ```text
/// Function(Race akRace) native
/// ```
fn set_race(caller: Caller<'_, State>, self_ptr: Object, ak_race: Object) -> Result<(), Exception> {
    todo!()
}

/// `SetRelationshipRank`
///
/// ```text
/// Function(Actor akOther, int aiRank) native
/// ```
fn set_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
    ai_rank: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetRestrained`
///
/// ```text
/// bool Function(bool abRestrained) native
/// ```
fn set_restrained(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_restrained: bool,
) -> Result<bool, Exception> {
    todo!()
}

/// `SetSubGraphFloatVariable`
///
/// ```text
/// Function(string asVariableName, float afValue) native
/// ```
fn set_sub_graph_float_variable(
    caller: Caller<'_, State>,
    self_ptr: Object,
    as_variable_name: Object,
    af_value: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetUnconscious`
///
/// ```text
/// bool Function(bool abUnconscious) native
/// ```
fn set_unconscious(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_unconscious: bool,
) -> Result<bool, Exception> {
    todo!()
}

/// `SetVehicle`
///
/// ```text
/// Function(Actor akVehicle) native
/// ```
fn set_vehicle(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_vehicle: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `ShowBarterMenu`
///
/// ```text
/// Function() native
/// ```
fn show_barter_menu(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `StartCannibal`
///
/// ```text
/// Function(Actor akTarget) native
/// ```
fn start_cannibal(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `StartCombat`
///
/// ```text
/// Function(Actor akTarget, bool abPreferredTarget) native
/// ```
fn start_combat(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
    ab_preferred_target: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `StartDeferredKill`
///
/// ```text
/// Function() native
/// ```
fn start_deferred_kill(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `StartVampireFeed`
///
/// ```text
/// Function(Actor akTarget) native
/// ```
fn start_vampire_feed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `StartFrenzyAttack`
///
/// ```text
/// Function(float aChance, float aInterval) native
/// ```
fn start_frenzy_attack(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_chance: f32,
    a_interval: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `StopCombat`
///
/// ```text
/// Function() native
/// ```
fn stop_combat(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `StopCombatAlarm`
///
/// ```text
/// Function() native
/// ```
fn stop_combat_alarm(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `SwitchToPowerArmor`
///
/// ```text
/// Function(ObjectReference aArmorFurniture) native
/// ```
fn switch_to_power_armor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    a_armor_furniture: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SnapIntoInteraction`
///
/// ```text
/// bool Function(ObjectReference akTarget) native
/// ```
fn snap_into_interaction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `TrapSoul`
///
/// ```text
/// bool Function(Actor akTarget) native
/// ```
fn trap_soul(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_target: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `UnequipAll`
///
/// ```text
/// Function() native
/// ```
fn unequip_all(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `UnequipItem`
///
/// ```text
/// Function(Form akItem, bool abPreventEquip, bool abSilent) native
/// ```
fn unequip_item(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    prevent_equip: bool,
    silent: bool,
) -> Result<(), Exception> {
    let actor = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let Ok(inventory) = caller.data().world().get::<&Inventory>(actor) else {
        return Ok(());
    };

    let Some((slot, _)) = inventory.get_by_form(item).next() else {
        return Ok(());
    };

    drop(inventory);

    caller
        .data_mut()
        .run_effect_and_wait(UnequipItemEffect { actor, slot })?;

    Ok(())
}

/// `UnequipItemSlot`
///
/// ```text
/// Function(int aiSlot) native
/// ```
fn unequip_item_slot(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_slot: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `UnequipSpell`
///
/// ```text
/// Function(Spell akSpell, int aiSource) native
/// ```
fn unequip_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_spell: Object,
    ai_source: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `UnLockOwnedDoorsInCell`
///
/// ```text
/// Function() native
/// ```
fn un_lock_owned_doors_in_cell(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `WillIntimidateSucceed`
///
/// ```text
/// bool Function() native
/// ```
fn will_intimidate_succeed(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `WornHasKeyword`
///
/// ```text
/// bool Function(Keyword akKeyword) native
/// ```
fn worn_has_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `WouldBeStealing`
///
/// ```text
/// bool Function(ObjectReference akObject) native
/// ```
fn would_be_stealing(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_object: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `WouldRefuseCommand`
///
/// ```text
/// int Function(ObjectReference akObject) native
/// ```
fn would_refuse_command(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_object: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `StartSneaking`
///
/// ```text
/// Function() native
/// ```
fn start_sneaking(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DrawWeapon`
///
/// ```text
/// Function() native
/// ```
fn draw_weapon(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DogPlaceInMouth`
///
/// ```text
/// Function(Form akItem) native
/// ```
fn dog_place_in_mouth(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_item: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `DogDropItems`
///
/// ```text
/// Function() native
/// ```
fn dog_drop_items(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ForceMovementDirection`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn force_movement_direction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceMovementSpeed`
///
/// ```text
/// Function(float afSpeedMult) native
/// ```
fn force_movement_speed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_speed_mult: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceMovementRotationSpeed`
///
/// ```text
/// Function(float afXMult, float afYMult, float afZMult) native
/// ```
fn force_movement_rotation_speed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_mult: f32,
    af_y_mult: f32,
    af_z_mult: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceMovementDirectionRamp`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle, float afRampTime) native
/// ```
fn force_movement_direction_ramp(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
    af_ramp_time: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceMovementSpeedRamp`
///
/// ```text
/// Function(float afSpeedMult, float afRampTime) native
/// ```
fn force_movement_speed_ramp(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_speed_mult: f32,
    af_ramp_time: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceMovementRotationSpeedRamp`
///
/// ```text
/// Function(float afXMult, float afYMult, float afZMult, float afRampTime) native
/// ```
fn force_movement_rotation_speed_ramp(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_mult: f32,
    af_y_mult: f32,
    af_z_mult: f32,
    af_ramp_time: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceTargetDirection`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn force_target_direction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceTargetSpeed`
///
/// ```text
/// Function(float afSpeed) native
/// ```
fn force_target_speed(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_speed: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ForceTargetAngle`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn force_target_angle(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_x_angle: f32,
    af_y_angle: f32,
    af_z_angle: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `ClearForcedMovement`
///
/// ```text
/// Function() native
/// ```
fn clear_forced_movement(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `CanMoveVertical`
///
/// ```text
/// bool Function() native
/// ```
fn can_move_vertical(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `CanStrafe`
///
/// ```text
/// bool Function() native
/// ```
fn can_strafe(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetWornItem`
///
/// ```text
/// WornItem Function(int slotIndex, bool firstPerson) native
/// ```
fn get_worn_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    slot_index: i32,
    first_person: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetWornItemMods`
///
/// ```text
/// ObjectMod[] Function(int slotIndex) native
/// ```
fn get_worn_item_mods(
    caller: Caller<'_, State>,
    self_ptr: Object,
    slot_index: i32,
) -> Result<Object, Exception> {
    todo!()
}

/// `QueueUpdate`
///
/// ```text
/// Function(bool bDoEquipment, int flags) native
/// ```
fn queue_update(
    caller: Caller<'_, State>,
    self_ptr: Object,
    b_do_equipment: bool,
    flags: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `GetFurnitureReference`
///
/// ```text
/// ObjectReference Function() native
/// ```
fn get_furniture_reference(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `IsProtected`
///
/// ```text
/// bool Function() native
/// ```
fn is_protected(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    with_actor_props(caller, self_ptr, |actor| {
        actor.protection == ActorProtection::Protected
    })
}

fn with_actor_props<T, F>(caller: Caller<'_, State>, self_ptr: Object, f: F) -> Result<T, Exception>
where
    F: FnOnce(&ActorProperties) -> T,
{
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let props = caller
        .data()
        .world()
        .get::<&ActorProperties>(entity)
        .map_err(|_| Exception::Error)?;
    Ok(f(&props))
}

fn with_inventory<T, F>(caller: Caller<'_, State>, self_ptr: Object, f: F) -> Result<T, Exception>
where
    F: FnOnce(&Inventory) -> T,
{
    let entity = caller.data().entity(self_ptr).ok_or(Exception::Error)?;
    let inventory = caller
        .data()
        .world()
        .get::<&Inventory>(entity)
        .map_err(|_| Exception::Error)?;
    Ok(f(&inventory))
}
