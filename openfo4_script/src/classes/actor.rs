use crate::classes::{Bool, Int, Object, State};
use crate::runtime::Caller;

/// `AddPerk`
///
/// ```text
/// Function AddPerk(Perk akPerk, bool abNotify = false) native
/// ```
pub fn add_perk(caller: Caller<'_, State>, self_ptr: Object, perk: Object, notify: bool) {
    todo!()
}

/// `AddSpell`
///
/// ```text
/// bool Function AddSpell(Spell akSpell, bool abVerbose = true) native
/// ```
pub fn add_spell(caller: Caller<'_, State>, self_ptr: Object, verbose: Bool) {
    todo!()
}

/// `AllowBleedoutDialogue`
///
/// ```text
/// Function AllowBleedoutDialogue(bool abCanTalk) native
/// ```
pub fn allow_bleedout_dialogue(caller: Caller<'_, State>, self_ptr: Object, can_talk: Bool) {
    todo!()
}

/// `AllowPCDialogue`
///
/// ```text
/// Function AllowPCDialogue(bool abTalk) native
/// ```
pub fn allow_pc_dialogue(caller: Caller<'_, State>, self_ptr: Object, talk: Bool) {
    todo!()
}

/// `AttachAshPile`
///
/// ```text
/// Function AttachAshPile(Form akAshPileBase = None) native
/// ```
pub fn attach_ash_pile(caller: Caller<'_, State>, self_ptr: Object, ash_pile_form: Object) {
    todo!()
}

/// `AttemptAnimationSetSwitch`
///
/// ```text
/// Function AttemptAnimationSetSwitch() native
/// ```
pub fn attempt_animation_set_switch(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `CanFlyHere`
///
/// ```text
/// bool Function CanFlyHere() native
/// ```
pub fn can_fly_here(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `ChangeAnimArchetype`
///
/// ```text
/// bool Function ChangeAnimArchetype(keyword apKeyword = none) native
/// ```
pub fn change_anim_archetype(caller: Caller<'_, State>, self_ptr: Object, keyword: Object) {
    todo!()
}

/// `ChangeAnimFaceArchetype`
///
/// ```text
/// Function ChangeAnimFaceArchetype(keyword apKeyword = none) native
/// ```
pub fn change_anim_face_archetype(caller: Caller<'_, State>, self_ptr: Object, keyword: Object) {
    todo!()
}

/// `ChangeAnimFlavor`
///
/// ```text
/// bool Function ChangeAnimFlavor(keyword apKeyword = none) native
/// ```
pub fn change_anim_flavor(caller: Caller<'_, State>, self_ptr: Object, keyword: Object) -> Bool {
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
) {
    todo!()
}

/// `ClearArrested`
///
/// ```text
/// Function ClearArrested() native
/// ```
pub fn clear_arrested(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `ClearExpressionOverride`
///
/// ```text
/// Function ClearExpressionOverride() native
/// ```
pub fn clear_expression_override(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `ClearExtraArrows`
///
/// ```text
/// Function ClearExtraArrows() native
/// ```
pub fn clear_extra_arrows(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `ClearLookAt`
///
/// ```text
/// Function ClearLookAt() native
/// ```
pub fn clear_look_at(caller: Caller<'_, State>, self_ptr: Object) {
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
) {
    todo!()
}

/// `Dismount`
///
/// ```text
/// bool Function Dismount() native
/// ```
pub fn dismount(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `DispelAllSpells`
///
/// ```text
/// Function DispelAllSpells() native
/// ```
pub fn dispel_all_spells(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `DispelSpell`
///
/// ```text
/// bool Function DispelSpell(Spell akSpell) native
/// ```
pub fn dispel_spell(caller: Caller<'_, State>, self_ptr: Object, spell: Object) {
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
) {
    todo!()
}

/// `DogDropItems`
///
/// ```text
/// Function DogDropItems() native
/// ```
pub fn dog_drop_items(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `DogPlaceInMouth`
///
/// ```text
/// Function DogPlaceInMouth(Form akItem) native
/// ```
pub fn dog_place_in_mouth(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `DrawWeapon`
///
/// ```text
/// Function DrawWeapon() native
/// ```
pub fn draw_weapon(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `EnableAI`
///
/// ```text
/// Function EnableAI(bool abEnable = true, bool abPauseVoice = false) native
/// ```
pub fn enable_ai(caller: Caller<'_, State>, enable: Bool, pause_voice: Bool) {
    todo!()
}

/// `EndDeferredKill`
///
/// ```text
/// Function EndDeferredKill() native
/// ```
pub fn end_deferred_kill(caller: Caller<'_, State>, self_ptr: Object) {
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
) {
    todo!()
}

/// `EquipSpell`
///
/// ```text
/// Function EquipSpell(Spell akSpell, int aiSource) native
/// ```
pub fn equip_spell(caller: Caller<'_, State>, self_ptr: Object, spell: Object, source: Int) {
    todo!()
}

/// `EvaluatePackage`
///
/// ```text
/// Function EvaluatePackage(bool abResetAI=false) native
/// ```
pub fn evaluate_package(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `GetAllCombatTargets`
///
/// ```text
/// Actor[] Function GetAllCombatTargets() native
/// ```
pub fn get_all_combat_targets(caller: Caller<'_, State>, self_ptr: Object) -> isize {
    todo!()
}

/// `GetBribeAmount`
///
/// ```text
/// int Function GetBribeAmount() native
/// ```
pub fn get_bribe_amount(caller: Caller<'_, State>, self_ptr: Object) {
    todo!()
}

/// `GetCrimeFaction`
///
/// ```text
/// Faction Function GetCrimeFaction() native
/// ```
pub fn get_crime_faction(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetCombatState`
///
/// ```text
/// int Function GetCombatState() native
/// ```
pub fn get_combat_state(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetCombatTarget`
///
/// ```text
/// Actor Function GetCombatTarget() native
/// ```
pub fn get_combat_target(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetCurrentPackage`
///
/// ```text
/// Package Function GetCurrentPackage() native
/// ```
pub fn get_current_package(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetDialogueTarget`
///
/// ```text
/// Actor Function GetDialogueTarget() native
/// ```
pub fn get_dialogue_target(caller: Caller<'_, State>, self_ptr: Object) -> Object {
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
) -> Int {
    todo!()
}

/// `GetEquippedShield`
///
/// ```text
/// Armor Function GetEquippedShield() native
/// ```
pub fn get_equipped_shield(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetEquippedSpell`
///
/// ```text
/// Spell Function GetEquippedSpell(int aiSource) native
/// ```
pub fn get_equipped_spell(caller: Caller<'_, State>, self_ptr: Object, source: Int) -> Object {
    todo!()
}

/// `GetEquippedWeapon`
///
/// ```text
/// Weapon Function GetEquippedWeapon(int aiEquipIndex = 0) native
/// ```
pub fn get_equipped_weapon(caller: Caller<'_, State>, self_ptr: Object, equip_index: Int) -> Int {
    todo!()
}

/// `GetFactionRank`
///
/// ```text
/// int Function GetFactionRank(Faction akFaction) native
/// ```
pub fn get_faction_rank(caller: Caller<'_, State>, self_ptr: Object, faction: Object) -> Int {
    todo!()
}

/// `GetFactionReaction`
///
/// ```text
/// int Function GetFactionReaction(Actor akOther) native
/// ```
pub fn get_faction_reaction(caller: Caller<'_, State>, self_ptr: Object, other: Object) -> Int {
    todo!()
}

/// `GetFlyingState`
///
/// ```text
/// int Function GetFlyingState() native
/// ```
pub fn get_flying_state(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetForcedLandingMarker`
///
/// ```text
/// ObjectReference Function GetForcedLandingMarker() native
/// ```
pub fn get_forced_landing_marker(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetGoldAmount`
///
/// ```text
/// int Function GetGoldAmount() native
/// ```
pub fn get_gold_amount(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetHighestRelationshipRank`
///
/// ```text
/// int Function GetHighestRelationshipRank() native
/// ```
pub fn get_highest_relationship_rank(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetKiller`
///
/// ```text
/// Actor Function GetKiller() native
/// ```
pub fn get_killer(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetLevel`
///
/// ```text
/// int Function GetLevel() native
/// ```
pub fn get_level(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetLeveledActorBase`
///
/// ```text
/// ActorBase Function GetLeveledActorBase() native
/// ```
pub fn get_leveled_actor_base(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetLightLevel`
///
/// ```text
/// float Function GetLightLevel() native
/// ```
pub fn get_light_level(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetLowestRelationshipRank`
///
/// ```text
/// int Function GetLowestRelationshipRank() native
/// ```
pub fn get_lowest_relationship_rank(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetNoBleedoutRecovery`
///
/// ```text
/// bool Function GetNoBleedoutRecovery() native
/// ```
pub fn get_no_bleedout_recovery(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `GetPlayerControls`
///
/// ```text
/// bool Function GetPlayerControls() native
/// ```
pub fn get_player_controls(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `GetRace`
///
/// ```text
/// Race Function GetRace() native
/// ```
pub fn get_race(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}

/// `GetRelationshipRank`
///
/// ```text
/// int Function GetRelationshipRank(Actor akOther) native
/// ```
pub fn get_relationship_rank(caller: Caller<'_, State>, self_ptr: Object, other: Object) -> Int {
    todo!()
}

/// `GetSitState`
///
/// ```text
/// int Function GetSitState() native
/// ```
pub fn get_sit_state(caller: Caller<'_, State>, self_ptr: Object) -> Int {
    todo!()
}

/// `GetSleepState`
///
/// ```text
/// int Function GetSleepState() native
/// ```
pub fn get_sleep_state(caller: Caller<'_, State>, self_ptr: Object) -> Int {
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
) -> Bool {
    todo!()
}

/// `HasDetectionLOS`
///
/// ```text
/// bool Function HasDetectionLOS(ObjectReference akOther) native
/// ```
pub fn has_detection_los(caller: Caller<'_, State>, self_ptr: Object, other: Object) -> Bool {
    todo!()
}

/// `HasFamilyRelationship`
///
/// ```text
/// bool Function HasFamilyRelationship(Actor akOther = None) native
/// ```
pub fn has_family_relationship(caller: Caller<'_, State>, self_ptr: Object, other: Object) -> Bool {
    todo!()
}

/// `HasMagicEffect`
///
/// ```text
/// bool Function HasMagicEffect(MagicEffect akEffect) native
/// ```
pub fn has_magic_effect(caller: Caller<'_, State>, self_ptr: Object, other: Object) -> Bool {
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
) -> Bool {
    todo!()
}

/// `HasParentRelationship`
///
/// ```text
/// bool Function HasParentRelationship(Actor akOther) native
/// ```
pub fn has_parent_relationship(caller: Caller<'_, State>, self_ptr: Object, other: Object) -> Bool {
    todo!()
}

/// `HasPerk`
///
/// ```text
/// bool Function HasPerk(Perk akPerk) native
/// ```
pub fn has_perk(caller: Caller<'_, State>, self_ptr: Object, perk: Object) -> Bool {
    todo!()
}

/// `HasSpell`
///
/// ```text
/// bool Function HasSpell(Form akForm) native
/// ```
pub fn has_spell(caller: Caller<'_, State>, self_ptr: Object, spell: Object) -> Bool {
    todo!()
}

/// `IsAIEnabled`
///
/// ```text
/// bool Function IsAIEnabled() native
/// ```
pub fn is_ai_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsAlarmed`
///
/// ```text
/// bool Function IsAlarmed() native
/// ```
pub fn is_alarmed(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsAlerted`
///
/// ```text
/// bool Function IsAlerted() native
/// ```
pub fn is_alerted(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsAllowedToFly`
///
/// ```text
/// bool Function IsAllowedToFly() native
/// ```
pub fn is_allowed_to_fly(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsArrested`
///
/// ```text
/// bool Function IsArrested() native
/// ```
pub fn is_arrested(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsArrestingTarget`
///
/// ```text
/// bool Function IsArrestingTarget() native
/// ```
pub fn is_arresting_target(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsBeingRidden`
///
/// ```text
/// bool Function IsBeingRidden() native
/// ```
pub fn is_being_ridden(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsBeingRiddenBy`
///
/// ```text
/// bool Function IsBeingRiddenBy(Actor akActor) native
/// ```
pub fn is_being_ridden_by(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsBleedingOut`
///
/// ```text
/// bool Function IsBleedingOut() native
/// ```
pub fn is_bleeding_out(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsBribed`
///
/// ```text
/// bool Function IsBribed() native
/// ```
pub fn is_bribed(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsChild`
///
/// ```text
/// bool Function IsChild() native
/// ```
pub fn is_child(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsCommandedActor`
///
/// ```text
/// bool Function IsCommandedActor() native
/// ```
pub fn is_commanded_actor(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsDead`
///
/// ```text
/// bool Function IsDead() native
/// ```
pub fn is_dead(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsDetectedBy`
///
/// ```text
/// bool Function IsDetectedBy(Actor akOther) native
/// ```
pub fn is_detected_by(caller: Caller<'_, State>, other: Object) -> Bool {
    todo!()
}

/// `IsDismembered`
///
/// ```text
/// bool Function IsDismembered(string asBodyPart = "") native
/// ```
pub fn is_dismembered(caller: Caller<'_, State>, body_part: isize) -> Bool {
    todo!()
}

/// `IsDoingFavor`
///
/// ```text
/// bool Function IsDoingFavor() native
/// ```
pub fn is_doing_favor(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsEquipped`
///
/// ```text
/// bool Function IsEquipped(Form akItem) native
/// ```
pub fn is_equipped(caller: Caller<'_, State>, self_ptr: Object, form: Object) -> Bool {
    todo!()
}

/// `IsEssential`
///
/// ```text
/// bool Function IsEssential() native
/// ```
pub fn is_essential(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsFlying`
///
/// ```text
/// bool Function IsFlying() native
/// ```
pub fn is_flying(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsGhost`
///
/// ```text
/// bool Function IsGhost() native
/// ```
pub fn is_ghost(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}

/// `IsGuard`
///
/// ```text
/// bool Function IsGuard() native
/// ```
pub fn is_guard(caller: Caller<'_, State>, self_ptr: Object) -> Bool {
    todo!()
}
