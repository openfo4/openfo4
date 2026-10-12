use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("UI", "IsMenuOpen", is_menu_open);
    engine.register_if_exists("UI", "IsMenuRegistered", is_menu_registered);
    engine.register_if_exists("UI", "RegisterCustomMenu", register_custom_menu);
    engine.register_if_exists("UI", "OpenMenu", open_menu);
    engine.register_if_exists("UI", "CloseMenu", close_menu);
    engine.register_if_exists("UI", "Set", set);
    engine.register_if_exists("UI", "Get", get);
    engine.register_if_exists("UI", "Invoke", invoke);
    engine.register_if_exists("UI", "Load", load);
}

/// `IsMenuOpen`
///
/// ```text
/// bool Function(string menu) native global
/// ```
fn is_menu_open(_caller: Caller<'_, State>, _menu: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsMenuRegistered`
///
/// ```text
/// bool Function(string menu) native global
/// ```
fn is_menu_registered(_caller: Caller<'_, State>, _menu: Object) -> Result<bool, Error> {
    todo!()
}

/// `RegisterCustomMenu`
///
/// ```text
/// bool Function(string menuName, string menuPath, string rootPath, MenuData mData) native global
/// ```
fn register_custom_menu(
    _caller: Caller<'_, State>,
    _menu_name: Object,
    _menu_path: Object,
    _root_path: Object,
    _m_data: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `OpenMenu`
///
/// ```text
/// bool Function(string menuName) native global
/// ```
fn open_menu(_caller: Caller<'_, State>, _menu_name: Object) -> Result<bool, Error> {
    todo!()
}

/// `CloseMenu`
///
/// ```text
/// bool Function(string menuName) native global
/// ```
fn close_menu(_caller: Caller<'_, State>, _menu_name: Object) -> Result<bool, Error> {
    todo!()
}

/// `Set`
///
/// ```text
/// bool Function(string menu, string path, Var arg) native global
/// ```
fn set(
    _caller: Caller<'_, State>,
    _menu: Object,
    _path: Object,
    _arg: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `Get`
///
/// ```text
/// Var Function(string menu, string path) native global
/// ```
fn get(_caller: Caller<'_, State>, _menu: Object, _path: Object) -> Result<Object, Error> {
    todo!()
}

/// `Invoke`
///
/// ```text
/// Var Function(string menu, string path, Var[] args) native global
/// ```
fn invoke(
    _caller: Caller<'_, State>,
    _menu: Object,
    _path: Object,
    _args: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `Load`
///
/// ```text
/// bool Function(string menu, string sourceVar, string assetPath, ScriptObject receiver, string callback) native global
/// ```
fn load(
    _caller: Caller<'_, State>,
    _menu: Object,
    _source_var: Object,
    _asset_path: Object,
    _receiver: Object,
    _callback: Object,
) -> Result<bool, Error> {
    todo!()
}
