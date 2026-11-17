; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName ScriptObject

none Function AddInventoryEventFilter(Form akFilter) native

var Function CallFunction(string asFuncName, var[] aParams) native

none Function CallFunctionNoWait(string asFuncName, var[] aParams) native

none Function CancelTimer(int aiTimerID) native

none Function CancelTimerGameTime(int aiTimerID) native

var Function GetPropertyValue(string asPropertyName) native

ScriptObject Function CastAs(string asScriptName) native

bool Function IsBoundGameObjectAvailable() native

bool Function RegisterForAnimationEvent(ObjectReference akSender, string asEventName) native

none Function RegisterForCustomEvent(ScriptObject akSender, string asEventName) native

none Function RegisterForDetectionLOSGain(Actor akViewer, ObjectReference akTarget) native

none Function RegisterForDetectionLOSLost(Actor akViewer, ObjectReference akTarget) native

none Function RegisterForDirectLOSGain(ObjectReference akViewer, ObjectReference akTarget, string asViewerNode, string asTargetNode) native

none Function RegisterForDirectLOSLost(ObjectReference akViewer, ObjectReference akTarget, string asViewerNode, string asTargetNode) native

none Function RegisterForDistanceLessThanEvent(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native

none Function RegisterForDistanceGreaterThanEvent(ScriptObject akObj1, ScriptObject akObj2, float afDistance) native

none Function RegisterForHitEvent(ScriptObject akTarget, ScriptObject akAggressorFilter, Form akSourceFilter, Form akProjectileFilter, int aiPowerFilter, int aiSneakFilter, int aiBashFilter, int aiBlockFilter, bool abMatch) native

none Function RegisterForMagicEffectApplyEvent(ScriptObject akTarget, ScriptObject akCasterFilter, Form akEffectFilter, bool abMatch) native

none Function RegisterForMenuOpenCloseEvent(string asMenuName) native

none Function RegisterForPlayerSleep() native

none Function RegisterForPlayerTeleport() native

none Function RegisterForPlayerWait() native

none Function RegisterForRadiationDamageEvent(ScriptObject akTarget) native

bool Function RegisterForRemoteEvent(ScriptObject akEventSource, string asEventName) native

none Function RegisterForTrackedStatsEvent(string asStat, int aiThreshold) native

none Function RegisterForLooksMenuEvent() native

none Function RegisterForTutorialEvent(string asEventName) native

none Function RemoveAllInventoryEventFilters() native

none Function RemoveInventoryEventFilter(Form akFilter) native

none Function SendCustomEvent(string asEvent, var[] akArgs) native

none Function SetPropertyValue(string asPropertyName, var aValue) native

none Function SetPropertyValueNoWait(string asPropertyName, var aValue) native

none Function StartTimer(float afInterval, int aiTimerID) native

none Function StartTimerGameTime(float afInterval, int aiTimerID) native

none Function UnregisterForAllEvents() native

none Function UnregisterForAllCustomEvents() native

none Function UnregisterForAllHitEvents(ScriptObject akTarget) native

none Function UnregisterForAllMagicEffectApplyEvents(ScriptObject akTarget) native

none Function UnregisterForAllMenuOpenCloseEvents() native

none Function UnregisterForAllRadiationDamageEvents() native

none Function UnregisterForAllRemoteEvents() native

none Function UnregisterForAllTrackedStatsEvents() native

none Function UnregisterForAnimationEvent(ObjectReference akSender, string asEventName) native

none Function UnregisterForCustomEvent(ScriptObject akSender, string asEventName) native

none Function UnregisterForDistanceEvents(ScriptObject akObj1, ScriptObject akObj2) native

none Function UnregisterForHitEvent(ScriptObject akTarget, ScriptObject akAggressorFilter, Form akSourceFilter, Form akProjectileFilter, int aiPowerFilter, int aiSneakFilter, int aiBashFilter, int aiBlockFilter, bool abMatch) native

none Function UnregisterForLOS(ObjectReference akViewer, ObjectReference akTarget) native

none Function UnregisterForMagicEffectApplyEvent(ScriptObject akTarget, ScriptObject akCasterFilter, Form akEffectFilter, bool abMatch) native

none Function UnregisterForMenuOpenCloseEvent(string asMenuName) native

none Function UnregisterForPlayerSleep() native

none Function UnregisterForPlayerTeleport() native

none Function UnregisterForPlayerWait() native

none Function UnregisterForRadiationDamageEvent(ScriptObject akTarget) native

none Function UnregisterForRemoteEvent(ScriptObject akEventSource, string asEventName) native

none Function UnregisterForTrackedStatsEvent(string asStat) native

none Function UnregisterForLooksMenuEvent() native

none Function UnregisterForTutorialEvent(string asEventName) native

