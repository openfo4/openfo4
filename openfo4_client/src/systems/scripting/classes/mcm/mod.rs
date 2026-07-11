use openfo4_script::engine::Engine;

use crate::systems::scripting::classes::EngineExt;

mod mcm;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("MCM", "IsInstalled", mcm::is_installed);
    engine.register_if_exists("MCM", "GetVersionCode", mcm::get_version_code);
    engine.register_if_exists("MCM", "RefreshMenu", mcm::refresh_menu);
    engine.register_if_exists("MCM", "GetModSettingInt", mcm::get_mod_setting_int);
    engine.register_if_exists("MCM", "GetModSettingBool", mcm::get_mod_setting_bool);
    engine.register_if_exists("MCM", "GetModSettingFloat", mcm::get_mod_setting_float);
    engine.register_if_exists("MCM", "GetModSettingString", mcm::get_mod_setting_string);
    engine.register_if_exists("MCM", "SetModSettingInt", mcm::set_mod_setting_int);
    engine.register_if_exists("MCM", "SetModSettingBool", mcm::set_mod_setting_bool);
    engine.register_if_exists("MCM", "SetModSettingFloat", mcm::set_mod_setting_float);
    engine.register_if_exists("MCM", "SetModSettingString", mcm::set_mod_setting_string);
}
