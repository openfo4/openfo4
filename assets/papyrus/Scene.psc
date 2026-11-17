; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Scene extends Form

none Function ForceStart() native

none Function Start() native

none Function Stop() native

bool Function IsPlaying() native

Quest Function GetOwningQuest() native

bool Function IsActionComplete(int aiActionID) native

none Function Pause(bool abPause) native

