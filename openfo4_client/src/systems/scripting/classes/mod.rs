mod active_magic_effect;
mod actor;
mod faction;
mod form;
mod form_list;
mod game;
mod global_variable;
mod keyword;
mod magic_effect;
mod math;
mod message;
mod object_reference;
mod potion;
mod quest;
mod ref_collection_alias;
mod reference_alias;
mod script_object;
mod sound;
mod spell;
mod utility;

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use openfo4_script::engine::Engine;
use openfo4_script::runtime::{ObjectAddr, ObjectRef};
use parking_lot::Mutex;
use slab::Slab;

pub type Int = i32;
pub type Float = f32;
pub type Object = ObjectAddr;
pub type Bool = bool;

// Don't reformat long lines in here. This function is essentially a big table
// and one line per entry makes it clearer.
// FIXME: Optimally we would like to only skip long lines check, but is this possible?
#[rustfmt::skip]
pub fn register_builtins(engine: &mut Engine) {
    engine.register_native_function("Math", "abs", math::abs);
    engine.register_native_function("Math", "acos", math::acos);
    engine.register_native_function("Math", "asin", math::asin);
    engine.register_native_function("Math", "atan", math::atan);
    engine.register_native_function("Math", "Ceiling", math::ceiling);
    engine.register_native_function("Math", "cos", math::cos);
    engine.register_native_function("Math", "DegreesToRadians", math::degrees_to_radians);
    engine.register_native_function("Math", "Floor", math::floor);
    engine.register_native_function("Math", "Max", math::max);
    engine.register_native_function("Math", "Min", math::min);
    engine.register_native_function("Math", "Pow", math::pow);
    engine.register_native_function("Math", "RadiansToDegrees", math::radians_to_degrees);
    engine.register_native_function("Math", "sin", math::sin);
    engine.register_native_function("Math", "sqrt", math::sqrt);
    engine.register_native_function("Math", "tan", math::tan);

    engine.register_native_function("ObjectReference", "Activate", object_reference::activate);
    engine.register_native_function("ObjectReference", "AddItem", object_reference::add_item);
    engine.register_native_function("ObjectReference", "DamageValue", object_reference::damage_value);
    engine.register_native_function("ObjectReference", "Delete", object_reference::delete);
    engine.register_native_function("ObjectReference", "DeleteWhenAble", object_reference::delete_when_able);
    engine.register_native_function("ObjectReference", "Disable", object_reference::disable);
    engine.register_native_function("ObjectReference", "GetValue", object_reference::get_value);
    engine.register_native_function("ObjectReference", "SetValue", object_reference::set_value);
    engine.register_native_function("ObjectReference", "GetBaseValue", object_reference::get_base_value);
    engine.register_native_function("ObjectReference", "GetItemCount", object_reference::get_item_count);
    engine.register_native_function("ObjectReference", "GetLinkedRef", object_reference::get_linked_ref);
    engine.register_native_function("ObjectReference", "HasKeyword", object_reference::has_keyword);
    engine.register_native_function("ObjectReference", "SetActorOwner", object_reference::set_actor_owner);
    engine.register_native_function("ObjectReference", "GetBaseObject", object_reference::get_base_object);
    engine.register_native_function("ObjectReference", "GetDistance", object_reference::get_distance);
    engine.register_native_function("ObjectReference", "IsOwnedBy", object_reference::is_owned_by);
    engine.register_native_function("ObjectReference", "RemoveItem", object_reference::remove_item);
    engine.register_native_function("ObjectReference", "RestoreValue", object_reference::restore_value);

    engine.register_native_function("Game", "GetAggressionAV", game::get_agression_av);
    engine.register_native_function("Game", "GetAgilityAV", game::get_agility_av);
    engine.register_native_function("Game", "GetCharismaAV",  game::get_charisma_av);
    engine.register_native_function("Game", "GetConfidenceAV", game::get_confidence_av);
    engine.register_native_function("Game", "GetDifficulty", game::get_difficulty);
    engine.register_native_function("Game", "GetEnduranceAV", game::get_endurance_av);
    engine.register_native_function("Game", "GetForm", game::get_form);
    engine.register_native_function("Game", "GetFormFromFile", game::get_form_from_file);
    engine.register_native_function("Game", "GetHealthAV", game::get_health_av);
    engine.register_native_function("Game", "GetIntelligenceAV", game::get_intelligence_av);
    engine.register_native_function("Game", "GetLuckAV", game::get_luck_av);
    engine.register_native_function("Game", "GetPerceptionAV", game::get_perception_av);
    engine.register_native_function("Game", "GetPlayer", game::get_player);
    engine.register_native_function("Game", "GetSuspiciousAV", game::get_suspicious_av);
    engine.register_native_function("Game", "GetStrengthAV", game::get_strength_av);
    engine.register_native_function("Game", "ShowFatigueWarningOnHUD", game::show_fatigue_warning_on_hud);
    engine.register_native_function("Game", "ShowPerkVaultBoyOnHUD", game::show_perk_vault_boy_on_hud);

    engine.register_native_function("Message", "ClearHelpMessages", message::clear_help_messages);
    engine.register_native_function("Message", "ResetHelpMessage", message::reset_help_message);
    engine.register_native_function("Message", "Show", message::show);
    engine.register_native_function("Message", "ShowAsHelpMessage", message::show_as_help_message);
    engine.register_native_function("Message", "UnshowAsHelpMessage", message::unshow_as_help_message);

    engine.register_native_function("ScriptObject", "CancelTimer", script_object::cancel_timer);
    engine.register_native_function("ScriptObject", "CancelTimerGameTime", script_object::cancel_timer_game_time);
    engine.register_native_function("ScriptObject", "RegisterForAnimationEvent", script_object::register_for_animation_event);
    engine.register_native_function("ScriptObject", "RegisterForCustomEvent", script_object::register_for_custom_event);
    engine.register_native_function("ScriptObject", "RegisterForDistanceGreaterThanEvent", script_object::register_for_distance_greater_than_event);
    engine.register_native_function("ScriptObject", "RegisterForDistanceLessThanEvent", script_object::register_for_distance_less_than_event);
    engine.register_native_function("ScriptObject", "RegisterForHitEvent", script_object::register_for_hit_event);
    engine.register_native_function("ScriptObject", "RegisterForMenuOpenCloseEvent", script_object::register_for_menu_open_close_event);
    engine.register_native_function("ScriptObject", "RegisterForPlayerSleep", script_object::register_for_player_sleep);
    engine.register_native_function("ScriptObject", "RegisterForPlayerTeleport", script_object::register_for_player_teleport);
    engine.register_native_function("ScriptObject", "RegisterForPlayerWait", script_object::register_for_player_wait);
    engine.register_native_function("ScriptObject", "RegisterForRemoteEvent", script_object::register_for_remote_event);
    // engine.register_native_function("ScriptObject", "UnregisterForLOS", script_object::unregister_for_los);
    // engine.register_native_function("ScriptObject", "UnregisterForRemoteEvent", script_object::unregister_for_remote_event);
    engine.register_native_function("ScriptObject", "StartTimer", script_object::start_timer);
    engine.register_native_function("ScriptObject", "StartTimerGameTime", script_object::start_timer_game_time);
    engine.register_native_function("ScriptObject", "UnregisterForTutorialEvent", script_object::unregister_for_tutorial_event);

    engine.register_native_function("Form", "GetFormID", form::get_form_id);
    engine.register_native_function("Form", "GetGoldValue", form::get_gold_value);
    engine.register_native_function("Form", "HasKeyword", form::has_keyword);
    engine.register_native_function("Form", "HasKeywordInFormList", form::has_keyword_in_form_list);
    engine.register_native_function("Form", "PlayerKnows", form::player_knows);

    // engine.register_native_function("GlobalVariable", "GetValue", global_variable::get_value);
    // engine.register_native_function("GlobalVariable", "SetValue", global_variable::set_value);

    engine.register_native_function("Actor", "AddPerk", actor::add_perk);
    engine.register_native_function("Actor", "AddSpell", actor::add_spell);
    engine.register_native_function("Actor", "AllowBleedoutDialogue", actor::allow_bleedout_dialogue);
    engine.register_native_function("Actor", "AllowPCDialogue", actor::allow_pc_dialogue);
    engine.register_native_function("Actor", "AttachAshPile", actor::attach_ash_pile);
    engine.register_native_function("Actor", "AttemptAnimationSetSwitch", actor::attempt_animation_set_switch);
    engine.register_native_function("Actor", "CanFlyHere", actor::can_fly_here);
    engine.register_native_function("Actor", "ChangeAnimArchetype", actor::change_anim_archetype);
    engine.register_native_function("Actor", "ChangeAnimFaceArchetype", actor::change_anim_face_archetype);
    engine.register_native_function("Actor", "ChangeAnimFlavor", actor::change_anim_flavor);
    engine.register_native_function("Actor", "ChangeHeadPart", actor::change_head_part);
    engine.register_native_function("Actor", "ClearArrested", actor::clear_arrested);
    engine.register_native_function("Actor", "ClearExpressionOverride", actor::clear_expression_override);
    engine.register_native_function("Actor", "ClearExtraArrows", actor::clear_extra_arrows);
    engine.register_native_function("Actor", "ClearLookAt", actor::clear_look_at);
    engine.register_native_function("Actor", "Dismember", actor::dismember);
    engine.register_native_function("Actor", "Dismount", actor::dismount);
    engine.register_native_function("Actor", "DispelAllSpells", actor::dispel_all_spells);
    engine.register_native_function("Actor", "DispelSpell", actor::dispel_spell);
    engine.register_native_function("Actor", "DoCombatSpellApply",  actor::do_combat_spell_apply);
    engine.register_native_function("Actor", "DogDropItems", actor::dog_drop_items);
    engine.register_native_function("Actor", "DogPlaceInMouth", actor::dog_place_in_mouth);
    engine.register_native_function("Actor", "DrawWeapon", actor::draw_weapon);
    engine.register_native_function("Actor", "EnableAI", actor::enable_ai);
    engine.register_native_function("Actor", "EndDeferredKill", actor::end_deferred_kill);
    engine.register_native_function("Actor", "EquipItem",  actor::equip_item);
    engine.register_native_function("Actor", "EquipSpell", actor::equip_spell);
    engine.register_native_function("Actor", "EvaluatePackage", actor::evaluate_package);
    engine.register_native_function("Actor", "GetAllCombatTargets", actor::get_all_combat_targets);
    engine.register_native_function("Actor", "GetBribeAmount", actor::get_bribe_amount);
    engine.register_native_function("Actor", "GetCrimeFaction", actor::get_crime_faction);
    engine.register_native_function("Actor", "GetCombatState", actor::get_combat_state);
    engine.register_native_function("Actor", "GetCombatTarget", actor::get_combat_target);
    engine.register_native_function("Actor", "GetCurrentPackage", actor::get_current_package);
    engine.register_native_function("Actor", "GetDialogueTarget", actor::get_dialogue_target);
    engine.register_native_function("Actor", "GetEquippedItemType", actor::get_equipped_item_type);
    engine.register_native_function("Actor", "GetEquippedShield", actor::get_equipped_shield);
    engine.register_native_function("Actor", "GetEquippedSpell", actor::get_equipped_spell);
    engine.register_native_function("Actor", "GetEquippedWeapon", actor::get_equipped_weapon);
    engine.register_native_function("Actor", "GetFactionRank", actor::get_faction_rank);
    engine.register_native_function("Actor", "GetFactionReaction", actor::get_faction_reaction);
    engine.register_native_function("Actor", "GetFlyingState", actor::get_flying_state);
    engine.register_native_function("Actor", "GetForcedLandingMarker", actor::get_forced_landing_marker);
    engine.register_native_function("Actor", "GetGoldAmount", actor::get_gold_amount);
    engine.register_native_function("Actor", "GetHighestRelationshipRank", actor::get_highest_relationship_rank);
    engine.register_native_function("Actor", "GetKiller", actor::get_killer);
    engine.register_native_function("Actor", "GetLevel", actor::get_level);
    engine.register_native_function("Actor", "GetLeveledActorBase", actor::get_leveled_actor_base);
    engine.register_native_function("Actor", "GetLightLevel", actor::get_light_level);
    engine.register_native_function("Actor", "GetLowestRelationshipRank", actor::get_lowest_relationship_rank);
    engine.register_native_function("Actor", "GetNoBleedoutRecovery", actor::get_no_bleedout_recovery);
    engine.register_native_function("Actor", "GetPlayerControls", actor::get_player_controls);
    engine.register_native_function("Actor", "GetRace", actor::get_race);
    engine.register_native_function("Actor", "GetRelationshipRank", actor::get_relationship_rank);
    engine.register_native_function("Actor", "GetSitState", actor::get_sit_state);
    engine.register_native_function("Actor", "GetSleepState", actor::get_sleep_state);
    engine.register_native_function("Actor", "HasAssociation", actor::has_association);
    engine.register_native_function("Actor", "HasDetectionLOS", actor::has_detection_los);
    engine.register_native_function("Actor", "HasFamilyRelationship", actor::has_family_relationship);
    engine.register_native_function("Actor", "HasMagicEffect", actor::has_magic_effect);
    engine.register_native_function("Actor", "HasMagicEffectWithKeyword", actor::has_magic_effect_with_keyword);
    engine.register_native_function("Actor", "HasParentRelationship", actor::has_parent_relationship);
    engine.register_native_function("Actor", "HasPerk", actor::has_perk);
    engine.register_native_function("Actor", "HasSpell", actor::has_spell);
    engine.register_native_function("Actor", "IsAIEnabled", actor::is_ai_enabled);
    engine.register_native_function("Actor", "IsAlarmed", actor::is_alarmed);
    engine.register_native_function("Actor", "IsAlerted", actor::is_alerted);
    engine.register_native_function("Actor", "IsAllowedToFly", actor::is_allowed_to_fly);
    engine.register_native_function("Actor", "IsArrested", actor::is_arrested);
    engine.register_native_function("Actor", "IsArrestingTarget", actor::is_arresting_target);
    engine.register_native_function("Actor", "IsBeingRidden", actor::is_being_ridden);
    engine.register_native_function("Actor", "IsBeingRiddenBy", actor::is_being_ridden_by);
    engine.register_native_function("Actor", "IsBleedingOut", actor::is_bleeding_out);
    engine.register_native_function("Actor", "IsBribed", actor::is_bribed);
    engine.register_native_function("Actor", "IsChild", actor::is_child);
    engine.register_native_function("Actor", "IsCommandedActor", actor::is_commanded_actor);
    engine.register_native_function("Actor", "IsDead", actor::is_dead);
    engine.register_native_function("Actor", "IsDetectedBy", actor::is_detected_by);
    engine.register_native_function("Actor", "IsDismembered", actor::is_dismembered);
    engine.register_native_function("Actor", "IsDoingFavor", actor::is_doing_favor);
    engine.register_native_function("Actor", "IsEquipped", actor::is_equipped);
    engine.register_native_function("Actor", "IsEssential", actor::is_essential);
    engine.register_native_function("Actor", "IsFlying", actor::is_flying);
    engine.register_native_function("Actor", "IsGhost", actor::is_ghost);
    engine.register_native_function("Actor", "IsGuard", actor::is_guard);
    engine.register_native_function("Actor", "RemovePerk", actor::remove_perk);
    engine.register_native_function("Actor", "SetNoBleedoutRecovery", actor::set_no_bleedout_recovery);
    engine.register_native_function("Actor", "UnequipItem", actor::unequip_item);

    engine.register_native_function("Utility", "GetCurrentGameTime", utility::get_current_game_time);
    engine.register_native_function("Utility", "RandomFloat", utility::random_float);
    engine.register_native_function("Utility", "RandomInt", utility::random_int);
    engine.register_native_function("Utility", "Wait", utility::wait);
    engine.register_native_function("Utility", "WaitMenuMode", utility::wait_menu_mode);

    engine.register_native_function("Sound", "SetInstanceVolume", sound::set_instance_volume);
    engine.register_native_function("Sound", "StopInstance", sound::stop_instance);
    engine.register_native_function("Sound", "Play", sound::play);
    engine.register_native_function("Sound", "PlayAndWait", sound::play_and_wait);

    engine.register_native_function("Spell", "Cast", spell::cast);
    engine.register_native_function("Spell", "IsHostile", spell::is_hostile);
    engine.register_native_function("Spell", "RemoteCast", spell::remote_cast);

    engine.register_native_function("GlobalVariable", "GetValue", global_variable::get_value);
    engine.register_native_function("GlobalVariable", "SetValue", global_variable::set_value);

    engine.register_native_function("Keyword", "SendStoryEvent", keyword::send_story_event);
    engine.register_native_function("Keyword", "SendStoryEventAndWait", keyword::send_story_event_and_wait);

    engine.register_native_function("RefCollectionAlias", "GetAt", ref_collection_alias::get_at);
    engine.register_native_function("RefCollectionAlias", "GetCount", ref_collection_alias::get_count);

    engine.register_native_function("ReferenceAlias", "GetReference", reference_alias::get_reference_alias);

    engine.register_native_function("ActiveMagicEffect", "Dispel", active_magic_effect::dispel);

    engine.register_native_function("MagicEffect", "GetAssociatedSkill", magic_effect::get_assocaited_skill);

    engine.register_native_function("Potion", "IsHostile", potion::is_hostile);

    engine.register_native_function("FormList", "AddForm", form_list::add_form);
    engine.register_native_function("FormList", "Find", form_list::find);
    engine.register_native_function("FormList", "GetAt", form_list::get_at);
    engine.register_native_function("FormList", "GetSize", form_list::get_size);
    engine.register_native_function("FormList", "HasForm", form_list::has_form);
    engine.register_native_function("FormList", "RemoveAddedForm", form_list::remove_added_form);
    engine.register_native_function("FormList", "Revert", form_list::revert);

    engine.register_native_function("Faction", "CanPayCrimeGold", faction::can_pay_crime_gold);

    engine.register_native_function("Quest", "CompleteAllObjectives", quest::complete_all_objectives);
    engine.register_native_function("Quest", "CompleteQuest", quest::complete_quest);
    engine.register_native_function("Quest", "FailAllObjectives", quest::fail_all_objectives);
    engine.register_native_function("Quest", "GetAlias", quest::get_alias);
    engine.register_native_function("Quest", "GetCurrentStageID", quest::get_current_stage_id);
    engine.register_native_function("Quest", "HasObjective", quest::has_objective);
    engine.register_native_function("Quest", "IsActive", quest::is_active);
    engine.register_native_function("Quest", "IsCompleted", quest::is_completed);
    engine.register_native_function("Quest", "IsObjectiveCompleted", quest::is_objective_completed);
    engine.register_native_function("Quest", "IsObjectiveDisplayed", quest::is_objective_displayed);
    engine.register_native_function("Quest", "IsObjectiveFailed", quest::is_objective_failed);
    engine.register_native_function("Quest", "IsRunning", quest::is_running);
    engine.register_native_function("Quest", "IsStageDone", quest::is_stage_done);
    engine.register_native_function("Quest", "IsStarting", quest::is_starting);
    engine.register_native_function("Quest", "IsStopping", quest::is_stopping);
    engine.register_native_function("Quest", "IsStopped", quest::is_stopped);
    engine.register_native_function("Quest", "Reset", quest::reset);
    engine.register_native_function("Quest", "ResetSpeechChallenges", quest::reset_speech_challenges);
    engine.register_native_function("Quest", "SetActive", quest::set_active);
    engine.register_native_function("Quest", "SetCurrentStageID", quest::set_current_stage_id);
    engine.register_native_function("Quest", "SetObjectiveCompleted", quest::set_objective_completed);
    engine.register_native_function("Quest", "SetObjectiveDisplayed", quest::set_objective_displayed);
    engine.register_native_function("Quest", "SetObjectiveFailed", quest::set_objective_failed);
    engine.register_native_function("Quest", "Start", quest::start);
    engine.register_native_function("Quest", "stop", quest::stop);
    engine.register_native_function("Quest", "UpdateCurrentInstanceGlobal", quest::update_current_instance_global);
}

pub struct State {
    pub player: ObjectRef,
    pub actor_values: CoreActorValues,
    pub provider: Box<dyn StateProvider + 'static>,
    pub real_timers: Mutex<TimerSet>,
    pub game_timers: Mutex<TimerSet>,
}

impl State {}

#[derive(Debug)]
pub struct CoreActorValues {
    pub health: ObjectRef,
    // SPECIAL
    pub strength: ObjectRef,
    pub perception: ObjectRef,
    pub endurance: ObjectRef,
    pub charisma: ObjectRef,
    pub intelligence: ObjectRef,
    pub agility: ObjectRef,
    pub luck: ObjectRef,
    // AI
    pub aggression: ObjectRef,
    pub confidence: ObjectRef,
    pub suspicious: ObjectRef,
}

pub struct ObjectData {
    timers: HashMap<Int, Vec<Timer>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimerId(Int);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Timer {
    RealTime(Duration),
    GameTime(Duration),
}

pub trait StateProvider: Send + Sync + 'static {
    fn get_global(&self, global: ObjectAddr) -> Option<f32>;
    fn set_global(&self, global: ObjectAddr, value: f32);

    fn get_actor_value(&self, object: ObjectAddr, actor_value: ObjectAddr) -> Option<f32>;
    fn set_actor_value(&self, object: ObjectAddr, actor_value: ObjectAddr, value: f32);

    fn get_owner(&self, object: ObjectAddr) -> ObjectAddr;
    fn set_owner(&self, object: ObjectAddr, owner: ObjectAddr);

    fn linked_ref(&self, object: ObjectAddr, keyword: ObjectAddr) -> Option<ObjectAddr>;

    fn start_quest(&self, quest: ObjectAddr);
    fn stop_quest(&self, quest: ObjectAddr);
}

#[derive(Clone, Debug)]
pub struct TimerSet {
    now: Instant,
    entries: Slab<TimerEntry>,
    next: BTreeMap<Instant, usize>,
    objects: HashMap<Object, HashMap<TimerId, usize>>,
}

impl TimerSet {
    pub fn insert(&mut self, object: Object, id: TimerId, duration: Duration) {
        self.remove(object, id);

        let mut at = self.now + duration;
        // Find next slot that is not empty.
        // FIXME: This pushes tasks slightly back, and is O(n),
        // can we do it in O(log(n)) instead?
        while self.next.contains_key(&at) {
            at += Duration::from_nanos(1);
        }

        let index = self.entries.insert(TimerEntry { object, id, at });
        self.next.insert(at, index);
        self.objects.entry(object).or_default().insert(id, index);
    }

    pub fn remove(&mut self, object_id: Object, id: TimerId) {
        let Some(object) = self.objects.get_mut(&object_id) else {
            return;
        };

        let Some(index) = object.remove(&id) else {
            return;
        };

        let entry = self.entries.remove(index);
        self.next.remove(&entry.at);

        if object.is_empty() {
            self.objects.remove(&object_id);
        }
    }

    pub fn tick(&mut self, time: Duration) -> impl Iterator<Item = TimerEntry> {
        self.now += time;

        let mut timers = Vec::new();
        for (_, index) in self.next.extract_if(..self.now, |_, _| true) {
            let entry = &self.entries[index];
            timers.push(*entry);
        }

        for e in &timers {
            self.remove(e.object, e.id);
        }

        timers.into_iter()
    }
}

impl Default for TimerSet {
    fn default() -> Self {
        Self {
            now: Instant::now(),
            entries: Slab::new(),
            next: BTreeMap::new(),
            objects: HashMap::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TimerEntry {
    pub object: ObjectAddr,
    pub id: TimerId,
    pub at: Instant,
}
