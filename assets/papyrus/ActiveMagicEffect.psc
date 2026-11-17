; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName ActiveMagicEffect extends ScriptObject

none Function Dispel() native

MagicEffect Function GetBaseObject() native

Actor Function GetCasterActor() native

Actor Function GetTargetActor() native

none Function StartObjectProfiling() native

none Function StopObjectProfiling() native

