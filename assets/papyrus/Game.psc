; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Game

none Function AddAchievement(int aiAchievementID) global native

none Function AddPerkPoints(int aiPerkPoints) global native

none Function AdvanceSkill(string asSkillName, float afMagnitude) global native

none Function ClearPrison() global native

none Function ClearTempEffects() global native

none Function EnablePipboyHDRMask(bool abEnable) global native

none Function Error(string asMessage) global native

none Function FadeOutGame(bool abFadingOut, bool abBlackFade, float afSecsBeforeFade, float afFadeDuration, bool abStayFaded) global native

none Function FastTravel(ObjectReference akDestination) global native

ObjectReference Function FindClosestReferenceOfType(Form arBaseObject, float afX, float afY, float afZ, float afRadius) global native

ObjectReference Function FindRandomReferenceOfType(Form arBaseObject, float afX, float afY, float afZ, float afRadius) global native

ObjectReference Function FindClosestReferenceOfAnyTypeInList(FormList arBaseObjects, float afX, float afY, float afZ, float afRadius) global native

ObjectReference Function FindRandomReferenceOfAnyTypeInList(FormList arBaseObjects, float afX, float afY, float afZ, float afRadius) global native

Actor Function FindClosestActor(float afX, float afY, float afZ, float afRadius) global native

Actor Function FindRandomActor(float afX, float afY, float afZ, float afRadius) global native

none Function ForceDisableSSRGodraysDirLight(bool abDisableSSR, bool abDisableGodrays, bool abDisableDirLight) global native

none Function ForceThirdPerson() global native

none Function ForceFirstPerson() global native

int Function GetXPForLevel(int auiLevel) global native

none Function ShowFirstPersonGeometry(bool abShow) global native

none Function ShowAllMapMarkers() global native

ActorValue Function GetAggressionAV() global native

ActorValue Function GetAgilityAV() global native

ActorValue Function GetCharismaAV() global native

ActorValue Function GetConfidenceAV() global native

int Function GetDifficulty() global native

ActorValue Function GetEnduranceAV() global native

Form Function GetForm(int aiFormID) global native

Form Function GetFormFromFile(int aiFormID, string asFilename) global native

float Function GetGameSettingFloat(string asGameSetting) global native

int Function GetGameSettingInt(string asGameSetting) global native

string Function GetGameSettingString(string asGameSetting) global native

ActorValue Function GetHealthAV() global native

ActorValue Function GetIntelligenceAV() global native

ActorValue Function GetLuckAV() global native

ActorValue Function GetPerceptionAV() global native

Actor Function GetPlayer() global native

Actor[] Function GetPlayerFollowers() global native

ObjectReference Function GetPlayerGrabbedRef() global native

float Function GetPlayerRadioFrequency() global native

Actor Function GetPlayersLastRiddenHorse() global native

float Function GetRealHoursPassed() global native

ActorValue Function GetSuspiciousAV() global native

ActorValue Function GetStrengthAV() global native

none Function IncrementSkill(ActorValue akActorValue, int aiCount) global native

none Function IncrementStat(string asStatName, int aiModAmount) global native

none Function InitializeMarkerDistances() global native

bool Function IsActivateControlsEnabled() global native

bool Function IsVATSControlsEnabled() global native

bool Function IsVATSPlaybackActive() global native

bool Function IsCamSwitchControlsEnabled() global native

bool Function IsFastTravelControlsEnabled() global native

bool Function IsFastTravelEnabled() global native

bool Function IsFavoritesControlsEnabled() global native

bool Function IsFightingControlsEnabled() global native

bool Function IsJournalControlsEnabled() global native

bool Function IsJumpingControlsEnabled() global native

bool Function IsLookingControlsEnabled() global native

bool Function IsMenuControlsEnabled() global native

bool Function IsMovementControlsEnabled() global native

bool Function IsPluginInstalled(string asName) global native

bool Function IsPlayerInRadioRange(float afFrequency) global native

bool Function IsPlayerListening(float afFrequency) global native

bool Function IsPlayerRadioOn() global native

bool Function IsSneakingControlsEnabled() global native

none Function PassTime(int aiHours) global native

none Function PlayBink(string asFileName, bool abInterruptible, bool abMuteAudio, bool abMuteMusic, bool abLetterbox, bool abIsNewGameBink) global native

none Function PrecacheCharGen() global native

none Function PrecacheCharGenClear() global native

int Function QueryStat(string asStat) global native

none Function QuitToMainMenu() global native

none Function RequestAutoSave() global native

none Function RequestModel(string asModelName) global native

none Function RequestSave() global native

none Function RewardPlayerXP(int auiXPAmount, bool abDirect) global native

none Function ServeTime() global native

none Function SetCameraTarget(Actor arTarget) global native

none Function SetCharGenHUDMode(int aiCGHUDMode) global native

none Function SetInsideMemoryHUDMode(bool aInsideMemory) global native

none Function ShowPerkVaultBoyOnHUD(string aVaultBoySwf, Sound aSoundDescriptor) global native

none Function SetInChargen(bool abDisableSaving, bool abDisableWaiting, bool abShowControlsDisabledMessage) global native

none Function SetPlayerAIDriven(bool abAIDriven) global native

none Function SetPlayerOnElevator(bool abOnElevator) global native

none Function SetPlayerRadioFrequency(float afFrequency) global native

none Function SetPlayerReportCrime(bool abReportCrime) global native

none Function SetSittingRotation(float afValue) global native

none Function ShakeCamera(ObjectReference akSource, float afStrength, float afDuration) global native

none Function ShakeController(float afSmallMotorStrength, float afBigMotorStreangth, float afDuration) global native

none Function ShowFatigueWarningOnHUD() global native

none Function ShowRaceMenu(ObjectReference akMenuTarget, int uiMode, ObjectReference akMenuSpouseFemale, ObjectReference akMenuSpouseMale, ObjectReference akVendor) global native

none Function ShowSPECIALMenu() global native

none Function ShowTitleSequenceMenu() global native

none Function HideTitleSequenceMenu() global native

none Function StartTitleSequence(string asSequenceName) global native

none Function ShowPipboyBootSequence(string asAnimationName) global native

none Function ShowPipboyPlugin() global native

none Function ShowTrainingMenu(Actor aTrainer) global native

none Function TriggerScreenBlood(int aiValue) global native

none Function PlayEventCamera(CameraShot akCamera, ObjectReference akRef) global native

none Function StartDialogueCameraOrCenterOnTarget(ObjectReference akCameraTarget) global native

none Function StopDialogueCamera(bool abConsiderResume, bool abSwitchingTo1stP) global native

none Function TurnPlayerRadioOn(bool abRadioOn) global native

bool Function UsingGamepad() global native

none Function Warning(string asMessage) global native

