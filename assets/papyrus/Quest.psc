; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Quest extends Form

none Function CompleteAllObjectives() native

none Function CompleteQuest() native

none Function FailAllObjectives() native

Alias Function GetAlias(int aiAliasID) native

int Function GetCurrentStageID() native

bool Function HasObjective(int aiObjective) native

bool Function IsActive() native

bool Function IsCompleted() native

bool Function IsObjectiveCompleted(int aiObjective) native

bool Function IsObjectiveDisplayed(int aiObjective) native

bool Function IsObjectiveFailed(int aiObjective) native

bool Function IsRunning() native

bool Function IsStageDone(int aiStage) native

bool Function IsStarting() native

bool Function IsStopping() native

bool Function IsStopped() native

none Function Reset() native

none Function ResetSpeechChallenges() native

none Function SetActive(bool abActive) native

bool Function SetCurrentStageID(int aiStageID) native

none Function SetObjectiveCompleted(int aiObjective, bool abCompleted) native

none Function SetObjectiveDisplayed(int aiObjective, bool abDisplayed, bool abForce) native

none Function SetObjectiveFailed(int aiObjective, bool abFailed) native

bool Function Start() native

none Function Stop() native

bool Function UpdateCurrentInstanceGlobal(GlobalVariable aUpdateGlobal) native

