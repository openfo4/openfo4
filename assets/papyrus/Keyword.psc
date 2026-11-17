; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Keyword extends Form

none Function SendStoryEvent(Location akLoc, ObjectReference akRef1, ObjectReference akRef2, int aiValue1, int aiValue2) native

bool Function SendStoryEventAndWait(Location akLoc, ObjectReference akRef1, ObjectReference akRef2, int aiValue1, int aiValue2) native

