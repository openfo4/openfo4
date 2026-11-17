; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Message extends Form

int Function Show(float afArg1, float afArg2, float afArg3, float afArg4, float afArg5, float afArg6, float afArg7, float afArg8, float afArg9) native

none Function ShowAsHelpMessage(string asEvent, float afDuration, float afInterval, int aiMaxTimes, string asContext, int aiPriority) native

none Function UnshowAsHelpMessage() native

none Function ClearHelpMessages() global native

none Function ResetHelpMessage(string asEvent) global native

