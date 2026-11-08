use openfo4_esp::records::magic_effect::MagicffectTargetType::Self_;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::components::{ActorProperties, ActorProtection, MovementState, Player};
use crate::state::hardcoded::fallout4::GOLD;
use crate::systems::inventory::{Inventory, ItemData, WeaponItemData};
use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::effects::{EquipItemEffect, UnequipItemEffect};
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Actor", "AddPerk", add_perk);
    engine.register_if_exists("Actor", "AddSpell", add_spell);
    engine.register_if_exists("Actor", "AllowBleedoutDialogue", allow_bleedout_dialogue);
    engine.register_if_exists("Actor", "AllowPCDialogue", allow_pc_dialogue);
    engine.register_if_exists("Actor", "AttachAshPile", attach_ash_pile);
    engine.register_if_exists("Actor", "AttemptAnimationSetSwitch", attempt_animation_set_switch);
    engine.register_if_exists("Actor", "CanFlyHere", can_fly_here);
    engine.register_if_exists("Actor", "ChangeAnimArchetype", change_anim_archetype);
    engine.register_if_exists("Actor", "ChangeAnimFlavor", change_anim_flavor);
    engine.register_if_exists("Actor", "ChangeHeadPart", change_head_part);
    engine.register_if_exists("Actor", "ClearArrested", clear_arrested);
    engine.register_if_exists("Actor", "ClearExpressionOverride", clear_expression_override);
    engine.register_if_exists("Actor", "ClearExtraArrows", clear_extra_arrows);
    engine.register_if_exists("Actor", "ClearLookAt", clear_look_at);
    engine.register_if_exists("Actor", "Dismount", dismount);
    engine.register_if_exists("Actor", "DispelAllSpells", dispel_all_spells);
    engine.register_if_exists("Actor", "DispelSpell", dispel_spell);
    engine.register_if_exists("Actor", "DoCombatSpellApply", do_combat_spell_apply);
    engine.register_if_exists("Actor", "EnableAI", enable_ai);
    engine.register_if_exists("Actor", "EndDeferredKill", end_deferred_kill);
    engine.register_if_exists("Actor", "EquipItem", equip_item);
    engine.register_if_exists("Actor", "EquipSpell", equip_spell);
    engine.register_if_exists("Actor", "EvaluatePackage", evaluate_package);
    engine.register_if_exists("Actor", "GetBribeAmount", get_bribe_amount);
    engine.register_if_exists("Actor", "GetAllCombatTargets", get_all_combat_targets);
    engine.register_if_exists("Actor", "GetCrimeFaction", get_crime_faction);
    engine.register_if_exists("Actor", "GetCombatState", get_combat_state);
    engine.register_if_exists("Actor", "GetCombatTarget", get_combat_target);
    engine.register_if_exists("Actor", "GetCurrentPackage", get_current_package);
    engine.register_if_exists("Actor", "GetDialogueTarget", get_dialogue_target);
    engine.register_if_exists("Actor", "GetEquippedItemType", get_equipped_item_type);
    engine.register_if_exists("Actor", "GetEquippedWeapon", get_equipped_weapon);
    engine.register_if_exists("Actor", "GetEquippedShield", get_equipped_shield);
    engine.register_if_exists("Actor", "GetEquippedSpell", get_equipped_spell);
    engine.register_if_exists("Actor", "GetFactionRank", get_faction_rank);
    engine.register_if_exists("Actor", "GetFactionReaction", get_faction_reaction);
    engine.register_if_exists("Actor", "GetFlyingState", get_flying_state);
    engine.register_if_exists("Actor", "GetForcedLandingMarker", get_forced_landing_marker);
    engine.register_if_exists("Actor", "GetGoldAmount", get_gold_amount);
    engine.register_if_exists("Actor", "GetHighestRelationshipRank", get_highest_relationship_rank);
    engine.register_if_exists("Actor", "GetKiller", get_killer);
    engine.register_if_exists("Actor", "GetLevel", get_level);
    engine.register_if_exists("Actor", "GetLightLevel", get_light_level);
    engine.register_if_exists("Actor", "GetLowestRelationshipRank", get_lowest_relationship_rank);
    engine.register_if_exists("Actor", "GetLeveledActorBase", get_leveled_actor_base);
    engine.register_if_exists("Actor", "GetNoBleedoutRecovery", get_no_bleedout_recovery);
    engine.register_if_exists("Actor", "GetPlayerControls", get_player_controls);
    engine.register_if_exists("Actor", "GetRace", get_race);
    engine.register_if_exists("Actor", "GetRelationshipRank", get_relationship_rank);
    engine.register_if_exists("Actor", "GetSitState", get_sit_state);
    engine.register_if_exists("Actor", "GetSleepState", get_sleep_state);
    engine.register_if_exists("Actor", "HasAssociation", has_association);
    engine.register_if_exists("Actor", "HasFamilyRelationship", has_family_relationship);
    engine.register_if_exists("Actor", "HasDetectionLOS", has_detection_los);
    engine.register_if_exists("Actor", "HasMagicEffect", has_magic_effect);
    engine.register_if_exists("Actor", "HasMagicEffectWithKeyword", has_magic_effect_with_keyword);
    engine.register_if_exists("Actor", "HasParentRelationship", has_parent_relationship);
    engine.register_if_exists("Actor", "HasPerk", has_perk);
    engine.register_if_exists("Actor", "HasSpell", has_spell);
    engine.register_if_exists("Actor", "IsAIEnabled", is_ai_enabled);
    engine.register_if_exists("Actor", "IsAlarmed", is_alarmed);
    engine.register_if_exists("Actor", "IsAlerted", is_alerted);
    engine.register_if_exists("Actor", "IsAllowedToFly", is_allowed_to_fly);
    engine.register_if_exists("Actor", "IsArrested", is_arrested);
    engine.register_if_exists("Actor", "IsArrestingTarget", is_arresting_target);
    engine.register_if_exists("Actor", "IsBeingRidden", is_being_ridden);
    engine.register_if_exists("Actor", "IsBeingRiddenBy", is_being_ridden_by);
    engine.register_if_exists("Actor", "IsBleedingOut", is_bleeding_out);
    engine.register_if_exists("Actor", "IsBribed", is_bribed);
    engine.register_if_exists("Actor", "IsChild", is_child);
    engine.register_if_exists("Actor", "IsCommandedActor", is_commanded_actor);
    engine.register_if_exists("Actor", "IsDead", is_dead);
    engine.register_if_exists("Actor", "IsDetectedBy", is_detected_by);
    engine.register_if_exists("Actor", "IsDismembered", is_dismembered);
    engine.register_if_exists("Actor", "IsDoingFavor", is_doing_favor);
    engine.register_if_exists("Actor", "IsEquipped", is_equipped);
    engine.register_if_exists("Actor", "IsEssential", is_essential);
    engine.register_if_exists("Actor", "IsFlying", is_flying);
    engine.register_if_exists("Actor", "IsGuard", is_guard);
    engine.register_if_exists("Actor", "IsGhost", is_ghost);
    engine.register_if_exists("Actor", "IsHostileToActor", is_hostile_to_actor);
    engine.register_if_exists("Actor", "IsInCombat", is_in_combat);
    engine.register_if_exists("Actor", "IsInFaction", is_in_faction);
    engine.register_if_exists("Actor", "IsInIronSights", is_in_iron_sights);
    engine.register_if_exists("Actor", "IsInKillMove", is_in_kill_move);
    engine.register_if_exists("Actor", "IsInScene", is_in_scene);
    engine.register_if_exists("Actor", "IsIntimidated", is_intimidated);
    engine.register_if_exists("Actor", "IsOnMount", is_on_mount);
    engine.register_if_exists("Actor", "IsOverEncumbered", is_over_encumbered);
    engine.register_if_exists("Actor", "IsPlayersLastRiddenHorse", is_players_last_ridden_horse);
    engine.register_if_exists("Actor", "IsPlayerTeammate", is_player_teammate);
    engine.register_if_exists("Actor", "IsRunning", is_running);
    engine.register_if_exists("Actor", "IsSeatOccupied", is_seat_occupied);
    engine.register_if_exists("Actor", "IsSneaking", is_sneaking);
    engine.register_if_exists("Actor", "IsSprinting", is_sprinting);
    engine.register_if_exists("Actor", "IsTalking", is_talking);
    engine.register_if_exists("Actor", "IsTrespassing", is_trespassing);
    engine.register_if_exists("Actor", "IsUnconscious", is_unconscious);
    engine.register_if_exists("Actor", "IsWeaponDrawn", is_weapon_drawn);
    engine.register_if_exists("Actor", "Kill", kill);
    engine.register_if_exists("Actor", "KillSilent", kill_silent);
    engine.register_if_exists("Actor", "Dismember", dismember);
    engine.register_if_exists("Actor", "MarkItemAsFavorite", mark_item_as_favorite);
    engine.register_if_exists("Actor", "ModFactionRank", mod_faction_rank);
    engine.register_if_exists("Actor", "MoveToPackageLocation", move_to_package_location);
    engine.register_if_exists("Actor", "OpenInventory", open_inventory);
    engine.register_if_exists("Actor", "PathToReference", path_to_reference);
    engine.register_if_exists("Actor", "PlayIdle", play_idle);
    engine.register_if_exists("Actor", "PlayIdleAction", play_idle_action);
    engine.register_if_exists("Actor", "PlayIdleWithTarget", play_idle_with_target);
    engine.register_if_exists("Actor", "PlaySubGraphAnimation", play_sub_graph_animation);
    engine.register_if_exists("Actor", "RemoveFromFaction", remove_from_faction);
    engine.register_if_exists("Actor", "RemoveFromAllFactions", remove_from_all_factions);
    engine.register_if_exists("Actor", "RemovePerk", remove_perk);
    engine.register_if_exists("Actor", "RemoveSpell", remove_spell);
    engine.register_if_exists("Actor", "ResetHealthAndLimbs", reset_health_and_limbs);
    engine.register_if_exists("Actor", "Resurrect", resurrect);
    engine.register_if_exists("Actor", "SendAssaultAlarm", send_assault_alarm);
    engine.register_if_exists("Actor", "SendTrespassAlarm", send_trespass_alarm);
    engine.register_if_exists("Actor", "SetAlert", set_alert);
    engine.register_if_exists("Actor", "SetAllowFlying", set_allow_flying);
    engine.register_if_exists("Actor", "SetAlpha", set_alpha);
    engine.register_if_exists("Actor", "SetAttackActorOnSight", set_attack_actor_on_sight);
    engine.register_if_exists("Actor", "SetAvoidPlayer", set_avoid_player);
    engine.register_if_exists("Actor", "SetCommandState", set_command_state);
    engine.register_if_exists("Actor", "SetBribed", set_bribed);
    engine.register_if_exists("Actor", "SetCanDoCommand", set_can_do_command);
    engine.register_if_exists("Actor", "SetCombatStyle", set_combat_style);
    engine.register_if_exists("Actor", "SetCrimeFaction", set_crime_faction);
    engine.register_if_exists("Actor", "SetCriticalStage", set_critical_stage);
    engine.register_if_exists("Actor", "SetDoingFavor", set_doing_favor);
    engine.register_if_exists("Actor", "ChangeAnimFaceArchetype", change_anim_face_archetype);
    engine.register_if_exists("Actor", "SetEyeTexture", set_eye_texture);
    engine.register_if_exists("Actor", "SetEssential", set_essential);
    engine.register_if_exists("Actor", "SetFactionRank", set_faction_rank);
    engine.register_if_exists("Actor", "SetForcedLandingMarker", set_forced_landing_marker);
    engine.register_if_exists("Actor", "SetGhost", set_ghost);
    engine.register_if_exists("Actor", "SetHasCharGenSkeleton", set_has_char_gen_skeleton);
    engine.register_if_exists("Actor", "SetHeadTracking", set_head_tracking);
    engine.register_if_exists("Actor", "SetIntimidated", set_intimidated);
    engine.register_if_exists("Actor", "SetLookAt", set_look_at);
    engine.register_if_exists("Actor", "SetNoBleedoutRecovery", set_no_bleedout_recovery);
    engine.register_if_exists("Actor", "SetNotShowOnStealthMeter", set_not_show_on_stealth_meter);
    engine.register_if_exists("Actor", "SetOutfit", set_outfit);
    engine.register_if_exists("Actor", "SetOverrideVoiceType", set_override_voice_type);
    engine.register_if_exists("Actor", "SetPlayerControls", set_player_controls);
    engine.register_if_exists("Actor", "SetPlayerResistingArrest", set_player_resisting_arrest);
    engine.register_if_exists("Actor", "SetPlayerTeammate", set_player_teammate);
    engine.register_if_exists("Actor", "SetProtected", set_protected);
    engine.register_if_exists("Actor", "SetRace", set_race);
    engine.register_if_exists("Actor", "SetRelationshipRank", set_relationship_rank);
    engine.register_if_exists("Actor", "SetRestrained", set_restrained);
    engine.register_if_exists("Actor", "SetSubGraphFloatVariable", set_sub_graph_float_variable);
    engine.register_if_exists("Actor", "SetUnconscious", set_unconscious);
    engine.register_if_exists("Actor", "SetVehicle", set_vehicle);
    engine.register_if_exists("Actor", "ShowBarterMenu", show_barter_menu);
    engine.register_if_exists("Actor", "StartCannibal", start_cannibal);
    engine.register_if_exists("Actor", "StartCombat", start_combat);
    engine.register_if_exists("Actor", "StartDeferredKill", start_deferred_kill);
    engine.register_if_exists("Actor", "StartVampireFeed", start_vampire_feed);
    engine.register_if_exists("Actor", "StartFrenzyAttack", start_frenzy_attack);
    engine.register_if_exists("Actor", "StopCombat", stop_combat);
    engine.register_if_exists("Actor", "StopCombatAlarm", stop_combat_alarm);
    engine.register_if_exists("Actor", "SwitchToPowerArmor", switch_to_power_armor);
    engine.register_if_exists("Actor", "SnapIntoInteraction", snap_into_interaction);
    engine.register_if_exists("Actor", "TrapSoul", trap_soul);
    engine.register_if_exists("Actor", "UnequipAll", unequip_all);
    engine.register_if_exists("Actor", "UnequipItem", unequip_item);
    engine.register_if_exists("Actor", "UnequipItemSlot", unequip_item_slot);
    engine.register_if_exists("Actor", "UnequipSpell", unequip_spell);
    engine.register_if_exists("Actor", "UnLockOwnedDoorsInCell", unlock_owned_doors_in_cell);
    engine.register_if_exists("Actor", "WillIntimidateSucceed", will_intimidate_succeed);
    engine.register_if_exists("Actor", "WornHasKeyword", worn_has_keyword);
    engine.register_if_exists("Actor", "WouldBeStealing", would_be_stealing);
    engine.register_if_exists("Actor", "WouldRefuseCommand", would_refuse_command);
    engine.register_if_exists("Actor", "StartSneaking", start_sneaking);
    engine.register_if_exists("Actor", "DrawWeapon", draw_weapon);
    engine.register_if_exists("Actor", "DogPlaceInMouth", dog_place_in_mouth);
    engine.register_if_exists("Actor", "DogDropItems", dog_drop_items);
    engine.register_if_exists("Actor", "ForceMovementDirection", force_movement_direction);
    engine.register_if_exists("Actor", "ForceMovementSpeed", force_movement_speed);
    engine.register_if_exists("Actor", "ForceMovementRotationSpeed", force_movement_rotation_speed);
    engine.register_if_exists("Actor", "ForceMovementDirectionRamp", force_movement_direction_ramp);
    engine.register_if_exists("Actor", "ForceMovementSpeedRamp", force_movement_speed_ramp);
    engine.register_if_exists("Actor", "ForceMovementRotationSpeedRamp", force_movement_rotation_speed_ramp);
    engine.register_if_exists("Actor", "ForceTargetDirection", force_target_direction);
    engine.register_if_exists("Actor", "ForceTargetSpeed", force_target_speed);
    engine.register_if_exists("Actor", "ForceTargetAngle", force_target_angle);
    engine.register_if_exists("Actor", "ClearForcedMovement", clear_forced_movement);
    engine.register_if_exists("Actor", "CanMoveVertical", can_move_vertical);
    engine.register_if_exists("Actor", "CanStrafe", can_strafe);
    engine.register_if_exists("Actor", "GetWornItem", get_worn_item);
    engine.register_if_exists("Actor", "GetWornItemMods", get_worn_item_mods);
    engine.register_if_exists("Actor", "QueueUpdate", queue_update);
    engine.register_if_exists("Actor", "GetFurnitureReference", get_furniture_reference);
    engine.register_if_exists("Actor", "IsProtected", is_protected);
}

/// `AddPerk`
///
/// ```text
/// Function(Perk akPerk, bool abNotify=false) native
/// ```
fn add_perk(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_perk: Object,
    _ab_notify_false: bool,
) -> Result<(), Error> {
    todo!()
}

/// `AddSpell`
///
/// ```text
/// bool Function(Spell akSpell, bool abVerbose=true) native
/// ```
fn add_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_spell: Object,
    _ab_verbose_true: bool,
) -> Result<bool, Error> {
    // todo!()
    Ok(true)
}

/// `AllowBleedoutDialogue`
///
/// ```text
/// Function(bool abCanTalk) native
/// ```
fn allow_bleedout_dialogue(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_can_talk: bool,
) -> Result<(), Error> {
    todo!()
}

/// `AllowPCDialogue`
///
/// ```text
/// Function(bool abTalk) native
/// ```
fn allow_pc_dialogue(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_talk: bool,
) -> Result<(), Error> {
    todo!()
}

/// `AttachAshPile`
///
/// ```text
/// Function(Form akAshPileBase) native
/// ```
fn attach_ash_pile(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_ash_pile_base: Object,
) -> Result<(), Error> {
    todo!()
}

/// `AttemptAnimationSetSwitch`
///
/// ```text
/// Function() native
/// ```
fn attempt_animation_set_switch(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<(), Error> {
    todo!()
}

/// `CanFlyHere`
///
/// ```text
/// bool Function() native
/// ```
fn can_fly_here(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `ChangeAnimArchetype`
///
/// ```text
/// bool Function(keyword apKeyword) native
/// ```
fn change_anim_archetype(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ap_keyword: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `ChangeAnimFlavor`
///
/// ```text
/// bool Function(keyword apKeyword) native
/// ```
fn change_anim_flavor(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ap_keyword: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `ChangeHeadPart`
///
/// ```text
/// Function(headpart apHeadPart, bool abRemovePart, bool abRemoveExtraParts) native
/// ```
fn change_head_part(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ap_head_part: Object,
    _ab_remove_part: bool,
    _ab_remove_extra_parts: bool,
) -> Result<(), Error> {
    todo!()
}

/// `ClearArrested`
///
/// ```text
/// Function() native
/// ```
fn clear_arrested(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `ClearExpressionOverride`
///
/// ```text
/// Function() native
/// ```
fn clear_expression_override(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `ClearExtraArrows`
///
/// ```text
/// Function() native
/// ```
fn clear_extra_arrows(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `ClearLookAt`
///
/// ```text
/// Function() native
/// ```
fn clear_look_at(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Dismount`
///
/// ```text
/// bool Function() native
/// ```
fn dismount(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `DispelAllSpells`
///
/// ```text
/// Function() native
/// ```
fn dispel_all_spells(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `DispelSpell`
///
/// ```text
/// bool Function(Spell akSpell) native
/// ```
fn dispel_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_spell: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `DoCombatSpellApply`
///
/// ```text
/// Function(Spell akSpell, ObjectReference akTarget) native
/// ```
fn do_combat_spell_apply(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_spell: Object,
    _ak_target: Object,
) -> Result<(), Error> {
    todo!()
}

/// `EnableAI`
///
/// ```text
/// Function(bool abEnable, bool abPauseVoice) native
/// ```
fn enable_ai(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_enable: bool,
    _ab_pause_voice: bool,
) -> Result<(), Error> {
    todo!()
}

/// `EndDeferredKill`
///
/// ```text
/// Function() native
/// ```
fn end_deferred_kill(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `EquipItem`
///
/// ```text
/// Function(Form akItem, bool abPreventRemoval, bool abSilent) native
/// ```
async fn equip_item(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    prevent_removal: bool,
    silent: bool,
) -> Result<(), Error> {
    let actor = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    caller
        .data_mut()
        .run_effect_and_wait(EquipItemEffect { actor, item })
        .await?;

    Ok(())
}

/// `EquipSpell`
///
/// ```text
/// Function(Spell akSpell, int aiSource) native
/// ```
fn equip_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_spell: Object,
    _ai_source: i32,
) -> Result<(), Error> {
    todo!()
}

/// `EvaluatePackage`
///
/// ```text
/// Function(bool abResetAI) native
/// ```
fn evaluate_package(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_reset_ai: bool,
) -> Result<(), Error> {
    todo!()
}

/// `GetBribeAmount`
///
/// ```text
/// int Function() native
/// ```
fn get_bribe_amount(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetAllCombatTargets`
///
/// ```text
/// Actor[] Function() native
/// ```
fn get_all_combat_targets(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetCrimeFaction`
///
/// ```text
/// Faction Function() native
/// ```
fn get_crime_faction(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetCombatState`
///
/// ```text
/// int Function() native
/// ```
fn get_combat_state(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetCombatTarget`
///
/// ```text
/// Actor Function() native
/// ```
fn get_combat_target(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetCurrentPackage`
///
/// ```text
/// Package Function() native
/// ```
fn get_current_package(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetDialogueTarget`
///
/// ```text
/// Actor Function() native
/// ```
fn get_dialogue_target(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetEquippedItemType`
///
/// ```text
/// int Function(int aiEquipIndex) native
/// ```
fn get_equipped_item_type(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_equip_index: i32,
) -> Result<i32, Error> {
    todo!()
}

/// `GetEquippedWeapon`
///
/// ```text
/// Weapon Function(int aiEquipIndex) native
/// ```
fn get_equipped_weapon(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_equip_index: i32,
) -> Result<Object, Error> {
    todo!()
}

/// `GetEquippedShield`
///
/// ```text
/// Armor Function() native
/// ```
fn get_equipped_shield(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetEquippedSpell`
///
/// ```text
/// Spell Function(int aiSource) native
/// ```
fn get_equipped_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_source: i32,
) -> Result<Object, Error> {
    todo!()
}

/// `GetFactionRank`
///
/// ```text
/// int Function(Faction akFaction) native
/// ```
async fn get_faction_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<i32, Error> {
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
    let world = caller.data().world().await;
    let props = world.get::<&ActorProperties>(entity).unwrap();

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
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetFlyingState`
///
/// ```text
/// int Function() native
/// ```
fn get_flying_state(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetForcedLandingMarker`
///
/// ```text
/// ObjectReference Function() native
/// ```
fn get_forced_landing_marker(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `GetGoldAmount`
///
/// ```text
/// int Function() native
/// ```
async fn get_gold_amount(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    with_inventory(caller, self_ptr, |inventory| {
        let mut sum: i32 = 0;
        for (_, item) in inventory.get_by_form(GOLD) {
            sum = sum.saturating_add(item.count.try_into().unwrap_or(i32::MAX));
        }

        sum
    })
    .await
}

/// `GetHighestRelationshipRank`
///
/// ```text
/// int Function() native
/// ```
fn get_highest_relationship_rank(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetKiller`
///
/// ```text
/// Actor Function() native
/// ```
fn get_killer(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetLevel`
///
/// ```text
/// int Function() native
/// ```
async fn get_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(props) = world.get::<&ActorProperties>(entity) else {
        return Ok(0);
    };

    Ok(props.level.try_into().unwrap_or(i32::MAX))
}

/// `GetLightLevel`
///
/// ```text
/// float Function() native
/// ```
fn get_light_level(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<f32, Error> {
    todo!()
}

/// `GetLowestRelationshipRank`
///
/// ```text
/// int Function() native
/// ```
fn get_lowest_relationship_rank(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetLeveledActorBase`
///
/// ```text
/// ActorBase Function() native
/// ```
fn get_leveled_actor_base(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetNoBleedoutRecovery`
///
/// ```text
/// bool Function() native
/// ```
async fn get_no_bleedout_recovery(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Error> {
    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(props) = world.get::<&ActorProperties>(entity) else {
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
async fn get_player_controls(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let is_player = world.entity(entity).is_ok_and(|e| e.has::<Player>());
    Ok(is_player)
}

/// `GetRace`
///
/// ```text
/// Race Function() native
/// ```
fn get_race(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetRelationshipRank`
///
/// ```text
/// int Function(Actor akOther) native
/// ```
fn get_relationship_rank(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetSitState`
///
/// ```text
/// int Function() native
/// ```
fn get_sit_state(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetSleepState`
///
/// ```text
/// int Function() native
/// ```
fn get_sleep_state(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `HasAssociation`
///
/// ```text
/// bool Function(AssociationType akAssociation, Actor akOther) native
/// ```
fn has_association(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_association: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasFamilyRelationship`
///
/// ```text
/// bool Function(Actor akOther) native
/// ```
fn has_family_relationship(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasDetectionLOS`
///
/// ```text
/// bool Function(ObjectReference akOther) native
/// ```
fn has_detection_los(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasMagicEffect`
///
/// ```text
/// bool Function(MagicEffect akEffect) native
/// ```
fn has_magic_effect(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_effect: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasMagicEffectWithKeyword`
///
/// ```text
/// bool Function(Keyword akKeyword) native
/// ```
fn has_magic_effect_with_keyword(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_keyword: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasParentRelationship`
///
/// ```text
/// bool Function(Actor akOther) native
/// ```
fn has_parent_relationship(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasPerk`
///
/// ```text
/// bool Function(Perk akPerk) native
/// ```
fn has_perk(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_perk: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasSpell`
///
/// ```text
/// bool Function(Form akForm) native
/// ```
fn has_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_form: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsAIEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_ai_enabled(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsAlarmed`
///
/// ```text
/// bool Function() native
/// ```
fn is_alarmed(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsAlerted`
///
/// ```text
/// bool Function() native
/// ```
fn is_alerted(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsAllowedToFly`
///
/// ```text
/// bool Function() native
/// ```
fn is_allowed_to_fly(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsArrested`
///
/// ```text
/// bool Function() native
/// ```
fn is_arrested(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsArrestingTarget`
///
/// ```text
/// bool Function() native
/// ```
fn is_arresting_target(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsBeingRidden`
///
/// ```text
/// bool Function() native
/// ```
fn is_being_ridden(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsBeingRiddenBy`
///
/// ```text
/// bool Function(Actor akActor) native
/// ```
fn is_being_ridden_by(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_actor: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsBleedingOut`
///
/// ```text
/// bool Function() native
/// ```
fn is_bleeding_out(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsBribed`
///
/// ```text
/// bool Function() native
/// ```
fn is_bribed(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsChild`
///
/// ```text
/// bool Function() native
/// ```
async fn is_child(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_actor_props(caller, self_ptr, |actor| actor.is_child).await
}

/// `IsCommandedActor`
///
/// ```text
/// bool Function() native
/// ```
fn is_commanded_actor(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsDead`
///
/// ```text
/// bool Function() native
/// ```
async fn is_dead(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_actor_props(caller, self_ptr, |actor| actor.is_dead).await
}

/// `IsDetectedBy`
///
/// ```text
/// bool Function(Actor akOther) native
/// ```
fn is_detected_by(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsDismembered`
///
/// ```text
/// bool Function(string asBodyPart) native
/// ```
fn is_dismembered(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _as_body_part: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsDoingFavor`
///
/// ```text
/// bool Function() native
/// ```
fn is_doing_favor(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsEquipped`
///
/// ```text
/// bool Function(Form akItem) native
/// ```
async fn is_equipped(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
) -> Result<bool, Error> {
    let actor = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let world = caller.data().world().await;
    let Ok(inventory) = world.get::<&Inventory>(actor) else {
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
async fn is_essential(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_actor_props(caller, self_ptr, |actor| {
        actor.protection == ActorProtection::Essential
    })
    .await
}

/// `IsFlying`
///
/// ```text
/// bool Function() native
/// ```
fn is_flying(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsGuard`
///
/// ```text
/// bool Function() native
/// ```
fn is_guard(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsGhost`
///
/// ```text
/// bool Function() native
/// ```
fn is_ghost(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsHostileToActor`
///
/// ```text
/// bool Function(Actor akActor) native
/// ```
fn is_hostile_to_actor(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_actor: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsInCombat`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_combat(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsInFaction`
///
/// ```text
/// bool Function(Faction akFaction) native
/// ```
async fn is_in_faction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<bool, Error> {
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
    let world = caller.data().world().await;
    let props = world.get::<&ActorProperties>(entity).unwrap();

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
fn is_in_iron_sights(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsInKillMove`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_kill_move(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsInScene`
///
/// ```text
/// bool Function() native
/// ```
fn is_in_scene(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsIntimidated`
///
/// ```text
/// bool Function() native
/// ```
fn is_intimidated(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsOnMount`
///
/// ```text
/// bool Function() native
/// ```
fn is_on_mount(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsOverEncumbered`
///
/// ```text
/// bool Function() native
/// ```
fn is_over_encumbered(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsPlayersLastRiddenHorse`
///
/// ```text
/// bool Function() native
/// ```
fn is_players_last_ridden_horse(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsPlayerTeammate`
///
/// ```text
/// bool Function() native
/// ```
fn is_player_teammate(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsRunning`
///
/// ```text
/// bool Function() native
/// ```
fn is_running(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsSeatOccupied`
///
/// ```text
/// bool Function(keyword apKeyword) native
/// ```
fn is_seat_occupied(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ap_keyword: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsSneaking`
///
/// ```text
/// bool Function() native
/// ```
async fn is_sneaking(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    let Some(entity) = caller.data().entity(self_ptr) else {
        return false;
    };

    let world = caller.data().world().await;
    let Ok(state) = world.get::<&MovementState>(entity) else {
        return false;
    };

    state.is_sneaking
}

/// `IsSprinting`
///
/// ```text
/// bool Function() native
/// ```
fn is_sprinting(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsTalking`
///
/// ```text
/// bool Function() native
/// ```
fn is_talking(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsTrespassing`
///
/// ```text
/// bool Function() native
/// ```
fn is_trespassing(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsUnconscious`
///
/// ```text
/// bool Function() native
/// ```
fn is_unconscious(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsWeaponDrawn`
///
/// ```text
/// bool Function() native
/// ```
async fn is_weapon_drawn(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_inventory(caller, self_ptr, |inventory| {
        inventory.items().any(|(_, item)| {
            item.equipped
                && matches!(
                    &item.data,
                    ItemData::Weapon(WeaponItemData { drawn: true, .. }),
                )
        })
    })
    .await
}

/// `Kill`
///
/// ```text
/// Function(Actor akKiller) native
/// ```
fn kill(_caller: Caller<'_, State>, _self_ptr: Object, _ak_killer: Object) -> Result<(), Error> {
    todo!()
}

/// `KillSilent`
///
/// ```text
/// Function(Actor akKiller) native
/// ```
fn kill_silent(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_killer: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Dismember`
///
/// ```text
/// Function(string asBodyPart, bool abForceExplode, bool abForceDismember, bool abForceBloodyMess) native
/// ```
fn dismember(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _as_body_part: Object,
    _ab_force_explode: bool,
    _ab_force_dismember: bool,
    _ab_force_bloody_mess: bool,
) -> Result<(), Error> {
    todo!()
}

/// `MarkItemAsFavorite`
///
/// ```text
/// Function(Form akItem, int aiSlot=-1) native
/// ```
fn mark_item_as_favorite(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_item: Object,
    _ai_slot_1: i32,
) -> Result<(), Error> {
    todo!()
}

/// `ModFactionRank`
///
/// ```text
/// Function(Faction akFaction, int aiMod) native
/// ```
fn mod_faction_rank(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_faction: Object,
    _ai_mod: i32,
) -> Result<(), Error> {
    todo!()
}

/// `MoveToPackageLocation`
///
/// ```text
/// Function() native
/// ```
fn move_to_package_location(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `OpenInventory`
///
/// ```text
/// Function(bool abForceOpen) native
/// ```
fn open_inventory(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_force_open: bool,
) -> Result<(), Error> {
    todo!()
}

/// `PathToReference`
///
/// ```text
/// bool Function(ObjectReference aTarget, float afWalkRunPercent) native
/// ```
fn path_to_reference(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _a_target: Object,
    _af_walk_run_percent: f32,
) -> Result<bool, Error> {
    todo!()
}

/// `PlayIdle`
///
/// ```text
/// bool Function(Idle akIdle) native
/// ```
fn play_idle(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_idle: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `PlayIdleAction`
///
/// ```text
/// bool Function(Action aAction, ObjectReference aTarget) native
/// ```
fn play_idle_action(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _a_action: Object,
    _a_target: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `PlayIdleWithTarget`
///
/// ```text
/// bool Function(Idle akIdle, ObjectReference akTarget) native
/// ```
fn play_idle_with_target(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_idle: Object,
    _ak_target: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `PlaySubGraphAnimation`
///
/// ```text
/// Function(string asEventName) native
/// ```
fn play_sub_graph_animation(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _as_event_name: Object,
) -> Result<(), Error> {
    todo!()
}

/// `RemoveFromFaction`
///
/// ```text
/// Function(Faction akFaction) native
/// ```
async fn remove_from_faction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<(), Error> {
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
    let world = caller.data().world().await;
    let mut props = world.get::<&mut ActorProperties>(entity).unwrap();

    props.factions.retain(|v| v.faction != faction);
    Ok(())
}

/// `RemoveFromAllFactions`
///
/// ```text
/// Function() native
/// ```
async fn remove_from_all_factions(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Error> {
    let entity = caller.data().entity(self_ptr).unwrap();
    let world = caller.data().world().await;
    let mut props = world.get::<&mut ActorProperties>(entity).unwrap();

    props.factions.clear();
    Ok(())
}

/// `RemovePerk`
///
/// ```text
/// Function(Perk akPerk) native
/// ```
fn remove_perk(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_perk: Object,
) -> Result<(), Error> {
    todo!()
}

/// `RemoveSpell`
///
/// ```text
/// bool Function(Spell akSpell) native
/// ```
fn remove_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_spell: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `ResetHealthAndLimbs`
///
/// ```text
/// Function() native
/// ```
fn reset_health_and_limbs(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Resurrect`
///
/// ```text
/// Function() native
/// ```
fn resurrect(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SendAssaultAlarm`
///
/// ```text
/// Function() native
/// ```
fn send_assault_alarm(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SendTrespassAlarm`
///
/// ```text
/// Function(Actor akCriminal) native
/// ```
fn send_trespass_alarm(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_criminal: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetAlert`
///
/// ```text
/// Function(bool abAlerted) native
/// ```
fn set_alert(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_alerted: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetAllowFlying`
///
/// ```text
/// Function(bool abAllowed, bool abAllowCrash, bool abAllowSearch) native
/// ```
fn set_allow_flying(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_allowed: bool,
    _ab_allow_crash: bool,
    _ab_allow_search: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetAlpha`
///
/// ```text
/// Function(float afTargetAlpha, bool abFade) native
/// ```
fn set_alpha(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_target_alpha: f32,
    _ab_fade: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetAttackActorOnSight`
///
/// ```text
/// Function(bool abAttackOnSight) native
/// ```
fn set_attack_actor_on_sight(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_attack_on_sight: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetAvoidPlayer`
///
/// ```text
/// Function(bool abAvoid) native
/// ```
fn set_avoid_player(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_avoid: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetCommandState`
///
/// ```text
/// Function(bool abStartCommandMode) native
/// ```
fn set_command_state(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_start_command_mode: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetBribed`
///
/// ```text
/// Function(bool abBribe) native
/// ```
fn set_bribed(_caller: Caller<'_, State>, _self_ptr: Object, _ab_bribe: bool) -> Result<(), Error> {
    todo!()
}

/// `SetCanDoCommand`
///
/// ```text
/// Function(bool abCanCommand=) native
/// ```
fn set_can_do_command(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_can_command: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetCombatStyle`
///
/// ```text
/// Function(CombatStyle akCombatStyle) native
/// ```
fn set_combat_style(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_combat_style: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetCrimeFaction`
///
/// ```text
/// Function(Faction akFaction) native
/// ```
fn set_crime_faction(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_faction: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetCriticalStage`
///
/// ```text
/// Function(int aiStage) native
/// ```
fn set_critical_stage(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_stage: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetDoingFavor`
///
/// ```text
/// Function(bool abDoingFavor, bool abWorkShopMode=false) native
/// ```
fn set_doing_favor(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_doing_favor: bool,
    _ab_work_shop_mode_false: bool,
) -> Result<(), Error> {
    todo!()
}

/// `ChangeAnimFaceArchetype`
///
/// ```text
/// Function(keyword apKeyword) native
/// ```
fn change_anim_face_archetype(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ap_keyword: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetEyeTexture`
///
/// ```text
/// Function(TextureSet akNewTexture) native
/// ```
fn set_eye_texture(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_new_texture: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetEssential`
///
/// ```text
/// Function(bool abEssential) native
/// ```
async fn set_essential(
    caller: Caller<'_, State>,
    self_ptr: Object,
    essential: bool,
) -> Result<(), Error> {
    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let mut props = world
        .get::<&mut ActorProperties>(entity)
        .map_err(|_| todo!())?;

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
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _faction: Object,
    _ai_rank: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetForcedLandingMarker`
///
/// ```text
/// Function(ObjectReference aMarker) native
/// ```
fn set_forced_landing_marker(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _a_marker: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetGhost`
///
/// ```text
/// Function(bool abIsGhost) native
/// ```
fn set_ghost(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_is_ghost: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetHasCharGenSkeleton`
///
/// ```text
/// Function(bool abCharGen) native
/// ```
fn set_has_char_gen_skeleton(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_char_gen: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetHeadTracking`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn set_head_tracking(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_enable: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetIntimidated`
///
/// ```text
/// Function(bool abIntimidate) native
/// ```
fn set_intimidated(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_intimidate: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetLookAt`
///
/// ```text
/// Function(ObjectReference akTarget, bool abPathingLookAt) native
/// ```
fn set_look_at(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_target: Object,
    _ab_pathing_look_at: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetNoBleedoutRecovery`
///
/// ```text
/// Function(bool abAllowed) native
/// ```
async fn set_no_bleedout_recovery(
    caller: Caller<'_, State>,
    self_ptr: Object,
    not_allowed: bool,
) -> Result<(), Error> {
    // The function sets whether bleedout recovery is disabled.
    let allowed = !not_allowed;

    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let Ok(mut props) = world.get::<&mut ActorProperties>(entity) else {
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
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_not_show: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetOutfit`
///
/// ```text
/// Function(Outfit akOutfit, bool abSleepOutfit) native
/// ```
fn set_outfit(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_outfit: Object,
    _ab_sleep_outfit: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetOverrideVoiceType`
///
/// ```text
/// Function(VoiceType akVoiceType) native
/// ```
fn set_override_voice_type(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_voice_type: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerControls`
///
/// ```text
/// Function(bool abControls) native
/// ```
fn set_player_controls(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_controls: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerResistingArrest`
///
/// ```text
/// Function() native
/// ```
fn set_player_resisting_arrest(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerTeammate`
///
/// ```text
/// Function(bool abTeammate, bool abCanDoFavor=true, bool abGivePlayerXP=false) native
/// ```
fn set_player_teammate(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_teammate: bool,
    _ab_can_do_favor_true: bool,
    _ab_give_player_xp_false: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetProtected`
///
/// ```text
/// Function(bool abProtected) native
/// ```
fn set_protected(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_protected: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetRace`
///
/// ```text
/// Function(Race akRace) native
/// ```
fn set_race(_caller: Caller<'_, State>, _self_ptr: Object, _ak_race: Object) -> Result<(), Error> {
    todo!()
}

/// `SetRelationshipRank`
///
/// ```text
/// Function(Actor akOther, int aiRank) native
/// ```
fn set_relationship_rank(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
    _ai_rank: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetRestrained`
///
/// ```text
/// bool Function(bool abRestrained) native
/// ```
fn set_restrained(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_restrained: bool,
) -> Result<bool, Error> {
    todo!()
}

/// `SetSubGraphFloatVariable`
///
/// ```text
/// Function(string asVariableName, float afValue) native
/// ```
fn set_sub_graph_float_variable(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _as_variable_name: Object,
    _af_value: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetUnconscious`
///
/// ```text
/// bool Function(bool abUnconscious) native
/// ```
fn set_unconscious(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_unconscious: bool,
) -> Result<bool, Error> {
    todo!()
}

/// `SetVehicle`
///
/// ```text
/// Function(Actor akVehicle) native
/// ```
fn set_vehicle(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_vehicle: Object,
) -> Result<(), Error> {
    todo!()
}

/// `ShowBarterMenu`
///
/// ```text
/// Function() native
/// ```
fn show_barter_menu(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `StartCannibal`
///
/// ```text
/// Function(Actor akTarget) native
/// ```
fn start_cannibal(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_target: Object,
) -> Result<(), Error> {
    todo!()
}

/// `StartCombat`
///
/// ```text
/// Function(Actor akTarget, bool abPreferredTarget) native
/// ```
fn start_combat(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_target: Object,
    _ab_preferred_target: bool,
) -> Result<(), Error> {
    todo!()
}

/// `StartDeferredKill`
///
/// ```text
/// Function() native
/// ```
fn start_deferred_kill(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `StartVampireFeed`
///
/// ```text
/// Function(Actor akTarget) native
/// ```
fn start_vampire_feed(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_target: Object,
) -> Result<(), Error> {
    todo!()
}

/// `StartFrenzyAttack`
///
/// ```text
/// Function(float aChance, float aInterval) native
/// ```
fn start_frenzy_attack(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _a_chance: f32,
    _a_interval: f32,
) -> Result<(), Error> {
    todo!()
}

/// `StopCombat`
///
/// ```text
/// Function() native
/// ```
fn stop_combat(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `StopCombatAlarm`
///
/// ```text
/// Function() native
/// ```
fn stop_combat_alarm(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SwitchToPowerArmor`
///
/// ```text
/// Function(ObjectReference aArmorFurniture) native
/// ```
fn switch_to_power_armor(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _a_armor_furniture: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SnapIntoInteraction`
///
/// ```text
/// bool Function(ObjectReference akTarget) native
/// ```
fn snap_into_interaction(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_target: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `TrapSoul`
///
/// ```text
/// bool Function(Actor akTarget) native
/// ```
fn trap_soul(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_target: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `UnequipAll`
///
/// ```text
/// Function() native
/// ```
fn unequip_all(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `UnequipItem`
///
/// ```text
/// Function(Form akItem, bool abPreventEquip, bool abSilent) native
/// ```
async fn unequip_item(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    prevent_equip: bool,
    silent: bool,
) -> Result<(), Error> {
    let actor = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let item = caller
        .data()
        .objects
        .get_by_addr(item)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;

    let slot = {
        let world = caller.data().world().await;
        let Ok(inventory) = world.get::<&Inventory>(actor) else {
            return Ok(());
        };

        let Some((slot, _)) = inventory.get_by_form(item).next() else {
            return Ok(());
        };

        slot
    };

    caller
        .data_mut()
        .run_effect_and_wait(UnequipItemEffect { actor, slot })
        .await?;

    Ok(())
}

/// `UnequipItemSlot`
///
/// ```text
/// Function(int aiSlot) native
/// ```
fn unequip_item_slot(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_slot: i32,
) -> Result<(), Error> {
    todo!()
}

/// `UnequipSpell`
///
/// ```text
/// Function(Spell akSpell, int aiSource) native
/// ```
fn unequip_spell(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_spell: Object,
    _ai_source: i32,
) -> Result<(), Error> {
    todo!()
}

/// `UnLockOwnedDoorsInCell`
///
/// ```text
/// Function() native
/// ```
fn unlock_owned_doors_in_cell(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `WillIntimidateSucceed`
///
/// ```text
/// bool Function() native
/// ```
fn will_intimidate_succeed(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `WornHasKeyword`
///
/// ```text
/// bool Function(Keyword akKeyword) native
/// ```
async fn worn_has_keyword(caller: Caller<'_, State>, self_ptr: Object, keyword: Object) -> bool {
    if keyword.is_null() {
        return false;
    }

    let entity = caller.data().entity(self_ptr).unwrap();
    let keyword = caller.data().record(keyword);

    let world = caller.data().world().await;

    let Ok(inventory) = world.get::<&Inventory>(entity) else {
        return false;
    };

    for (_, item) in inventory.items() {
        if item.equipped && item.combined_keywords.contains(&keyword) {
            return true;
        }
    }

    false
}

/// `WouldBeStealing`
///
/// ```text
/// bool Function(ObjectReference akObject) native
/// ```
fn would_be_stealing(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_object: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `WouldRefuseCommand`
///
/// ```text
/// int Function(ObjectReference akObject) native
/// ```
fn would_refuse_command(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_object: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `StartSneaking`
///
/// ```text
/// Function() native
/// ```
fn start_sneaking(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `DrawWeapon`
///
/// ```text
/// Function() native
/// ```
fn draw_weapon(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `DogPlaceInMouth`
///
/// ```text
/// Function(Form akItem) native
/// ```
fn dog_place_in_mouth(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_item: Object,
) -> Result<(), Error> {
    todo!()
}

/// `DogDropItems`
///
/// ```text
/// Function() native
/// ```
fn dog_drop_items(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `ForceMovementDirection`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn force_movement_direction(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_x_angle: f32,
    _af_y_angle: f32,
    _af_z_angle: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceMovementSpeed`
///
/// ```text
/// Function(float afSpeedMult) native
/// ```
fn force_movement_speed(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_speed_mult: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceMovementRotationSpeed`
///
/// ```text
/// Function(float afXMult, float afYMult, float afZMult) native
/// ```
fn force_movement_rotation_speed(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_x_mult: f32,
    _af_y_mult: f32,
    _af_z_mult: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceMovementDirectionRamp`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle, float afRampTime) native
/// ```
fn force_movement_direction_ramp(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_x_angle: f32,
    _af_y_angle: f32,
    _af_z_angle: f32,
    _af_ramp_time: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceMovementSpeedRamp`
///
/// ```text
/// Function(float afSpeedMult, float afRampTime) native
/// ```
fn force_movement_speed_ramp(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_speed_mult: f32,
    _af_ramp_time: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceMovementRotationSpeedRamp`
///
/// ```text
/// Function(float afXMult, float afYMult, float afZMult, float afRampTime) native
/// ```
fn force_movement_rotation_speed_ramp(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_x_mult: f32,
    _af_y_mult: f32,
    _af_z_mult: f32,
    _af_ramp_time: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceTargetDirection`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn force_target_direction(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_x_angle: f32,
    _af_y_angle: f32,
    _af_z_angle: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceTargetSpeed`
///
/// ```text
/// Function(float afSpeed) native
/// ```
fn force_target_speed(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_speed: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ForceTargetAngle`
///
/// ```text
/// Function(float afXAngle, float afYAngle, float afZAngle) native
/// ```
fn force_target_angle(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_x_angle: f32,
    _af_y_angle: f32,
    _af_z_angle: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ClearForcedMovement`
///
/// ```text
/// Function() native
/// ```
fn clear_forced_movement(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `CanMoveVertical`
///
/// ```text
/// bool Function() native
/// ```
fn can_move_vertical(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `CanStrafe`
///
/// ```text
/// bool Function() native
/// ```
fn can_strafe(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetWornItem`
///
/// ```text
/// WornItem Function(int slotIndex, bool firstPerson) native
/// ```
fn get_worn_item(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _slot_index: i32,
    _first_person: bool,
) -> Result<Object, Error> {
    todo!()
}

/// `GetWornItemMods`
///
/// ```text
/// ObjectMod[] Function(int slotIndex) native
/// ```
fn get_worn_item_mods(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _slot_index: i32,
) -> Result<Object, Error> {
    todo!()
}

/// `QueueUpdate`
///
/// ```text
/// Function(bool bDoEquipment, int flags) native
/// ```
fn queue_update(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _b_do_equipment: bool,
    _flags: i32,
) -> Result<(), Error> {
    todo!()
}

/// `GetFurnitureReference`
///
/// ```text
/// ObjectReference Function() native
/// ```
fn get_furniture_reference(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `IsProtected`
///
/// ```text
/// bool Function() native
/// ```
async fn is_protected(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_actor_props(caller, self_ptr, |actor| {
        actor.protection == ActorProtection::Protected
    })
    .await
}

async fn with_actor_props<T, F>(
    caller: Caller<'_, State>,
    self_ptr: Object,
    f: F,
) -> Result<T, Error>
where
    F: FnOnce(&ActorProperties) -> T,
{
    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let props = world.get::<&ActorProperties>(entity).map_err(|_| todo!())?;
    Ok(f(&props))
}

async fn with_inventory<T, F>(caller: Caller<'_, State>, self_ptr: Object, f: F) -> Result<T, Error>
where
    F: FnOnce(&Inventory) -> T,
{
    let entity = caller.data().entity(self_ptr).ok_or_else(|| todo!())?;
    let world = caller.data().world().await;
    let inventory = world.get::<&Inventory>(entity).map_err(|_| todo!())?;
    Ok(f(&inventory))
}
