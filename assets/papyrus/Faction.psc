; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Faction extends Form

bool Function CanPayCrimeGold() native

int Function GetCrimeGold() native

int Function GetCrimeGoldNonViolent() native

int Function GetCrimeGoldViolent() native

int Function GetInfamy() native

int Function GetInfamyNonViolent() native

int Function GetInfamyViolent() native

int Function GetFactionReaction(Actor akOther) native

int Function GetStolenItemValueCrime() native

int Function GetStolenItemValueNoCrime() native

bool Function IsFactionInCrimeGroup(Faction akOther) native

bool Function IsPlayerEnemy() native

bool Function IsPlayerExpelled() native

none Function ModCrimeGold(int aiAmount, bool abViolent) native

none Function PlayerPayCrimeGold(bool abRemoveStolenItems, bool abGoToJail) native

none Function SendAssaultAlarm() native

none Function SendPlayerToJail(bool abRemoveInventory, bool abRealJail) native

none Function SetAlly(Faction akOther, bool abSelfIsFriendToOther, bool abOtherIsFriendToSelf) native

none Function SetCrimeGold(int aiGold) native

none Function SetCrimeGoldViolent(int aiGold) native

none Function SetEnemy(Faction akOther, bool abSelfIsNeutralToOther, bool abOtherIsNeutralToSelf) native

none Function SetPlayerEnemy(bool abIsEnemy) native

none Function SetPlayerExpelled(bool abIsExpelled) native

