use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Debug", "CenterOnCell", center_on_cell);
    engine.register_native_function("Debug", "CenterOnCellAndWait", center_on_cell_and_wait);
    engine.register_native_function("Debug", "PlayerMoveToAndWait", player_move_to_and_wait);
    engine.register_native_function("Debug", "CloseUserLog", close_user_log);
    engine.register_native_function("Debug", "DumpAliasData", dump_alias_data);
    engine.register_native_function("Debug", "DumpEventRegistrations", dump_event_registrations);
    engine.register_native_function("Debug", "EnableAI", enable_ai);
    engine.register_native_function("Debug", "EnableCollisions", enable_collisions);
    engine.register_native_function("Debug", "EnableDetection", enable_detection);
    engine.register_native_function("Debug", "EnableMenus", enable_menus);
    engine.register_native_function("Debug", "GetConfigName", get_config_name);
    engine.register_native_function("Debug", "GetPlatformName", get_platform_name);
    engine.register_native_function("Debug", "GetVersionNumber", get_version_number);
    engine.register_native_function("Debug", "MessageBox", message_box);
    engine.register_native_function("Debug", "Notification", notification);
    engine.register_native_function("Debug", "OpenUserLog", open_user_log);
    engine.register_native_function("Debug", "QuitGame", quit_game);
    engine.register_native_function("Debug", "SetFootIK", set_foot_ik);
    engine.register_native_function("Debug", "SetGodMode", set_god_mode);
    engine.register_native_function("Debug", "StartScriptProfiling", start_script_profiling);
    engine.register_native_function("Debug", "StartStackProfiling", start_stack_profiling);
    engine.register_native_function("Debug", "StartStackRootProfiling", start_stack_root_profiling);
    engine.register_native_function("Debug", "StopScriptProfiling", stop_script_profiling);
    engine.register_native_function("Debug", "StopStackProfiling", stop_stack_profiling);
    engine.register_native_function("Debug", "StopStackRootProfiling", stop_stack_root_profiling);
    engine.register_native_function("Debug", "Trace", trace);
    engine.register_native_function("Debug", "TraceFunction", trace_function);
    engine.register_native_function("Debug", "TraceStack", trace_stack);
    engine.register_native_function("Debug", "TraceUser", trace_user);
    engine.register_native_function("Debug", "ShowRefPosition", show_ref_position);
    engine.register_native_function("Debug", "DBSendPlayerPosition", db_send_player_position);
}

/// `CenterOnCell`
///
/// ```text
/// Function(string asCellname) native global
/// ```
fn center_on_cell(caller: Caller<'_, State>, as_cellname: Object) -> Result<(), Exception> {
    todo!()
}

/// `CenterOnCellAndWait`
///
/// ```text
/// float Function(string asCellname) native global
/// ```
fn center_on_cell_and_wait(
    caller: Caller<'_, State>,
    as_cellname: isize,
) -> Result<f32, Exception> {
    todo!()
}

/// `PlayerMoveToAndWait`
///
/// ```text
/// float Function(string asDestRef) native global
/// ```
fn player_move_to_and_wait(
    caller: Caller<'_, State>,
    as_dest_ref: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `CloseUserLog`
///
/// ```text
/// Function(string asLogName) native global
/// ```
fn close_user_log(_caller: Caller<'_, State>, _log_name: Object) -> Result<(), Exception> {
    Ok(())
}

/// `DumpAliasData`
///
/// ```text
/// Function(Quest akQuest) native global
/// ```
fn dump_alias_data(_caller: Caller<'_, State>, _quest: Object) -> Result<(), Exception> {
    Ok(())
}

/// `DumpEventRegistrations`
///
/// ```text
/// Function(ScriptObject akScript) native global
/// ```
fn dump_event_registrations(caller: Caller<'_, State>, ak_script: Object) -> Result<(), Exception> {
    todo!()
}

/// `EnableAI`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_ai(caller: Caller<'_, State>, ab_enable: bool) -> Result<(), Exception> {
    todo!()
}

/// `EnableCollisions`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_collisions(caller: Caller<'_, State>, ab_enable: bool) -> Result<(), Exception> {
    todo!()
}

/// `EnableDetection`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_detection(caller: Caller<'_, State>, ab_enable: bool) -> Result<(), Exception> {
    todo!()
}

/// `EnableMenus`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_menus(caller: Caller<'_, State>, ab_enable: bool) -> Result<(), Exception> {
    todo!()
}

/// `GetConfigName`
///
/// ```text
/// string Function() native global
/// ```
fn get_config_name(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `GetPlatformName`
///
/// ```text
/// string Function() native global
/// ```
fn get_platform_name(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `GetVersionNumber`
///
/// ```text
/// string Function() native global
/// ```
fn get_version_number(caller: Caller<'_, State>) -> Result<isize, Exception> {
    todo!()
}

/// `MessageBox`
///
/// ```text
/// Function(string asMessageBoxText) native global
/// ```
fn message_box(caller: Caller<'_, State>, as_message_box_text: isize) -> Result<(), Exception> {
    todo!()
}

/// `Notification`
///
/// ```text
/// Function(string asNotificationText) native global
/// ```
fn notification(caller: Caller<'_, State>, as_notification_text: isize) -> Result<(), Exception> {
    todo!()
}

/// `OpenUserLog`
///
/// ```text
/// bool Function(string asLogName) native global
/// ```
fn open_user_log(_caller: Caller<'_, State>, _log_name: Object) -> Result<bool, Exception> {
    Ok(true)
}

/// `QuitGame`
///
/// ```text
/// Function() native global
/// ```
fn quit_game(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `SetFootIK`
///
/// ```text
/// Function(bool abFootIK) native global
/// ```
fn set_foot_ik(caller: Caller<'_, State>, ab_foot_ik: bool) -> Result<(), Exception> {
    todo!()
}

/// `SetGodMode`
///
/// ```text
/// Function(bool abGodMode) native global
/// ```
fn set_god_mode(caller: Caller<'_, State>, ab_god_mode: bool) -> Result<(), Exception> {
    todo!()
}

/// `StartScriptProfiling`
///
/// ```text
/// Function(string asScriptName) native global
/// ```
fn start_script_profiling(
    _caller: Caller<'_, State>,
    _script_name: Object,
) -> Result<(), Exception> {
    Ok(())
}

/// `StartStackProfiling`
///
/// ```text
/// Function() native global
/// ```
fn start_stack_profiling(_caller: Caller<'_, State>) -> Result<(), Exception> {
    Ok(())
}

/// `StartStackRootProfiling`
///
/// ```text
/// Function(string asScriptName, ScriptObject akObj) native global
/// ```
fn start_stack_root_profiling(
    _caller: Caller<'_, State>,
    _script_name: Object,
    _object: Object,
) -> Result<(), Exception> {
    Ok(())
}

/// `StopScriptProfiling`
///
/// ```text
/// Function(string asScriptName) native global
/// ```
fn stop_script_profiling(
    _caller: Caller<'_, State>,
    _script_name: Object,
) -> Result<(), Exception> {
    Ok(())
}

/// `StopStackProfiling`
///
/// ```text
/// Function() native global
/// ```
fn stop_stack_profiling(_caller: Caller<'_, State>) -> Result<(), Exception> {
    Ok(())
}

/// `StopStackRootProfiling`
///
/// ```text
/// Function(string asScriptName, ScriptObject akObj) native global
/// ```
fn stop_stack_root_profiling(
    _caller: Caller<'_, State>,
    _script_name: Object,
    _object: Object,
) -> Result<(), Exception> {
    Ok(())
}

/// `Trace`
///
/// ```text
/// Function(string asTextToPrint, int aiSeverity) native global
/// ```
fn trace(caller: Caller<'_, State>, text: Object, severity: i32) -> Result<(), Exception> {
    let msg = unsafe { caller.string(text) };

    match severity {
        0 => {
            tracing::info!("{}", msg);
        }
        1 => {
            tracing::warn!("{}", msg);
        }
        2 => {
            tracing::error!("{}", msg);
        }
        _ => {
            tracing::info!("[UNKNOWN SEVERITY] {}", msg);
        }
    }

    Ok(())
}

/// `TraceFunction`
///
/// ```text
/// Function(string asTextToPrint, int aiSeverity) native global
/// ```
fn trace_function(caller: Caller<'_, State>, text: Object, severity: i32) -> Result<(), Exception> {
    trace(caller, text, severity)
}

/// `TraceStack`
///
/// ```text
/// Function(string asTextToPrint, int aiSeverity) native global
/// ```
fn trace_stack(caller: Caller<'_, State>, text: Object, severity: i32) -> Result<(), Exception> {
    trace(caller, text, severity)
}

/// `TraceUser`
///
/// ```text
/// bool Function(string asUserLog, string asTextToPrint, int aiSeverity) native global
/// ```
fn trace_user(
    caller: Caller<'_, State>,
    _user_log: Object,
    text: Object,
    severity: i32,
) -> Result<bool, Exception> {
    trace(caller, text, severity)?;
    Ok(true)
}

/// `ShowRefPosition`
///
/// ```text
/// Function(ObjectReference arRef) native global
/// ```
fn show_ref_position(caller: Caller<'_, State>, ar_ref: Object) -> Result<(), Exception> {
    todo!()
}

/// `DBSendPlayerPosition`
///
/// ```text
/// Function() native global
/// ```
fn db_send_player_position(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}
