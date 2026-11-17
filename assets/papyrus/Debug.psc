; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Debug

none Function CenterOnCell(string asCellname) global native

float Function CenterOnCellAndWait(string asCellname) global native

float Function PlayerMoveToAndWait(string asDestRef) global native

none Function CloseUserLog(string asLogName) global native

none Function DumpAliasData(Quest akQuest) global native

none Function DumpEventRegistrations(ScriptObject akScript) global native

none Function EnableAI(bool abEnable) global native

none Function EnableCollisions(bool abEnable) global native

none Function EnableDetection(bool abEnable) global native

none Function EnableMenus(bool abEnable) global native

string Function GetConfigName() global native

string Function GetPlatformName() global native

string Function GetVersionNumber() global native

none Function MessageBox(string asMessageBoxText) global native

none Function Notification(string asNotificationText) global native

bool Function OpenUserLog(string asLogName) global native

none Function QuitGame() global native

none Function SetFootIK(bool abFootIK) global native

none Function SetGodMode(bool abGodMode) global native

none Function StartScriptProfiling(string asScriptName) global native

none Function StartStackProfiling() global native

none Function StartStackRootProfiling(string asScriptName, ScriptObject akObj) global native

none Function StopScriptProfiling(string asScriptName) global native

none Function StopStackProfiling() global native

none Function StopStackRootProfiling(string asScriptName, ScriptObject akObj) global native

none Function Trace(string asTextToPrint, int aiSeverity) global native

none Function TraceFunction(string asTextToPrint, int aiSeverity) global native

none Function TraceStack(string asTextToPrint, int aiSeverity) global native

bool Function TraceUser(string asUserLog, string asTextToPrint, int aiSeverity) global native

none Function ShowRefPosition(ObjectReference arRef) global native

none Function DBSendPlayerPosition() global native

