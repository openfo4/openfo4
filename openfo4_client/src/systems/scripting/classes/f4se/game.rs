use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Game", "GetCurrentConsoleRef", get_current_console_ref);
    engine.register_if_exists("Game", "GetInstalledPlugins", get_installed_plugins);
    engine.register_if_exists("Game", "GetInstalledLightPlugins", get_installed_light_plugins);
    engine.register_if_exists("Game", "GetPluginDependencies", get_plugin_dependencies);
    engine.register_if_exists("Game", "SetGameSettingFloat", set_game_setting_float);
    engine.register_if_exists("Game", "SetGameSettingInt", set_game_setting_int);
    engine.register_if_exists("Game", "SetGameSettingBool", set_game_setting_bool);
    engine.register_if_exists("Game", "SetGameSettingString", set_game_setting_string);
    engine.register_if_exists("Game", "UpdateThirdPerson", update_third_person);
    engine.register_if_exists("Game", "GetCameraState", get_camera_state);
}

/// `GetCurrentConsoleRef`
///
/// ```text
/// ObjectReference Function GetCurrentConsoleRef() native global
/// ```
fn get_current_console_ref(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetInstalledPlugins`
///
/// ```text
/// PluginInfo[] Function GetInstalledPlugins() native global
/// ```
fn get_installed_plugins(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetInstalledLightPlugins`
///
/// ```text
/// PluginInfo[] Function GetInstalledLightPlugins() native global
/// ```
fn get_installed_light_plugins(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetPluginDependencies`
///
/// ```text
/// string[] Function GetPluginDependencies(string plugin) native global
/// ```
fn get_plugin_dependencies(caller: Caller<'_, State>, plugin: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetGameSettingFloat`
///
/// ```text
/// Function SetGameSettingFloat(string setting, float value) native global
/// ```
fn set_game_setting_float(
    caller: Caller<'_, State>,
    setting: Object,
    value: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetGameSettingInt`
///
/// ```text
/// Function SetGameSettingInt(string setting, int value) native global
/// ```
fn set_game_setting_int(
    caller: Caller<'_, State>,
    setting: Object,
    value: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetGameSettingBool`
///
/// ```text
/// Function SetGameSettingBool(string setting, bool value) native global
/// ```
fn set_game_setting_bool(
    caller: Caller<'_, State>,
    setting: Object,
    value: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetGameSettingString`
///
/// ```text
/// Function SetGameSettingString(string setting, string value) native global
/// ```
fn set_game_setting_string(
    caller: Caller<'_, State>,
    setting: Object,
    value: Object,
) -> Result<(), Error> {
    todo!()
}

/// `UpdateThirdPerson`
///
/// ```text
/// Function UpdateThirdPerson() native global
/// ```
fn update_third_person(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `GetCameraState`
///
/// ```text
/// int Function GetCameraState() native global
/// ```
fn get_camera_state(caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}
