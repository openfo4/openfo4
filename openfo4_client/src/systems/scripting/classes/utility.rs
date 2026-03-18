use std::time::Duration;

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
fn get_current_stack_id(_caller: Caller<'_, State>) -> Result<i32, Exception> {
    Ok(0)
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
fn random_int(_caller: Caller<'_, State>, min: i32, max: i32) -> Result<i32, Exception> {
    Ok(rand::random_range(min..=max))
}

/// `RandomFloat`
///
/// ```text
/// float Function(float afMin, float afMax) native global
/// ```
fn random_float(_caller: Caller<'_, State>, min: f32, max: f32) -> Result<f32, Exception> {
    Ok(rand::random_range(min..=max))
}

/// `SetINIFloat`
///
/// ```text
/// Function(string ini, float value) native global
/// ```
fn set_ini_float(_caller: Caller<'_, State>, _ini: Object, _value: f32) -> Result<(), Exception> {
    Ok(())
}

/// `SetINIInt`
///
/// ```text
/// Function(string ini, int value) native global
/// ```
fn set_ini_int(_caller: Caller<'_, State>, _ini: Object, _value: i32) -> Result<(), Exception> {
    Ok(())
}

/// `SetINIBool`
///
/// ```text
/// Function(string ini, bool value) native global
/// ```
fn set_ini_bool(_caller: Caller<'_, State>, _ini: Object, _value: bool) -> Result<(), Exception> {
    Ok(())
}

/// `SetINIString`
///
/// ```text
/// Function(string ini, string value) native global
/// ```
fn set_ini_string(
    _caller: Caller<'_, State>,
    _ini: Object,
    _value: isize,
) -> Result<(), Exception> {
    Ok(())
}

/// `Wait`
///
/// ```text
/// Function(float afSeconds) native global
/// ```
fn wait(mut caller: Caller<'_, State>, seconds: f32) -> Result<(), Exception> {
    let duration = Duration::from_secs_f32(seconds);
    if duration.is_zero() {
        return Ok(());
    }

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
fn capture_frame_rate(_caller: Caller<'_, State>, _num_frames: i32) -> Result<Object, Exception> {
    Ok(Object::NULL)
}

/// `EnterTestData`
///
/// ```text
/// Function(string astestType, string astestMatter, string astestDetails, string astestResultContext, string astestResult) native global
/// ```
fn enter_test_data(
    _caller: Caller<'_, State>,
    _astest_type: Object,
    _astest_matter: Object,
    _astest_details: Object,
    _astest_result_context: Object,
    _astest_result: Object,
) -> Result<(), Exception> {
    Ok(())
}

/// `PostStartUpTimes`
///
/// ```text
/// Function() native global
/// ```
fn post_start_up_times(_caller: Caller<'_, State>) -> Result<(), Exception> {
    Ok(())
}

/// `StartFrameRateCapture`
///
/// ```text
/// Function() native global
/// ```
fn start_frame_rate_capture(_caller: Caller<'_, State>) -> Result<(), Exception> {
    Ok(())
}

/// `EndFrameRateCapture`
///
/// ```text
/// Function() native global
/// ```
fn end_frame_rate_capture(_caller: Caller<'_, State>) -> Result<(), Exception> {
    Ok(())
}

/// `GetAverageFrameRate`
///
/// ```text
/// float Function() native global
/// ```
fn get_average_frame_rate(_caller: Caller<'_, State>) -> Result<f32, Exception> {
    Ok(60.0)
}

/// `GetMinFrameRate`
///
/// ```text
/// float Function() native global
/// ```
fn get_min_frame_rate(_caller: Caller<'_, State>) -> Result<f32, Exception> {
    Ok(60.0)
}

/// `GetMaxFrameRate`
///
/// ```text
/// float Function() native global
/// ```
fn get_max_frame_rate(_caller: Caller<'_, State>) -> Result<f32, Exception> {
    Ok(60.0)
}

/// `GetCurrentMemory`
///
/// ```text
/// string Function() native global
/// ```
fn get_current_memory(_caller: Caller<'_, State>) -> Result<Object, Exception> {
    Ok(Object::NULL)
}

/// `GetBudgetCount`
///
/// ```text
/// int Function() native global
/// ```
fn get_budget_count(_caller: Caller<'_, State>) -> Result<i32, Exception> {
    Ok(0)
}

/// `GetCurrentBudget`
///
/// ```text
/// string Function(int aiBudgetNumber) native global
/// ```
fn get_current_budget(
    _caller: Caller<'_, State>,
    _budget_number: i32,
) -> Result<Object, Exception> {
    Ok(Object::NULL)
}

/// `GetBudgetLimit`
///
/// ```text
/// string Function(int aiBudgetNumber) native global
/// ```
fn get_budget_limit(_caller: Caller<'_, State>, _budget_number: i32) -> Result<Object, Exception> {
    Ok(Object::NULL)
}

/// `OverBudget`
///
/// ```text
/// bool Function(int aiBudgetNumber) native global
/// ```
fn over_budget(_caller: Caller<'_, State>, _budget_number: i32) -> Result<bool, Exception> {
    Ok(false)
}

/// `GetBudgetName`
///
/// ```text
/// string Function(int aiBudgetNumber) native global
/// ```
fn get_budget_name(_caller: Caller<'_, State>, _budget_number: i32) -> Result<Object, Exception> {
    Ok(Object::NULL)
}
