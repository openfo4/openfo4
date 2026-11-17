; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName Math

float Function abs(float afValue) global native

float Function acos(float afValue) global native

float Function asin(float afValue) global native

float Function atan(float afValue) global native

int Function Ceiling(float afValue) global native

float Function cos(float afValue) global native

float Function DegreesToRadians(float afDegrees) global native

int Function Floor(float afValue) global native

float Function pow(float x, float y) global native

float Function RadiansToDegrees(float afRadians) global native

float Function sin(float afValue) global native

float Function sqrt(float afValue) global native

float Function tan(float afValue) global native

