; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Weather extends Form

none Function EnableAmbientParticles(bool abEnable) global native

Weather Function FindWeather(int auiType) global native

none Function ForceActive(bool abOverride) native

int Function GetClassification() native

Weather Function GetCurrentWeather() global native

float Function GetCurrentWeatherTransition() global native

Weather Function GetOutgoingWeather() global native

int Function GetSkyMode() global native

none Function ReleaseOverride() global native

none Function SetActive(bool abOverride, bool abAccelerate) native

