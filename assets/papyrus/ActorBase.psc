; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName ActorBase extends Form

Class Function GetClass() native

int Function GetDeadCount() native

FormList Function GetGiftFilter() native

Race Function GetRace() native

int Function GetLevel() native

int Function GetLevelExact() native

int Function GetSex() native

Actor Function GetUniqueActor() native

bool Function IsEssential() native

bool Function IsInvulnerable() native

bool Function IsProtected() native

bool Function IsUnique() native

none Function SetEssential(bool abEssential) native

none Function SetInvulnerable(bool abInvulnerable) native

none Function SetProtected(bool abProtected) native

none Function SetOutfit(Outfit akOutfit, bool abSleepOutfit) native

