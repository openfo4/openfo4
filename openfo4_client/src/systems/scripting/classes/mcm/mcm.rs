use openfo4_script::runtime::{Caller, Error, ObjectAddr};

use crate::systems::scripting::state::State;

/// `IsIntalled`
///
/// ```text
/// bool Function IsInstalled() native global
/// ```
pub fn is_installed(caller: Caller<'_, State>) -> Result<bool, Error> {
    todo!()
}

/// `GetVersionCode`
///
/// ```text
/// int Function GetVersionCode() native global
/// ```
pub fn get_version_code(caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}

/// `RefreshMenu`
///
/// ```text
/// Function RefreshMenu() native global
/// ```
pub fn refresh_menu(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `GetModSettingInt`
///
/// ```text
/// int Function GetModSettingInt(string asModName, string asSetting) native global
/// ```
pub fn get_mod_setting_int(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
) -> Result<i32, Error> {
    todo!()
}

/// `GetModSettingBool`
///
/// ```text
/// bool Function GetModSettingBool(string asModName, string asSetting) native global
/// ```
pub fn get_mod_setting_bool(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
) -> Result<bool, Error> {
    todo!()
}

/// `GetModSettingFloat`
///
/// ```text
/// float Function GetModSettingFloat(string asModName, string asSetting) native global
/// ```
pub fn get_mod_setting_float(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
) -> Result<f32, Error> {
    todo!()
}

/// `GetModSettingString`
///
/// ```text
/// string Function GetModSettingString(string asModName, string asSetting) native global
/// ```
pub fn get_mod_setting_string(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
) -> Result<ObjectAddr, Error> {
    todo!()
}

/// `SetModSettingInt`
///
/// ```text
/// Function SetModSettingInt(string asModName, string asSettingName, int aiValue) native global
/// ```
pub fn set_mod_setting_int(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
    value: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetModSettingBool`
///
/// ```text
/// Function SetModSettingBool(string asModName, string asSettingName, bool abValue) native global
/// ```
pub fn set_mod_setting_bool(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
    value: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetModSettingFloat`
///
/// ```text
/// Function SetModSettingFloat(string asModName, string asSettingName, float afValue) native global
/// ```
pub fn set_mod_setting_float(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
    value: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetModSettingString`
///
/// ```text
/// Function SetModSettingString(string asModName, string asSettingName, string asValue) native global
/// ```
pub fn set_mod_setting_string(
    caller: Caller<'_, State>,
    mod_name: ObjectAddr,
    setting: ObjectAddr,
    value: ObjectAddr,
) -> Result<(), Error> {
    todo!()
}
