; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Location extends Form

none Function AddLinkedLocation(Location akLoc, Keyword akKeyword) native

Location[] Function GetAllLinkedLocations(Keyword akKeyword) native

float Function GetKeywordData(Keyword akKeyword) native

int Function GetRefTypeAliveCount(LocationRefType akRefType) native

int Function GetRefTypeDeadCount(LocationRefType akRefType) native

bool Function HasCommonParent(Location akOther, Keyword akFilter) native

bool Function HasEverBeenCleared() native

bool Function HasRefType(LocationRefType akRefType) native

bool Function IsCleared() native

bool Function IsChild(Location akOther) native

bool Function IsLinkedLocation(Location akLocation, Keyword akKeyword) native

bool Function IsLoaded() native

none Function RemoveLinkedLocation(Location akLoc, Keyword akKeyword) native

none Function Reset() native

none Function SetKeywordData(Keyword akKeyword, float afData) native

none Function SetCleared(bool abCleared) native

