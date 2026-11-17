; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName ReferenceAlias extends Alias

none Function ApplyToRef(ObjectReference akRef) native

none Function Clear() native

ObjectReference Function GetReference() native

none Function ForceRefTo(ObjectReference akNewRef) native

none Function RemoveFromRef(ObjectReference akRef) native

