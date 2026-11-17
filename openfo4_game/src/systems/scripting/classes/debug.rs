use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Debug", "CenterOnCell", center_on_cell);
    engine.register_if_exists("Debug", "CenterOnCellAndWait", center_on_cell_and_wait);
    engine.register_if_exists("Debug", "PlayerMoveToAndWait", player_move_to_and_wait);
    engine.register_if_exists("Debug", "CloseUserLog", close_user_log);
    engine.register_if_exists("Debug", "DumpAliasData", dump_alias_data);
    engine.register_if_exists("Debug", "DumpEventRegistrations", dump_event_registrations);
    engine.register_if_exists("Debug", "EnableAI", enable_ai);
    engine.register_if_exists("Debug", "EnableCollisions", enable_collisions);
    engine.register_if_exists("Debug", "EnableDetection", enable_detection);
    engine.register_if_exists("Debug", "EnableMenus", enable_menus);
    engine.register_if_exists("Debug", "GetConfigName", get_config_name);
    engine.register_if_exists("Debug", "GetPlatformName", get_platform_name);
    engine.register_if_exists("Debug", "GetVersionNumber", get_version_number);
    engine.register_if_exists("Debug", "MessageBox", message_box);
    engine.register_if_exists("Debug", "Notification", notification);
    engine.register_if_exists("Debug", "OpenUserLog", open_user_log);
    engine.register_if_exists("Debug", "QuitGame", quit_game);
    engine.register_if_exists("Debug", "SetFootIK", set_foot_ik);
    engine.register_if_exists("Debug", "SetGodMode", set_god_mode);
    engine.register_if_exists("Debug", "StartScriptProfiling", start_script_profiling);
    engine.register_if_exists("Debug", "StartStackProfiling", start_stack_profiling);
    engine.register_if_exists("Debug", "StartStackRootProfiling", start_stack_root_profiling);
    engine.register_if_exists("Debug", "StopScriptProfiling", stop_script_profiling);
    engine.register_if_exists("Debug", "StopStackProfiling", stop_stack_profiling);
    engine.register_if_exists("Debug", "StopStackRootProfiling", stop_stack_root_profiling);
    engine.register_if_exists("Debug", "Trace", trace);
    engine.register_if_exists("Debug", "TraceFunction", trace_function);
    engine.register_if_exists("Debug", "TraceStack", trace_stack);
    engine.register_if_exists("Debug", "TraceUser", trace_user);
    engine.register_if_exists("Debug", "ShowRefPosition", show_ref_position);
    engine.register_if_exists("Debug", "DBSendPlayerPosition", db_send_player_position);
}

/// `CenterOnCell`
///
/// ```text
/// Function(string asCellname) native global
/// ```
fn center_on_cell(_caller: Caller<'_, State>, _as_cellname: Object) -> Result<(), Error> {
    todo!()
}

/// `CenterOnCellAndWait`
///
/// ```text
/// float Function(string asCellname) native global
/// ```
fn center_on_cell_and_wait(_caller: Caller<'_, State>, _as_cellname: Object) -> Result<f32, Error> {
    todo!()
}

/// `PlayerMoveToAndWait`
///
/// ```text
/// float Function(string asDestRef) native global
/// ```
fn player_move_to_and_wait(_caller: Caller<'_, State>, _as_dest_ref: Object) -> Result<f32, Error> {
    todo!()
}

/// `CloseUserLog`
///
/// ```text
/// Function(string asLogName) native global
/// ```
fn close_user_log(_caller: Caller<'_, State>, _log_name: Object) {}

/// `DumpAliasData`
///
/// ```text
/// Function(Quest akQuest) native global
/// ```
fn dump_alias_data(_caller: Caller<'_, State>, _quest: Object) {}

/// `DumpEventRegistrations`
///
/// ```text
/// Function(ScriptObject akScript) native global
/// ```
fn dump_event_registrations(_caller: Caller<'_, State>, _script: Object) {}

/// `EnableAI`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_ai(_caller: Caller<'_, State>, _ab_enable: bool) -> Result<(), Error> {
    todo!()
}

/// `EnableCollisions`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_collisions(_caller: Caller<'_, State>, _ab_enable: bool) -> Result<(), Error> {
    todo!()
}

/// `EnableDetection`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_detection(_caller: Caller<'_, State>, _ab_enable: bool) -> Result<(), Error> {
    todo!()
}

/// `EnableMenus`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_menus(_caller: Caller<'_, State>, _ab_enable: bool) -> Result<(), Error> {
    todo!()
}

/// `GetConfigName`
///
/// ```text
/// string Function() native global
/// ```
fn get_config_name(_caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetPlatformName`
///
/// ```text
/// string Function() native global
/// ```
fn get_platform_name(_caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetVersionNumber`
///
/// ```text
/// string Function() native global
/// ```
fn get_version_number(_caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `MessageBox`
///
/// ```text
/// Function(string asMessageBoxText) native global
/// ```
fn message_box(_caller: Caller<'_, State>, _as_message_box_text: Object) -> Result<(), Error> {
    todo!()
}

/// `Notification`
///
/// ```text
/// Function(string asNotificationText) native global
/// ```
fn notification(_caller: Caller<'_, State>, _as_notification_text: Object) -> Result<(), Error> {
    todo!()
}

/// `OpenUserLog`
///
/// ```text
/// bool Function(string asLogName) native global
/// ```
fn open_user_log(_caller: Caller<'_, State>, _log_name: Object) -> bool {
    true
}

/// `QuitGame`
///
/// ```text
/// Function() native global
/// ```
fn quit_game(_caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `SetFootIK`
///
/// ```text
/// Function(bool abFootIK) native global
/// ```
fn set_foot_ik(_caller: Caller<'_, State>, _ab_foot_ik: bool) -> Result<(), Error> {
    todo!()
}

/// `SetGodMode`
///
/// ```text
/// Function(bool abGodMode) native global
/// ```
fn set_god_mode(_caller: Caller<'_, State>, _ab_god_mode: bool) -> Result<(), Error> {
    todo!()
}

/// `StartScriptProfiling`
///
/// ```text
/// Function(string asScriptName) native global
/// ```
fn start_script_profiling(_caller: Caller<'_, State>, _script_name: Object) {}

/// `StartStackProfiling`
///
/// ```text
/// Function() native global
/// ```
fn start_stack_profiling(_caller: Caller<'_, State>) {}

/// `StartStackRootProfiling`
///
/// ```text
/// Function(string asScriptName, ScriptObject akObj) native global
/// ```
fn start_stack_root_profiling(_caller: Caller<'_, State>, _script_name: Object, _object: Object) {}

/// `StopScriptProfiling`
///
/// ```text
/// Function(string asScriptName) native global
/// ```
fn stop_script_profiling(_caller: Caller<'_, State>, _script_name: Object) {}

/// `StopStackProfiling`
///
/// ```text
/// Function() native global
/// ```
fn stop_stack_profiling(_caller: Caller<'_, State>) {}

/// `StopStackRootProfiling`
///
/// ```text
/// Function(string asScriptName, ScriptObject akObj) native global
/// ```
fn stop_stack_root_profiling(_caller: Caller<'_, State>, _script_name: Object, _object: Object) {}

/// `Trace`
///
/// ```text
/// Function(string asTextToPrint, int aiSeverity) native global
/// ```
fn trace(caller: Caller<'_, State>, text: Object, severity: i32) {
    let msg = caller.string(text);

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
}

/// `TraceFunction`
///
/// ```text
/// Function(string asTextToPrint, int aiSeverity) native global
/// ```
fn trace_function(caller: Caller<'_, State>, text: Object, severity: i32) {
    trace(caller, text, severity)
}

/// `TraceStack`
///
/// ```text
/// Function(string asTextToPrint, int aiSeverity) native global
/// ```
fn trace_stack(caller: Caller<'_, State>, text: Object, severity: i32) {
    trace(caller, text, severity)
}

/// `TraceUser`
///
/// ```text
/// bool Function(string asUserLog, string asTextToPrint, int aiSeverity) native global
/// ```
fn trace_user(caller: Caller<'_, State>, _user_log: Object, text: Object, severity: i32) -> bool {
    trace(caller, text, severity);
    true
}

/// `ShowRefPosition`
///
/// ```text
/// Function(ObjectReference arRef) native global
/// ```
fn show_ref_position(_caller: Caller<'_, State>, _object: Object) {}

/// `DBSendPlayerPosition`
///
/// ```text
/// Function() native global
/// ```
fn db_send_player_position(_caller: Caller<'_, State>) {}
