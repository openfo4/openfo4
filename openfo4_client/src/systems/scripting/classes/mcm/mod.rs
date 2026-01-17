use openfo4_script::engine::Engine;

mod mcm;

pub fn register(engine: &mut Engine) {
    engine.register_native_function("MCM", "IsInstalled", mcm::is_installed);
    engine.register_native_function("MCM", "GetVersionCode", mcm::get_version_code);
    engine.register_native_function("MCM", "RefreshMenu", mcm::refresh_menu);
    engine.register_native_function("MCM", "GetModSettingInt", mcm::get_mod_setting_int);
    engine.register_native_function("MCM", "GetModSettingBool", mcm::get_mod_setting_bool);
    engine.register_native_function("MCM", "GetModSettingFloat", mcm::get_mod_setting_float);
    engine.register_native_function("MCM", "GetModSettingString", mcm::get_mod_setting_string);
    engine.register_native_function("MCM", "SetModSettingInt", mcm::set_mod_setting_int);
    engine.register_native_function("MCM", "SetModSettingBool", mcm::set_mod_setting_bool);
    engine.register_native_function("MCM", "SetModSettingFloat", mcm::set_mod_setting_float);
    engine.register_native_function("MCM", "SetModSettingString", mcm::set_mod_setting_string);
}
