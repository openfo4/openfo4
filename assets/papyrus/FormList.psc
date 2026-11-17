; This file contains the native API definitions that the OpenFO4 engine links against.
; It contains no implementations, only native functions, do not add other functions to it.
; This file is not read by OpenFO4 itself in any way. Games need to ship with their own
; compiled scripts using the same API for native functions as specified in this file.
; This file exists to allow other compilers to link scripts compatible with OpenFO4.
ScriptName FormList extends Form

none Function AddForm(Form apForm) native

int Function Find(Form apForm) native

int Function GetSize() native

Form Function GetAt(int aiIndex) native

bool Function HasForm(Form akForm) native

none Function RemoveAddedForm(Form apForm) native

none Function Revert() native

