; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName ObjectReference extends Form

bool Function Activate(ObjectReference akActivator, bool abDefaultProcessingOnly) native

bool Function AddDependentAnimatedObjectReference(ObjectReference akDependent) native

none Function AddItem(Form akItemToAdd, int aiCount, bool abSilent) native

none Function AddKeyword(Keyword apKeyword) native

none Function AddTextReplacementData(string asTokenLabel, Form akForm) native

none Function AddToMap(bool abAllowFastTravel) native

none Function ApplyConveyorBelt(string aTarget, float aLinVelX, float aLinVelY, float aLinVelZ, bool abOn, bool abReverse) native

none Function ApplyHavokImpulse(float afX, float afY, float afZ, float afMagnitude) native

none Function ApplyFanMotor(string aTarget, float aAxisX, float aAxisY, float aAxisZ, float aForce, bool abOn) native

none Function FanMotorOn(bool abOn) native

bool Function IsFanMotorOn() native

bool Function AttachMod(ObjectMod akMod, int aiAttachIndex) native

bool Function AttachModToInventoryItem(Form akItem, ObjectMod akMod) native

none Function AttachTo(ObjectReference akParent) native

none Function BlockActivation(bool abBlocked, bool abHideActivateText) native

int Function CalculateEncounterLevel(int aiDifficulty) native

bool Function CanFastTravelToMarker() native

bool Function CanProduceForWorkshop() native

none Function ClearDestruction() native

none Function ClearFromOldLocations() native

none Function ConveyorBeltOn(bool abOn) native

int Function CountActorsLinkedToMe(Keyword apLinkKeyword, Keyword apExcludeKeyword) native

int Function CountLinkedRefChain(keyword apKeyword, int maxExpectedLinkedRefs) native

int Function CountRefsLinkedToMe(Keyword apLinkKeyword, Keyword apExcludeKeyword) native

none Function CreateDetectionEvent(Actor akOwner, int aiSoundLevel) native

none Function DamageObject(float afDamage) native

none Function DamageValue(ActorValue akAV, float afDamage) native

none Function Delete() native

none Function Disable(bool abFadeOut) native

none Function DisableLinkChain(Keyword apKeyword, bool abFadeOut) native

none Function DisableNoWait(bool abFadeOut) native

none Function Drop(bool abSilent) native

ObjectReference Function DropFirstObject(bool abInitiallyDisabled) native

ObjectReference Function DropObject(Form akObject, int aiCount) native

none Function Enable(bool abFadeIn) native

none Function EnableFastTravel(bool abEnable) native

none Function EnableLinkChain(Keyword apKeyword, bool abFadeIn) native

none Function EnableNoWait(bool abFadeIn) native

ObjectReference[] Function FindAllReferencesOfType(Form akObjectOrList, float afRadius) native

ObjectReference[] Function FindAllReferencesWithKeyword(Form akKeywordOrList, float afRadius) native

none Function ForceAddRagdollToWorld() native

none Function ForceRemoveRagdollFromWorld() native

ActorBase Function GetActorOwner() native

Actor Function GetActorRefOwner() native

Actor[] Function GetActorsLinkedToMe(Keyword apLinkKeyword, Keyword apExcludeKeyword) native

float Function GetAngleX() native

float Function GetAngleY() native

float Function GetAngleZ() native

bool Function GetAnimationVariableBool(string arVariableName) native

int Function GetAnimationVariableInt(string arVariableName) native

float Function GetAnimationVariableFloat(string arVariableName) native

Form Function GetBaseObject() native

float Function GetBaseValue(ActorValue akAV) native

int Function GetCurrentDestructionStage() native

Location Function GetCurrentLocation() native

Scene Function GetCurrentScene() native

float Function GetDistance(ObjectReference akOther) native

Location Function GetEditorLocation() native

EncounterZone Function GetEncounterZone() native

Faction Function GetFactionOwner() native

float Function GetHeadingAngle(ObjectReference akOther) native

float Function GetHeight() native

int Function GetInventoryValue() native

int Function GetItemCount(Form akItem) native

int Function GetComponentCount(Form akItem) native

ObjectReference Function GetContainer() native

float Function GetItemHealthPercent() native

Key Function GetKey() native

float Function GetLength() native

ObjectReference Function GetLinkedRef(Keyword apKeyword) native

ObjectReference[] Function GetLinkedRefChain(keyword apKeyword, int iMaxExpectedLinkedRefs) native

ObjectReference[] Function GetLinkedRefChildren(keyword apKeyword) native

int Function GetLockLevel() native

LocationRefType[] Function GetLocRefTypes() native

ObjectReference Function GetNthLinkedRef(int aiLinkedRef, Keyword apKeyword) native

float Function GetMass() native

int Function GetOpenState() native

Cell Function GetParentCell() native

float Function GetPositionX() native

float Function GetPositionY() native

float Function GetPositionZ() native

float Function GetRadioFrequency() native

float Function GetRadioVolume() native

float Function GetResourceDamage(ActorValue akValue) native

float[] Function GetSafePosition(float aSearchRadius, float aSafeRadius) native

ObjectReference[] Function GetRefsLinkedToMe(Keyword apLinkKeyword, Keyword apExcludeKeyword) native

ObjectReference[] Function GetWorkshopOwnedObjects(Actor akActor) native

float Function GetWorkshopResourceDamage(ActorValue akValue) native

ObjectReference[] Function GetWorkshopResourceObjects(ActorValue akAV, int aiOption) native

float Function GetScale() native

Cell Function GetTeleportCell() native

Cell Function GetTransitionCell() native

float Function GetTransmitterDistance() native

int Function GetTriggerObjectCount() native

float Function GetValue(ActorValue akAV) native

float Function GetValuePercentage(ActorValue akAV) native

VoiceType Function GetVoiceType() native

float Function GetWidth() native

WorldSpace Function GetWorldSpace() native

bool Function HasActorRefOwner() native

bool Function HasDirectLOS(ObjectReference akTarget, string asSourceNode, string asTargetNode) native

bool Function HasEffectKeyword(Keyword akKeyword) native

bool Function HasKeyword(Keyword apKeyword) native

bool Function HasKeywordInFormList(FormList akKeywordList) native

bool Function HasLocRefType(LocationRefType akRefType) native

bool Function HasNode(string asNodeName) native

bool Function HasSharedPowerGrid(ObjectReference akCompare) native

none Function IgnoreFriendlyHits(bool abIgnore) native

none Function InterruptCast() native

bool Function IsActivateChild(ObjectReference akChild) native

bool Function IsActivationBlocked() native

bool Function Is3DLoaded() native

bool Function IsConveyorBeltOn() native

bool Function IsCreated() native

bool Function IsDeleted() native

bool Function IsDestroyed() native

bool Function IsDisabled() native

bool Function IsFurnitureInUse(bool abIgnoreReserved) native

bool Function IsFurnitureMarkerInUse(int aiMarker, bool abIgnoreReserved) native

bool Function IsIgnoringFriendlyHits() native

bool Function IsInDialogueWithPlayer() native

bool Function IsLockBroken() native

bool Function IsLocked() native

bool Function IsMapMarkerVisible() native

bool Function IsOwnedBy(Actor akOwner) native

bool Function IsPowered() native

bool Function IsQuestItem() native

bool Function IsRadioOn() native

bool Function IsRefInTransitionCell(ObjectReference akRef) native

bool Function IsTeleportAreaLoaded() native

bool Function IsWithinBuildableArea(ObjectReference akRef) native

none Function KnockAreaEffect(float afMagnitude, float afRadius) native

none Function Lock(bool abLock, bool abAsOwner) native

none Function MakeRadioReceiver(float afFrequency, float afVolume, OutputModel aOverrideModel, bool abActive, bool abNoStatic) native

none Function MakeTransmitterRepeater(ObjectReference akTransmitterToRepeat, float afInnerRadius, float afOuterRadius, bool abUnlimitedRange) native

none Function ModValue(ActorValue akAV, float afAmount) native

none Function MoveTo(ObjectReference akTarget, float afXOffset, float afYOffset, float afZOffset, bool abMatchRotation) native

none Function MoveToMyEditorLocation() native

none Function MoveToNearestNavmeshLocation() native

none Function MoveToNode(ObjectReference akTarget, string asNodeName, string asMatchNodeName) native

Location Function OpenWorkshopSettlementMenu(Keyword akActionKW, Message astrConfirm, Location aLocToHighlight) native

Location Function OpenWorkshopSettlementMenuEx(Keyword akActionKW, Message astrConfirm, Location aLocToHighlight, FormList akIncludeKeywordList, FormList akExcludeKeywordList, bool abExcludeZeroPopulation, bool abOnlyOwnedWorkshops, bool abTurnOffHeader, bool abOnlyPotentialVassalSettlements, bool abDisableReservedByQuests) native

int Function RaidTargetsAvailable(Keyword akActionKW, Message astrConfirm, Location aLocToHighlight, FormList akIncludeKeywordList, FormList akExcludeKeywordList, bool abExcludeZeroPopulation, bool abOnlyOwnedWorkshops, bool abTurnOffHeader, bool abOnlyPotentialVassalSettlements, bool abDisableReservedByQuests) native

ObjectReference Function PlaceAtMe(Form akFormToPlace, int aiCount, bool abForcePersist, bool abInitiallyDisabled, bool abDeleteWhenAble) native

Actor Function PlaceActorAtMe(ActorBase akActorToPlace, int aiLevelMod, EncounterZone akZone) native

ObjectReference Function PlaceAtNode(string asNodeName, Form akFormToPlace, int aiCount, bool abForcePersist, bool abInitiallyDisabled, bool abDeleteWhenAble, bool abAttach) native

bool Function PlayAnimation(string asAnimation) native

bool Function PlayAnimationAndWait(string asAnimation, string asEventName) native

bool Function PlayGamebryoAnimation(string asAnimation, bool abStartOver, float afEaseInTime) native

bool Function PlayImpactEffect(ImpactDataSet akImpactEffect, string asNodeName, float afPickDirX, float afPickDirY, float afPickDirZ, float afPickLength, bool abApplyNodeRotation, bool abUseNodeLocalRotation) native

bool Function PlaySyncedAnimationSS(string asAnimation1, ObjectReference akObj2, string asAnimation2) native

bool Function PlaySyncedAnimationAndWaitSS(string asAnimation1, string asEvent1, ObjectReference akObj2, string asAnimation2, string asEvent2) native

none Function PlayTerrainEffect(string asEffectModelName, string asAttachBoneName) native

none Function PauseAudio() native

none Function PreloadExteriorCell() native

none Function PreloadTargetArea() native

none Function ProcessTrapHit(ObjectReference akTrap, float afDamage, float afPushback, float afXVel, float afYVel, float afZVel, float afXPos, float afYPos, float afZPos, int aeMaterial, float afStagger) native

none Function PushActorAway(Actor akActorToPush, float aiKnockbackForce) native

none Function RecalculateResources() native

none Function RemoveAllItems(ObjectReference akTransferTo, bool abKeepOwnership) native

none Function RemoveComponents(Component akComponent, int aiCount, bool abSilent) native

none Function RemoveItem(Form akItemToRemove, int aiCount, bool abSilent, ObjectReference akOtherContainer) native

none Function RemoveItemByComponent(Form akComponentToRemove, int aiCount, bool abSilent, ObjectReference akOtherContainer) native

none Function RemoveKeyword(Keyword apKeyword) native

none Function RemoveAllMods() native

none Function RemoveAllModsFromInventoryItem(Form akItem) native

none Function RemoveMod(ObjectMod akMod) native

none Function RemoveModFromInventoryItem(Form akItem, ObjectMod akMod) native

bool Function RemoveDependentAnimatedObjectReference(ObjectReference akDependent) native

none Function Repair() native

none Function Reset(ObjectReference akTarget) native

none Function ResetKeyword(Keyword apKeyword) native

none Function RestoreValue(ActorValue akAV, float afAmount) native

none Function ResumeAudio() native

none Function ReverseConveyorBelt(bool abReverse) native

none Function Say(Topic akTopicToSay, Actor akActorToSpeakAs, bool abSpeakInPlayersHead, ObjectReference akTarget) native

none Function SayCustom(Keyword akKeywordToSay, Actor akActorToSpeakAs, bool abSpeakInPlayersHead, ObjectReference akTarget) native

none Function SendStealAlarm(Actor akThief) native

none Function SetActivateTextOverride(Message akText) native

none Function SetActorCause(Actor akActor) native

none Function SetActorOwner(ActorBase akActorBase, bool abNoCrime) native

none Function SetActorRefOwner(Actor akActor, bool abNoCrime) native

none Function SetAngle(float afXAngle, float afYAngle, float afZAngle) native

none Function SetAnimationVariableBool(string arVariableName, bool abNewValue) native

none Function SetAnimationVariableInt(string arVariableName, int aiNewValue) native

none Function SetAnimationVariableFloat(string arVariableName, float afNewValue) native

none Function SetAttractionActive(Keyword apKeyword, bool abActive) native

none Function SetConveyorBeltVelocity(float afLinVelX, float afLinVelY, float afLinVelZ) native

none Function SetDestroyed(bool abDestroyed) native

none Function SetDirectAtTarget(ObjectReference akTarget) native

none Function SetFactionOwner(Faction akFaction, bool abNoCrime) native

none Function SetHarvested(bool abHarvested) native

none Function SetLinkedRef(ObjectReference akLinkedRef, Keyword apKeyword) native

none Function SetLockLevel(int aiLockLevel) native

none Function SetLocRefType(Location akLoc, LocationRefType akRefType) native

none Function SetMotionType(int aeMotionType, bool abAllowActivate) native

none Function SetNoFavorAllowed(bool abNoFavor) native

none Function SetOpen(bool abOpen) native

none Function SetPersistLoc(Location akLoc) native

none Function SetPlayerHasTaken(bool abTaken) native

none Function SetPosition(float afX, float afY, float afZ) native

none Function SetRadioOn(bool abOn) native

none Function SetRadioFrequency(float afFrequency) native

none Function SetRadioVolume(float afVolume) native

none Function SetScale(float afScale) native

none Function SetValue(ActorValue akAV, float afValue) native

none Function TranslateTo(float afX, float afY, float afZ, float afXAngle, float afYAngle, float afZAngle, float afSpeed, float afMaxRotationSpeed) native

none Function SplineTranslateTo(float afX, float afY, float afZ, float afXAngle, float afYAngle, float afZAngle, float afTangentMagnitude, float afSpeed, float afMaxRotationSpeed) native

none Function SplineTranslateToRefNode(ObjectReference arTarget, string arNodeName, float afTangentMagnitude, float afSpeed, float afMaxRotationSpeed) native

none Function StartWorkshop(bool abStart) native

none Function StopTranslation() native

none Function StoreInWorkshop(Form akBaseItem, int aiCount) native

none Function TetherToHorse(ObjectReference akHorse) native

bool Function WaitForAnimationEvent(string asEventName) native

bool Function WaitFor3DLoad() native

none Function WaitForWorkshopResourceRecalc() native

