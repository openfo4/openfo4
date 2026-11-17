; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Actor extends ObjectReference

none Function AddPerk(Perk akPerk, bool abNotify) native

bool Function AddSpell(Spell akSpell, bool abVerbose) native

none Function AllowBleedoutDialogue(bool abCanTalk) native

none Function AllowPCDialogue(bool abTalk) native

none Function AttachAshPile(Form akAshPileBase) native

none Function AttemptAnimationSetSwitch() native

bool Function CanFlyHere() native

bool Function ChangeAnimArchetype(keyword apKeyword) native

bool Function ChangeAnimFlavor(keyword apKeyword) native

none Function ChangeHeadPart(headpart apHeadPart, bool abRemovePart, bool abRemoveExtraParts) native

none Function ClearArrested() native

none Function ClearExpressionOverride() native

none Function ClearExtraArrows() native

none Function ClearLookAt() native

bool Function Dismount() native

none Function DispelAllSpells() native

bool Function DispelSpell(Spell akSpell) native

none Function DoCombatSpellApply(Spell akSpell, ObjectReference akTarget) native

none Function EnableAI(bool abEnable, bool abPauseVoice) native

none Function EndDeferredKill() native

none Function EquipItem(Form akItem, bool abPreventRemoval, bool abSilent) native

none Function EquipSpell(Spell akSpell, int aiSource) native

none Function EvaluatePackage(bool abResetAI) native

int Function GetBribeAmount() native

Actor[] Function GetAllCombatTargets() native

Faction Function GetCrimeFaction() native

int Function GetCombatState() native

Actor Function GetCombatTarget() native

Package Function GetCurrentPackage() native

Actor Function GetDialogueTarget() native

int Function GetEquippedItemType(int aiEquipIndex) native

Weapon Function GetEquippedWeapon(int aiEquipIndex) native

Armor Function GetEquippedShield() native

Spell Function GetEquippedSpell(int aiSource) native

int Function GetFactionRank(Faction akFaction) native

int Function GetFactionReaction(Actor akOther) native

int Function GetFlyingState() native

ObjectReference Function GetForcedLandingMarker() native

int Function GetGoldAmount() native

int Function GetHighestRelationshipRank() native

Actor Function GetKiller() native

int Function GetLevel() native

float Function GetLightLevel() native

int Function GetLowestRelationshipRank() native

ActorBase Function GetLeveledActorBase() native

bool Function GetNoBleedoutRecovery() native

bool Function GetPlayerControls() native

Race Function GetRace() native

int Function GetRelationshipRank(Actor akOther) native

int Function GetSitState() native

int Function GetSleepState() native

bool Function HasAssociation(AssociationType akAssociation, Actor akOther) native

bool Function HasFamilyRelationship(Actor akOther) native

bool Function HasDetectionLOS(ObjectReference akOther) native

bool Function HasMagicEffect(MagicEffect akEffect) native

bool Function HasMagicEffectWithKeyword(Keyword akKeyword) native

bool Function HasParentRelationship(Actor akOther) native

bool Function HasPerk(Perk akPerk) native

bool Function HasSpell(Form akForm) native

bool Function IsAIEnabled() native

bool Function IsAlarmed() native

bool Function IsAlerted() native

bool Function IsAllowedToFly() native

bool Function IsArrested() native

bool Function IsArrestingTarget() native

bool Function IsBeingRidden() native

bool Function IsBeingRiddenBy(Actor akActor) native

bool Function IsBleedingOut() native

bool Function IsBribed() native

bool Function IsChild() native

bool Function IsCommandedActor() native

bool Function IsDead() native

bool Function IsDetectedBy(Actor akOther) native

bool Function IsDismembered(string asBodyPart) native

bool Function IsDoingFavor() native

bool Function IsEquipped(Form akItem) native

bool Function IsEssential() native

bool Function IsFlying() native

bool Function IsGuard() native

bool Function IsGhost() native

bool Function IsHostileToActor(Actor akActor) native

bool Function IsInCombat() native

bool Function IsInFaction(Faction akFaction) native

bool Function IsInIronSights() native

bool Function IsInKillMove() native

bool Function IsInScene() native

bool Function IsIntimidated() native

bool Function IsOnMount() native

bool Function IsOverEncumbered() native

bool Function IsPlayersLastRiddenHorse() native

bool Function IsPlayerTeammate() native

bool Function IsRunning() native

bool Function IsSeatOccupied(keyword apKeyword) native

bool Function IsSneaking() native

bool Function IsSprinting() native

bool Function IsTalking() native

bool Function IsTrespassing() native

bool Function IsUnconscious() native

bool Function IsWeaponDrawn() native

none Function Kill(Actor akKiller) native

none Function KillSilent(Actor akKiller) native

none Function Dismember(string asBodyPart, bool abForceExplode, bool abForceDismember, bool abForceBloodyMess) native

none Function MarkItemAsFavorite(Form akItem, int aiSlot) native

none Function ModFactionRank(Faction akFaction, int aiMod) native

none Function MoveToPackageLocation() native

none Function OpenInventory(bool abForceOpen) native

bool Function PathToReference(ObjectReference aTarget, float afWalkRunPercent) native

bool Function PlayIdle(Idle akIdle) native

bool Function PlayIdleAction(Action aAction, ObjectReference aTarget) native

bool Function PlayIdleWithTarget(Idle akIdle, ObjectReference akTarget) native

none Function PlaySubGraphAnimation(string asEventName) native

none Function RemoveFromFaction(Faction akFaction) native

none Function RemoveFromAllFactions() native

none Function RemovePerk(Perk akPerk) native

bool Function RemoveSpell(Spell akSpell) native

none Function ResetHealthAndLimbs() native

none Function Resurrect() native

none Function SendAssaultAlarm() native

none Function SendTrespassAlarm(Actor akCriminal) native

none Function SetAlert(bool abAlerted) native

none Function SetAllowFlying(bool abAllowed, bool abAllowCrash, bool abAllowSearch) native

none Function SetAlpha(float afTargetAlpha, bool abFade) native

none Function SetAttackActorOnSight(bool abAttackOnSight) native

none Function SetAvoidPlayer(bool abAvoid) native

none Function SetCommandState(bool abStartCommandMode) native

none Function SetBribed(bool abBribe) native

none Function SetCanDoCommand(bool abCanCommand) native

none Function SetCombatStyle(CombatStyle akCombatStyle) native

none Function SetCrimeFaction(Faction akFaction) native

none Function SetCriticalStage(int aiStage) native

none Function SetDoingFavor(bool abDoingFavor, bool abWorkShopMode) native

none Function ChangeAnimFaceArchetype(keyword apKeyword) native

none Function SetEyeTexture(TextureSet akNewTexture) native

none Function SetEssential(bool abEssential) native

none Function SetFactionRank(Faction akFaction, int aiRank) native

none Function SetForcedLandingMarker(ObjectReference aMarker) native

none Function SetGhost(bool abIsGhost) native

none Function SetHasCharGenSkeleton(bool abCharGen) native

none Function SetHeadTracking(bool abEnable) native

none Function SetIntimidated(bool abIntimidate) native

none Function SetLookAt(ObjectReference akTarget, bool abPathingLookAt) native

none Function SetNoBleedoutRecovery(bool abAllowed) native

none Function SetNotShowOnStealthMeter(bool abNotShow) native

none Function SetOutfit(Outfit akOutfit, bool abSleepOutfit) native

none Function SetOverrideVoiceType(VoiceType akVoiceType) native

none Function SetPlayerControls(bool abControls) native

none Function SetPlayerResistingArrest() native

none Function SetPlayerTeammate(bool abTeammate, bool abCanDoFavor, bool abGivePlayerXP) native

none Function SetProtected(bool abProtected) native

none Function SetRace(Race akRace) native

none Function SetRelationshipRank(Actor akOther, int aiRank) native

bool Function SetRestrained(bool abRestrained) native

none Function SetSubGraphFloatVariable(string asVariableName, float afValue) native

bool Function SetUnconscious(bool abUnconscious) native

none Function SetVehicle(Actor akVehicle) native

none Function ShowBarterMenu() native

none Function StartCannibal(Actor akTarget) native

none Function StartCombat(Actor akTarget, bool abPreferredTarget) native

none Function StartDeferredKill() native

none Function StartVampireFeed(Actor akTarget) native

none Function StartFrenzyAttack(float aChance, float aInterval) native

none Function StopCombat() native

none Function StopCombatAlarm() native

none Function SwitchToPowerArmor(ObjectReference aArmorFurniture) native

bool Function SnapIntoInteraction(ObjectReference akTarget) native

bool Function TrapSoul(Actor akTarget) native

none Function UnequipAll() native

none Function UnequipItem(Form akItem, bool abPreventEquip, bool abSilent) native

none Function UnequipItemSlot(int aiSlot) native

none Function UnequipSpell(Spell akSpell, int aiSource) native

none Function UnLockOwnedDoorsInCell() native

bool Function WillIntimidateSucceed() native

bool Function WornHasKeyword(Keyword akKeyword) native

bool Function WouldBeStealing(ObjectReference akObject) native

int Function WouldRefuseCommand(ObjectReference akObject) native

none Function StartSneaking() native

none Function DrawWeapon() native

none Function DogPlaceInMouth(Form akItem) native

none Function DogDropItems() native

none Function ForceMovementDirection(float afXAngle, float afYAngle, float afZAngle) native

none Function ForceMovementSpeed(float afSpeedMult) native

none Function ForceMovementRotationSpeed(float afXMult, float afYMult, float afZMult) native

none Function ForceMovementDirectionRamp(float afXAngle, float afYAngle, float afZAngle, float afRampTime) native

none Function ForceMovementSpeedRamp(float afSpeedMult, float afRampTime) native

none Function ForceMovementRotationSpeedRamp(float afXMult, float afYMult, float afZMult, float afRampTime) native

none Function ForceTargetDirection(float afXAngle, float afYAngle, float afZAngle) native

none Function ForceTargetSpeed(float afSpeed) native

none Function ForceTargetAngle(float afXAngle, float afYAngle, float afZAngle) native

none Function ClearForcedMovement() native

bool Function CanMoveVertical() native

bool Function CanStrafe() native

