use openfo4_script::runtime::{Caller, Exception};

use super::{Bool, Int, Object, State};

/// `AddPerk`
///
/// ```text
/// Function AddPerk(Perk akPerk, bool abNotify = false) native
/// ```
pub fn add_perk(
    caller: Caller<'_, State>,
    self_ptr: Object,
    perk: Object,
    notify: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `AddSpell`
///
/// ```text
/// bool Function AddSpell(Spell akSpell, bool abVerbose = true) native
/// ```
pub fn add_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    spell: Object,
    verbose: Bool,
) -> Result<Bool, Exception> {
    todo!()
}

/// `AllowBleedoutDialogue`
///
/// ```text
/// Function AllowBleedoutDialogue(bool abCanTalk) native
/// ```
pub fn allow_bleedout_dialogue(
    caller: Caller<'_, State>,
    self_ptr: Object,
    can_talk: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `AllowPCDialogue`
///
/// ```text
/// Function AllowPCDialogue(bool abTalk) native
/// ```
pub fn allow_pc_dialogue(
    caller: Caller<'_, State>,
    self_ptr: Object,
    talk: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `AttachAshPile`
///
/// ```text
/// Function AttachAshPile(Form akAshPileBase = None) native
/// ```
pub fn attach_ash_pile(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ash_pile_form: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `AttemptAnimationSetSwitch`
///
/// ```text
/// Function AttemptAnimationSetSwitch() native
/// ```
pub fn attempt_animation_set_switch(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `CanFlyHere`
///
/// ```text
/// bool Function CanFlyHere() native
/// ```
pub fn can_fly_here(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `ChangeAnimArchetype`
///
/// ```text
/// bool Function ChangeAnimArchetype(keyword apKeyword = none) native
/// ```
pub fn change_anim_archetype(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `ChangeAnimFaceArchetype`
///
/// ```text
/// Function ChangeAnimFaceArchetype(keyword apKeyword = none) native
/// ```
pub fn change_anim_face_archetype(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `ChangeAnimFlavor`
///
/// ```text
/// bool Function ChangeAnimFlavor(keyword apKeyword = none) native
/// ```
pub fn change_anim_flavor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `ChangeHeadPart`
///
/// ```text
/// Function ChangeHeadPart(headpart apHeadPart, bool abRemovePart = false, bool abRemoveExtraParts = false) native
/// ```
pub fn change_head_part(
    caller: Caller<'_, State>,
    self_ptr: Object,
    head_part: Object,
    remove: Bool,
    remove_extra_parts: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `ClearArrested`
///
/// ```text
/// Function ClearArrested() native
/// ```
pub fn clear_arrested(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ClearExpressionOverride`
///
/// ```text
/// Function ClearExpressionOverride() native
/// ```
pub fn clear_expression_override(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `ClearExtraArrows`
///
/// ```text
/// Function ClearExtraArrows() native
/// ```
pub fn clear_extra_arrows(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ClearLookAt`
///
/// ```text
/// Function ClearLookAt() native
/// ```
pub fn clear_look_at(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Dismember`
///
/// ```text
/// Function Dismember(string asBodyPart, bool abForceExplode = false, bool abForceDismember = false, bool abForceBloodyMess = false) native
/// ```
pub fn dismember(
    caller: Caller<'_, State>,
    self_ptr: Object,
    body_part: isize,
    force_explode: Bool,
    force_dismember: Bool,
    force_bloody_mess: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `Dismount`
///
/// ```text
/// bool Function Dismount() native
/// ```
pub fn dismount(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DispelAllSpells`
///
/// ```text
/// Function DispelAllSpells() native
/// ```
pub fn dispel_all_spells(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DispelSpell`
///
/// ```text
/// bool Function DispelSpell(Spell akSpell) native
/// ```
pub fn dispel_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    spell: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `DoCombatSpellApply`
///
/// ```text
/// Function DoCombatSpellApply(Spell akSpell, ObjectReference akTarget) native
/// ```
pub fn do_combat_spell_apply(
    caller: Caller<'_, State>,
    self_ptr: Object,
    spell: Object,
    target: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `DogDropItems`
///
/// ```text
/// Function DogDropItems() native
/// ```
pub fn dog_drop_items(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DogPlaceInMouth`
///
/// ```text
/// Function DogPlaceInMouth(Form akItem) native
/// ```
pub fn dog_place_in_mouth(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DrawWeapon`
///
/// ```text
/// Function DrawWeapon() native
/// ```
pub fn draw_weapon(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `EnableAI`
///
/// ```text
/// Function EnableAI(bool abEnable = true, bool abPauseVoice = false) native
/// ```
pub fn enable_ai(
    caller: Caller<'_, State>,
    enable: Bool,
    pause_voice: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EndDeferredKill`
///
/// ```text
/// Function EndDeferredKill() native
/// ```
pub fn end_deferred_kill(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `EquipItem`
///
/// ```text
/// Function EquipItem(Form akItem, bool abPreventRemoval = false, bool abSilent = false) native
/// ```
pub fn equip_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
    prevent_removal: Bool,
    silent: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EquipSpell`
///
/// ```text
/// Function EquipSpell(Spell akSpell, int aiSource) native
/// ```
pub fn equip_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    spell: Object,
    source: Int,
) -> Result<(), Exception> {
    todo!()
}

/// `EvaluatePackage`
///
/// ```text
/// Function EvaluatePackage(bool abResetAI=false) native
/// ```
pub fn evaluate_package(
    caller: Caller<'_, State>,
    self_ptr: Object,
    reset_ai: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `GetAllCombatTargets`
///
/// ```text
/// Actor[] Function GetAllCombatTargets() native
/// ```
pub fn get_all_combat_targets(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetBribeAmount`
///
/// ```text
/// int Function GetBribeAmount() native
/// ```
pub fn get_bribe_amount(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `GetCrimeFaction`
///
/// ```text
/// Faction Function GetCrimeFaction() native
/// ```
pub fn get_crime_faction(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCombatState`
///
/// ```text
/// int Function GetCombatState() native
/// ```
pub fn get_combat_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetCombatTarget`
///
/// ```text
/// Actor Function GetCombatTarget() native
/// ```
pub fn get_combat_target(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCurrentPackage`
///
/// ```text
/// Package Function GetCurrentPackage() native
/// ```
pub fn get_current_package(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetDialogueTarget`
///
/// ```text
/// Actor Function GetDialogueTarget() native
/// ```
pub fn get_dialogue_target(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetEquippedItemType`
///
/// ```text
/// int Function GetEquippedItemType(int aiEquipIndex) native
/// ```
pub fn get_equipped_item_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    equip_index: Int,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetEquippedShield`
///
/// ```text
/// Armor Function GetEquippedShield() native
/// ```
pub fn get_equipped_shield(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetEquippedSpell`
///
/// ```text
/// Spell Function GetEquippedSpell(int aiSource) native
/// ```
pub fn get_equipped_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    source: Int,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetEquippedWeapon`
///
/// ```text
/// Weapon Function GetEquippedWeapon(int aiEquipIndex = 0) native
/// ```
pub fn get_equipped_weapon(
    caller: Caller<'_, State>,
    self_ptr: Object,
    equip_index: Int,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetFactionRank`
///
/// ```text
/// int Function GetFactionRank(Faction akFaction) native
/// ```
pub fn get_faction_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetFactionReaction`
///
/// ```text
/// int Function GetFactionReaction(Actor akOther) native
/// ```
pub fn get_faction_reaction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetFlyingState`
///
/// ```text
/// int Function GetFlyingState() native
/// ```
pub fn get_flying_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetForcedLandingMarker`
///
/// ```text
/// ObjectReference Function GetForcedLandingMarker() native
/// ```
pub fn get_forced_landing_marker(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetGoldAmount`
///
/// ```text
/// int Function GetGoldAmount() native
/// ```
pub fn get_gold_amount(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetHighestRelationshipRank`
///
/// ```text
/// int Function GetHighestRelationshipRank() native
/// ```
pub fn get_highest_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetKiller`
///
/// ```text
/// Actor Function GetKiller() native
/// ```
pub fn get_killer(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetLevel`
///
/// ```text
/// int Function GetLevel() native
/// ```
pub fn get_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetLeveledActorBase`
///
/// ```text
/// ActorBase Function GetLeveledActorBase() native
/// ```
pub fn get_leveled_actor_base(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetLightLevel`
///
/// ```text
/// float Function GetLightLevel() native
/// ```
pub fn get_light_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetLowestRelationshipRank`
///
/// ```text
/// int Function GetLowestRelationshipRank() native
/// ```
pub fn get_lowest_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetNoBleedoutRecovery`
///
/// ```text
/// bool Function GetNoBleedoutRecovery() native
/// ```
pub fn get_no_bleedout_recovery(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `GetPlayerControls`
///
/// ```text
/// bool Function GetPlayerControls() native
/// ```
pub fn get_player_controls(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `GetRace`
///
/// ```text
/// Race Function GetRace() native
/// ```
pub fn get_race(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetRelationshipRank`
///
/// ```text
/// int Function GetRelationshipRank(Actor akOther) native
/// ```
pub fn get_relationship_rank(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetSitState`
///
/// ```text
/// int Function GetSitState() native
/// ```
pub fn get_sit_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetSleepState`
///
/// ```text
/// int Function GetSleepState() native
/// ```
pub fn get_sleep_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `HasAssociation`
///
/// ```text
/// bool Function HasAssociation(AssociationType akAssociation, Actor akOther = None) native
/// ```
pub fn has_association(
    caller: Caller<'_, State>,
    self_ptr: Object,
    assocation: Object,
    other: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasDetectionLOS`
///
/// ```text
/// bool Function HasDetectionLOS(ObjectReference akOther) native
/// ```
pub fn has_detection_los(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasFamilyRelationship`
///
/// ```text
/// bool Function HasFamilyRelationship(Actor akOther = None) native
/// ```
pub fn has_family_relationship(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasMagicEffect`
///
/// ```text
/// bool Function HasMagicEffect(MagicEffect akEffect) native
/// ```
pub fn has_magic_effect(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasMagicEffectWithKeyword`
///
/// ```text
/// bool Function HasMagicEffectWithKeyword(Keyword akKeyword) native
/// ```
pub fn has_magic_effect_with_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasParentRelationship`
///
/// ```text
/// bool Function HasParentRelationship(Actor akOther) native
/// ```
pub fn has_parent_relationship(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasPerk`
///
/// ```text
/// bool Function HasPerk(Perk akPerk) native
/// ```
pub fn has_perk(
    caller: Caller<'_, State>,
    self_ptr: Object,
    perk: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasSpell`
///
/// ```text
/// bool Function HasSpell(Form akForm) native
/// ```
pub fn has_spell(
    caller: Caller<'_, State>,
    self_ptr: Object,
    spell: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `IsAIEnabled`
///
/// ```text
/// bool Function IsAIEnabled() native
/// ```
pub fn is_ai_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsAlarmed`
///
/// ```text
/// bool Function IsAlarmed() native
/// ```
pub fn is_alarmed(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsAlerted`
///
/// ```text
/// bool Function IsAlerted() native
/// ```
pub fn is_alerted(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsAllowedToFly`
///
/// ```text
/// bool Function IsAllowedToFly() native
/// ```
pub fn is_allowed_to_fly(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsArrested`
///
/// ```text
/// bool Function IsArrested() native
/// ```
pub fn is_arrested(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsArrestingTarget`
///
/// ```text
/// bool Function IsArrestingTarget() native
/// ```
pub fn is_arresting_target(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsBeingRidden`
///
/// ```text
/// bool Function IsBeingRidden() native
/// ```
pub fn is_being_ridden(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsBeingRiddenBy`
///
/// ```text
/// bool Function IsBeingRiddenBy(Actor akActor) native
/// ```
pub fn is_being_ridden_by(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsBleedingOut`
///
/// ```text
/// bool Function IsBleedingOut() native
/// ```
pub fn is_bleeding_out(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsBribed`
///
/// ```text
/// bool Function IsBribed() native
/// ```
pub fn is_bribed(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsChild`
///
/// ```text
/// bool Function IsChild() native
/// ```
pub fn is_child(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsCommandedActor`
///
/// ```text
/// bool Function IsCommandedActor() native
/// ```
pub fn is_commanded_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsDead`
///
/// ```text
/// bool Function IsDead() native
/// ```
pub fn is_dead(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsDetectedBy`
///
/// ```text
/// bool Function IsDetectedBy(Actor akOther) native
/// ```
pub fn is_detected_by(caller: Caller<'_, State>, other: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsDismembered`
///
/// ```text
/// bool Function IsDismembered(string asBodyPart = "") native
/// ```
pub fn is_dismembered(caller: Caller<'_, State>, body_part: isize) -> Result<Bool, Exception> {
    todo!()
}

/// `IsDoingFavor`
///
/// ```text
/// bool Function IsDoingFavor() native
/// ```
pub fn is_doing_favor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsEquipped`
///
/// ```text
/// bool Function IsEquipped(Form akItem) native
/// ```
pub fn is_equipped(
    caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `IsEssential`
///
/// ```text
/// bool Function IsEssential() native
/// ```
pub fn is_essential(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsFlying`
///
/// ```text
/// bool Function IsFlying() native
/// ```
pub fn is_flying(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsGhost`
///
/// ```text
/// bool Function IsGhost() native
/// ```
pub fn is_ghost(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsGuard`
///
/// ```text
/// bool Function IsGuard() native
/// ```
pub fn is_guard(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `IsHostileToActor`
///
/// ```text
/// bool Function IsHostileToActor(Actor akActor) native
/// ```
pub fn is_hostile_to_actor(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsInCombat`
///
/// ```text
/// bool Function IsInCombat() native
/// ```
pub fn is_in_combat(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInFaction`
///
/// ```text
/// bool Function IsInFaction(Faction akFaction) native
/// ```
pub fn is_in_faction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    faction: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsInIronSights`
///
/// ```text
/// bool Function IsInIronSights() native
/// ```
pub fn is_in_iron_sights(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInKillMove`
///
/// ```text
/// bool Function IsInKillMove() native
/// ```
pub fn is_in_kill_move(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInScene`
///
/// ```text
/// bool Function IsInScene() native
/// ```
pub fn is_in_scene(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsIntimidated`
///
/// ```text
/// bool Function IsIntimidated() native
/// ```
pub fn is_intimidated(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsOnMount`
///
/// ```text
/// bool Function IsOnMount() native
/// ```
pub fn is_on_mount(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsOverEncumbered`
///
/// ```text
/// bool Function IsOverEncumbered() native
/// ```
pub fn is_over_encumbered(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsPlayersLastRiddenHorse`
///
/// ```text
/// bool Function IsPlayersLastRiddenHorse() native
/// ```
pub fn is_players_last_ridden_horse(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsPlayerTeammate`
///
/// ```text
/// bool Function IsPlayerTeammate() native
/// ```
pub fn is_player_teammate(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsRunning`
///
/// ```text
/// bool Function IsRunning() native
/// ```
pub fn is_running(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsSeatOccupied`
///
/// ```text
/// bool Function IsSeatOccupied(keyword apKeyword) native
/// ```
pub fn is_seat_occupied(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsSneaking`
///
/// ```text
/// bool Function IsSneaking() native
/// ```
pub fn is_sneaking(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsSprinting`
///
/// ```text
/// bool Function IsSprinting() native
/// ```
pub fn is_sprinting(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsTalking`
///
/// ```text
/// bool Function IsTalking() native
/// ```
pub fn is_talking(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsTrespassing`
///
/// ```text
/// bool Function IsTrespassing() native
/// ```
pub fn is_trespassing(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsUnconscious`
///
/// ```text
/// bool Function IsUnconscious() native
/// ```
pub fn is_unconscious(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsWeaponDrawn`
///
/// ```text
/// bool Function IsWeaponDrawn() native
/// ```
pub fn is_weapon_drawn(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `RemovePerk`
///
/// ```text
/// Function RemovePerk(Perk akPerk) native
/// ```
pub fn remove_perk(
    caller: Caller<'_, State>,
    self_ptr: Object,
    perk: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetNoBleedoutRecovery`
///
/// ```text
/// Function SetNoBleedoutRecovery(bool abAllowed) native
/// ```
pub fn set_no_bleedout_recovery(
    caller: Caller<'_, State>,
    self_ptr: Object,
    allowed: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `UnequipItem`
///
/// ```text
/// Function UnequipItem(Form akItem, bool abPreventEquip = false, bool abSilent = false) native
/// ```
pub fn unequip_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    prevent_equip: bool,
    silent: bool,
) -> Result<(), Exception> {
    todo!()
}
