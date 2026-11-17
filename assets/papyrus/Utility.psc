; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Utility

var Function CallGlobalFunction(string asScriptName, string asFuncName, var[] aParams) global native

none Function CallGlobalFunctionNoWait(string asScriptName, string asFuncName, var[] aParams) global native

string Function GameTimeToString(float afGameTime) global native

float Function GetCurrentGameTime() global native

float Function GetCurrentRealTime() global native

int Function GetCurrentStackID() global native

bool Function IsInMenuMode() global native

int Function RandomInt(int aiMin, int aiMax) global native

float Function RandomFloat(float afMin, float afMax) global native

none Function SetINIFloat(string ini, float value) global native

none Function SetINIInt(string ini, int value) global native

none Function SetINIBool(string ini, bool value) global native

none Function SetINIString(string ini, string value) global native

none Function Wait(float afSeconds) global native

none Function WaitGameTime(float afHours) global native

none Function WaitMenuMode(float afSeconds) global native

string Function CaptureFrameRate(int numFrames) global native

none Function EnterTestData(string astestType, string astestMatter, string astestDetails, string astestResultContext, string astestResult) global native

none Function PostStartUpTimes() global native

none Function StartFrameRateCapture() global native

none Function EndFrameRateCapture() global native

float Function GetAverageFrameRate() global native

float Function GetMinFrameRate() global native

float Function GetMaxFrameRate() global native

string Function GetCurrentMemory() global native

int Function GetBudgetCount() global native

string Function GetCurrentBudget(int aiBudgetNumber) global native

string Function GetBudgetLimit(int aiBudgetNumber) global native

bool Function OverBudget(int aiBudgetNumber) global native

string Function GetBudgetName(int aiBudgetNumber) global native

