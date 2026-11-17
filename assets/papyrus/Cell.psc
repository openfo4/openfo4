; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Cell extends Form

none Function EnableFastTravel(bool abEnable) native

ActorBase Function GetActorOwner() native

Faction Function GetFactionOwner() native

bool Function IsAttached() native

bool Function IsInterior() native

bool Function IsLoaded() native

none Function Reset() native

none Function SetActorOwner(ActorBase akActor) native

none Function SetFactionOwner(Faction akFaction) native

none Function SetFogColor(int aiNearRed, int aiNearGreen, int aiNearBlue, int aiFarRed, int aiFarGreen, int aiFarBlue) native

none Function SetFogPlanes(float afNear, float afFar) native

none Function SetFogPower(float afPower) native

none Function SetPublic(bool abPublic) native

