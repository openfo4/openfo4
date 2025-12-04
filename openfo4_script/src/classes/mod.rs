use std::collections::HashMap;
use std::f32::consts::E;
use std::sync::RwLock;
use std::time::Duration;

use crate::Engine;

mod actor;
mod form;
mod game;
mod global_variable;
mod math;
mod message;
mod object_reference;
mod script_object;
mod utility;

pub type Int = i32;
pub type Float = f32;
pub type Object = isize;
pub type Bool = bool;

// Don't reformat long lines in here. This function is essentially a big table
// and one line per entry makes it clearer.
// FIXME: Optimally we would like to only skip long lines check, but is this possible?
#[rustfmt::skip]
pub fn register_builtins(engine: &mut Engine) {
    engine.register_native_function("Math", "sin", math::sin);
    engine.register_native_function("Math", "cos", math::cos);
    engine.register_native_function("Math", "tan", math::tan);
    engine.register_native_function("Math", "acos", math::acos);
    engine.register_native_function("Math", "asin", math::asin);
    engine.register_native_function("Math", "atan", math::atan);

    engine.register_native_function("ObjectReference", "Activate", object_reference::activate);
    engine.register_native_function("ObjectReference", "AddItem", object_reference::add_item);
    engine.register_native_function("ObjectReference", "Delete", object_reference::delete);
    engine.register_native_function("ObjectReference", "DeleteWhenAble", object_reference::delete_when_able);
    engine.register_native_function("ObjectReference", "Disable", object_reference::disable);
    engine.register_native_function("ObjectReference", "GetValue", object_reference::get_value);
    engine.register_native_function("ObjectReference", "SetValue", object_reference::set_value);
    engine.register_native_function("ObjectReference", "GetBaseValue", object_reference::get_base_value);
    engine.register_native_function("ObjectReference", "SetActorOwner", object_reference::set_actor_owner);
    engine.register_native_function("ObjectReference", "GetBaseObject", object_reference::get_base_object);
    engine.register_native_function("ObjectReference", "RemoveItem", object_reference::remove_item);

    engine.register_native_function("Game", "GetPlayer", game::get_player);
    engine.register_native_function("Game", "GetForm", game::get_form);

    engine.register_native_function("Message", "Show", message::show);

    // engine.register_native_function("ScriptObject", "CancelTimerGameTime", script_object::cancel_timer_game_time);
    // engine.register_native_function("ScriptObject", "RegisterForAnimationEvent", script_object::register_for_animation_event);
    // engine.register_native_function("ScriptObject", "UnregisterForLOS", script_object::unregister_for_los);
    // engine.register_native_function("ScriptObject", "UnregisterForRemoteEvent", script_object::unregister_for_remote_event);
    // engine.register_native_function("ScriptObject", "StartTimerGameTime", script_object::start_timer_game_time);

    // engine.register_native_function("Form", "GetFormID", form::get_form_id);
    // engine.register_native_function("Form", "GetGoldValue", form::get_gold_value);
    // engine.register_native_function("Form", "HasKeyword", form::has_keyword);
    // engine.register_native_function(
    //     "Form",
    //     "HasKeywordInFormList",
    //     form::has_keyword_in_form_list,
    // );
    // engine.register_native_function("Form", "PlayerKnows", form::player_knows);

    // engine.register_native_function("GlobalVariable", "GetValue", global_variable::get_value);
    // engine.register_native_function("GlobalVariable", "SetValue", global_variable::set_value);

    // engine.register_native_function("Actor", "AddPerk", actor::add_perk);
    // engine.register_native_function("Actor", "AddSpell", actor::add_spell);
    // engine.register_native_function("Actor", "AllowBleedoutDialogue", actor::allow_bleedout_dialogue);
    // engine.register_native_function("Actor", "AllowPCDialogue", actor::allow_pc_dialogue);
    // engine.register_native_function("Actor", "AttachAshPile", actor::attach_ash_pile);
    // engine.register_native_function("Actor", "AttemptAnimationSetSwitch", actor::attempt_animation_set_switch);
    // engine.register_native_function("Actor", "CanFlyHere", actor::can_fly_here);
    // engine.register_native_function("Actor", "ChangeAnimArchetype", actor::change_anim_archetype);
    // engine.register_native_function("Actor", "ChangeAnimFaceArchetype", actor::change_anim_face_archetype);
    // engine.register_native_function("Actor", "ChangeAnimFlavor", actor::change_anim_flavor);
    // engine.register_native_function("Actor", "ChangeHeadPart", actor::change_head_part);
    // engine.register_native_function("Actor", "ClearArrested", actor::clear_arrested);
    // engine.register_native_function("Actor", "ClearExpressionOverride", actor::clear_expression_override);
    // engine.register_native_function("Actor", "ClearExtraArrows", actor::clear_extra_arrows);
    // engine.register_native_function("Actor", "ClearLookAt", actor::clear_look_at);
    // engine.register_native_function("Actor", "Dismember", actor::dismember);
    // engine.register_native_function("Actor", "Dismount", actor::dismount);
    // engine.register_native_function("Actor", "DispelAllSpells", actor::dispel_all_spells);
    // engine.register_native_function("Actor", "DoCombatSpellApply",  actor::do_combat_spell_apply);
    // engine.register_native_function("Actor", "DogDropItems", actor::dog_drop_items);
    // engine.register_native_function("Actor", "DogPlaceInMouth", actor::dog_place_in_mouth);
    // engine.register_native_function("Actor", "DrawWeapon", actor::draw_weapon);
    // engine.register_native_function("Actor", "EnableAI", actor::enable_ai);
    // engine.register_native_function("Actor", "EndDeferredKill", actor::end_deferred_kill);
    // engine.register_native_function("Actor", "EquipItem",  actor::equip_item);
    // engine.register_native_function("Actor", "EquipSpell", actor::equip_spell);
    // engine.register_native_function("Actor", "EvaluatePackage", actor::evaluate_package);
    // engine.register_native_function("Actor", "GetAllCombatTargets", actor::get_all_combat_targets);
    // engine.register_native_function("Actor", "GetBribeAmount", actor::get_bribe_amount);
    // engine.register_native_function("Actor", "GetCrimeFaction", actor::get_crime_faction);
    // engine.register_native_function("Actor", "GetCombatState", actor::get_combat_state);
    // engine.register_native_function("Actor", "GetCombatTarget", actor::get_combat_target);
    // engine.register_native_function("Actor", "GetCurrentPackage", actor::get_current_package);
    // engine.register_native_function("Actor", "GetDialogueTarget", actor::get_dialogue_target);
    // engine.register_native_function("Actor", "GetEquippedItemType", actor::get_equipped_item_type);
    // engine.register_native_function("Actor", "GetEquippedShield", actor::get_equipped_shield);
    // engine.register_native_function("Actor", "GetEquippedSpell", actor::get_equipped_spell);
    // engine.register_native_function("Actor", "GetEquippedWeapon", actor::get_equipped_weapon);
    // engine.register_native_function("Actor", "GetFactionRank", actor::get_faction_rank);
    // engine.register_native_function("Actor", "GetFactionReaction", actor::get_faction_reaction);
    // engine.register_native_function("Actor", "GetFlyingState", actor::get_flying_state);
    // engine.register_native_function("Actor", "GetForcedLandingMarker", actor::get_forced_landing_marker);
    // engine.register_native_function("Actor", "GetGoldAmount", actor::get_gold_amount);
    // engine.register_native_function("Actor", "GetHighestRelationshipRank", actor::get_highest_relationship_rank);
    // engine.register_native_function("Actor", "GetKiller", actor::get_killer);
    // engine.register_native_function("Actor", "GetLevel", actor::get_level);
    // engine.register_native_function("Actor", "GetLeveledActorBase", actor::get_leveled_actor_base);
    // engine.register_native_function("Actor", "GetLightLevel", actor::get_light_level);
    // engine.register_native_function("Actor", "GetLowestRelationshipRank", actor::get_lowest_relationship_rank);
    // engine.register_native_function("Actor", "GetNoBleedoutRecovery", actor::get_no_bleedout_recovery);
    // engine.register_native_function("Actor", "GetPlayerControls", actor::get_player_controls);
    // engine.register_native_function("Actor", "GetRace", actor::get_race);
    // engine.register_native_function("Actor", "GetRelationshipRank", actor::get_relationship_rank);
    // engine.register_native_function("Actor", "GetSitState", actor::get_sit_state);
    // engine.register_native_function("Actor", "GetSleepState", actor::get_sleep_state);
    // engine.register_native_function("Actor", "HasAssociation", actor::has_association);
    // engine.register_native_function("Actor", "HasDetectionLOS", actor::has_detection_los);
    // engine.register_native_function("Actor", "HasFamilyRelationship", actor::has_family_relationship);
    // engine.register_native_function("Actor", "HasMagicEffect", actor::has_magic_effect);
    // engine.register_native_function("Actor", "HasMagicEffectWithKeyword", actor::has_magic_effect_with_keyword);
    // engine.register_native_function("Actor", "HasParentRelationship", actor::has_parent_relationship);
    // engine.register_native_function("Actor", "HasPerk", actor::has_perk);
    // engine.register_native_function("Actor", "HasSpell", actor::has_spell);
    // engine.register_native_function("Actor", "IsAIEnabled", actor::is_ai_enabled);
    // engine.register_native_function("Actor", "IsAlarmed", actor::is_alarmed);
    // engine.register_native_function("Actor", "IsAlerted", actor::is_alerted);
    // engine.register_native_function("Actor", "IsAllowedToFly", actor::is_allowed_to_fly);
    // engine.register_native_function("Actor", "IsArrested", actor::is_arrested);
    // engine.register_native_function("Actor", "IsArrestingTarget", actor::is_arresting_target);
    // engine.register_native_function("Actor", "IsBeingRidden", actor::is_being_ridden);
    // engine.register_native_function("Actor", "IsBeingRiddenBy", actor::is_being_ridden_by);
    // engine.register_native_function("Actor", "IsBleedingOut", actor::is_bleeding_out);
    // engine.register_native_function("Actor", "IsBribed", actor::is_bribed);
    // engine.register_native_function("Actor", "IsChild", actor::is_child);
    // engine.register_native_function("Actor", "IsCommandedActor", actor::is_commanded_actor);
    // engine.register_native_function("Actor", "IsDead", actor::is_dead);
    // engine.register_native_function("Actor", "IsDetectedBy", actor::is_detected_by);
    // engine.register_native_function("Actor", "IsDismembered", actor::is_dismembered);
    // engine.register_native_function("Actor", "IsDoingFavor", actor::is_doing_favor);
    // engine.register_native_function("Actor", "IsEquipped", actor::is_equipped);
    // engine.register_native_function("Actor", "IsEssential", actor::is_essential);
    // engine.register_native_function("Actor", "IsFlying", actor::is_flying);
    // engine.register_native_function("Actor", "IsGhost", actor::is_ghost);
    // engine.register_native_function("Actor", "IsGuard", actor::is_guard);

}

pub struct State {
    objects: RwLock<HashMap<Object, ObjectData>>,
}

pub struct ObjectData {
    timers: HashMap<Int, Vec<Timer>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Timer {
    RealTime(Duration),
    GameTime(Duration),
}
