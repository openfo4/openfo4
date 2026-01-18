use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Utility", "CallGlobalFunction", call_global_function);
    engine.register_native_function("Utility", "CallGlobalFunctionNoWait", call_global_function_no_wait);
    engine.register_native_function("Utility", "GameTimeToString", game_time_to_string);
    engine.register_native_function("Utility", "GetCurrentGameTime", get_current_game_time);
    engine.register_native_function("Utility", "GetCurrentRealTime", get_current_real_time);
    engine.register_native_function("Utility", "GetCurrentStackID", get_current_stack_id);
    engine.register_native_function("Utility", "IsInMenuMode", is_in_menu_mode);
    engine.register_native_function("Utility", "RandomInt", random_int);
    engine.register_native_function("Utility", "RandomFloat", random_float);
    engine.register_native_function("Utility", "SetINIFloat", set_ini_float);
    engine.register_native_function("Utility", "SetINIInt", set_ini_int);
    engine.register_native_function("Utility", "SetINIBool", set_ini_bool);
    engine.register_native_function("Utility", "SetINIString", set_ini_string);
    engine.register_native_function("Utility", "Wait", wait);
    engine.register_native_function("Utility", "WaitGameTime", wait_game_time);
    engine.register_native_function("Utility", "WaitMenuMode", wait_menu_mode);
    engine.register_native_function("Utility", "CaptureFrameRate", capture_frame_rate);
    engine.register_native_function("Utility", "EnterTestData", enter_test_data);
    engine.register_native_function("Utility", "PostStartUpTimes", post_start_up_times);
    engine.register_native_function("Utility", "StartFrameRateCapture", start_frame_rate_capture);
    engine.register_native_function("Utility", "EndFrameRateCapture", end_frame_rate_capture);
    engine.register_native_function("Utility", "GetAverageFrameRate", get_average_frame_rate);
    engine.register_native_function("Utility", "GetMinFrameRate", get_min_frame_rate);
    engine.register_native_function("Utility", "GetMaxFrameRate", get_max_frame_rate);
    engine.register_native_function("Utility", "GetCurrentMemory", get_current_memory);
    engine.register_native_function("Utility", "GetBudgetCount", get_budget_count);
    engine.register_native_function("Utility", "GetCurrentBudget", get_current_budget);
    engine.register_native_function("Utility", "GetBudgetLimit", get_budget_limit);
    engine.register_native_function("Utility", "OverBudget", over_budget);
    engine.register_native_function("Utility", "GetBudgetName", get_budget_name);
}

/// `CallGlobalFunction`
///
/// ```text
/// Var Function(string asScriptName, string asFuncName, Var[] aParams) native global
/// ```
fn call_global_function(
    caller: Caller<'_, State>,
    as_script_name: isize,
    as_func_name: isize,
    a_params: isize,
) -> Result<Object, Exception> {
    todo!()
}

/// `CallGlobalFunctionNoWait`
///
/// ```text
/// Function(string asScriptName, string asFuncName, Var[] aParams) native global
/// ```
fn call_global_function_no_wait(
    caller: Caller<'_, State>,
    as_script_name: isize,
    as_func_name: isize,
    a_params: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `GameTimeToString`
///
/// ```text
/// string Function(float afGameTime) native global
/// ```
fn game_time_to_string(caller: Caller<'_, State>, af_game_time: f32) -> Result<isize, Exception> {
    todo!()
}

/// `GetCurrentGameTime`
///
/// ```text
/// float Function() native global
/// ```
fn get_current_game_time(caller: Caller<'_, State>) -> Result<f32, Exception> {
    todo!()
}

/// `GetCurrentRealTime`
///
/// ```text
/// float Function() native global
/// ```
fn get_current_real_time(caller: Caller<'_, State>) -> Result<f32, Exception> {
    todo!()
}

/// `GetCurrentStackID`
///
/// ```text
/// int Function() native global
/// ```
fn get_current_stack_id(caller: Caller<'_, State>) -> Result<i32, Exception> {
    todo!()
}

/// `IsInMenuMode`
///
/// ```text
/// bool Function() native global
/// ```
fn is_in_menu_mode(caller: Caller<'_, State>) -> Result<bool, Exception> {
    todo!()
}

/// `RandomInt`
///
/// ```text
/// int Function(int aiMin, int aiMax) native global
/// ```
fn random_int(caller: Caller<'_, State>, ai_min: i32, ai_max: i32) -> Result<i32, Exception> {
    todo!()
}

/// `RandomFloat`
///
/// ```text
/// float Function(float afMin, float afMax) native global
/// ```
fn random_float(caller: Caller<'_, State>, af_min: f32, af_max: f32) -> Result<f32, Exception> {
    todo!()
}

/// `SetINIFloat`
///
/// ```text
/// Function(string ini, float value) native global
/// ```
fn set_ini_float(caller: Caller<'_, State>, ini: isize, value: f32) -> Result<(), Exception> {
    todo!()
}

/// `SetINIInt`
///
/// ```text
/// Function(string ini, int value) native global
/// ```
fn set_ini_int(caller: Caller<'_, State>, ini: isize, value: i32) -> Result<(), Exception> {
    todo!()
}

/// `SetINIBool`
///
/// ```text
/// Function(string ini, bool value) native global
/// ```
fn set_ini_bool(caller: Caller<'_, State>, ini: isize, value: bool) -> Result<(), Exception> {
    todo!()
}

/// `SetINIString`
///
/// ```text
/// Function(string ini, string value) native global
/// ```
fn set_ini_string(caller: Caller<'_, State>, ini: isize, value: isize) -> Result<(), Exception> {
    todo!()
}

/// `Wait`
///
/// ```text
/// Function(float afSeconds) native global
/// ```
fn wait(caller: Caller<'_, State>, af_seconds: f32) -> Result<(), Exception> {
    todo!()
}

/// `WaitGameTime`
///
/// ```text
/// Function(float afHours) native global
/// ```
fn wait_game_time(caller: Caller<'_, State>, af_hours: f32) -> Result<(), Exception> {
    todo!()
}

/// `WaitMenuMode`
///
/// ```text
/// Function(float afSeconds) native global
/// ```
fn wait_menu_mode(caller: Caller<'_, State>, af_seconds: f32) -> Result<(), Exception> {
    todo!()
}

/// `CaptureFrameRate`
///
/// ```text
/// string Function(int numFrames) native global
/// ```
fn capture_frame_rate(caller: Caller<'_, State>, num_frames: i32) -> Result<isize, Exception> {
    todo!()
}

/// `EnterTestData`
///
/// ```text
/// Function(string astestType, string astestMatter, string astestDetails, string astestResultContext, string astestResult) native global
/// ```
fn enter_test_data(
    caller: Caller<'_, State>,
    astest_type: isize,
    astest_matter: isize,
    astest_details: isize,
    astest_result_context: isize,
    astest_result: isize,
) -> Result<(), Exception> {
    todo!()
}

/// `PostStartUpTimes`
///
/// ```text
/// Function() native global
/// ```
fn post_start_up_times(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `StartFrameRateCapture`
///
/// ```text
/// Function() native global
/// ```
fn start_frame_rate_capture(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `EndFrameRateCapture`
///
/// ```text
/// Function() native global
/// ```
fn end_frame_rate_capture(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `GetAverageFrameRate`
///
/// ```text
/// float Function() native global
/// ```
fn get_average_frame_rate(caller: Caller<'_, State>) -> Result<f32, Exception> {
    todo!()
}

/// `GetMinFrameRate`
///
/// ```text
/// float Function() native global
/// ```
fn get_min_frame_rate(caller: Caller<'_, State>) -> Result<f32, Exception> {
    todo!()
}

/// `GetMaxFrameRate`
///
/// ```text
/// float Function() native global
/// ```
fn get_max_frame_rate(caller: Caller<'_, State>) -> Result<f32, Exception> {
    todo!()
}

/// `GetCurrentMemory`
///
/// ```text
/// string Function() native global
/// ```
fn get_current_memory(caller: Caller<'_, State>) -> Result<isize, Exception> {
    todo!()
}

/// `GetBudgetCount`
///
/// ```text
/// int Function() native global
/// ```
fn get_budget_count(caller: Caller<'_, State>) -> Result<i32, Exception> {
    todo!()
}

/// `GetCurrentBudget`
///
/// ```text
/// string Function(int aiBudgetNumber) native global
/// ```
fn get_current_budget(
    caller: Caller<'_, State>,
    ai_budget_number: i32,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetBudgetLimit`
///
/// ```text
/// string Function(int aiBudgetNumber) native global
/// ```
fn get_budget_limit(caller: Caller<'_, State>, ai_budget_number: i32) -> Result<isize, Exception> {
    todo!()
}

/// `OverBudget`
///
/// ```text
/// bool Function(int aiBudgetNumber) native global
/// ```
fn over_budget(caller: Caller<'_, State>, ai_budget_number: i32) -> Result<bool, Exception> {
    todo!()
}

/// `GetBudgetName`
///
/// ```text
/// string Function(int aiBudgetNumber) native global
/// ```
fn get_budget_name(caller: Caller<'_, State>, ai_budget_number: i32) -> Result<isize, Exception> {
    todo!()
}
