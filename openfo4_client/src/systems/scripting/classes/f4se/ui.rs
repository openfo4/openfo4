use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

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
fn is_menu_open(caller: Caller<'_, State>, menu: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsMenuRegistered`
///
/// ```text
/// bool Function(string menu) native global
/// ```
fn is_menu_registered(caller: Caller<'_, State>, menu: Object) -> Result<bool, Error> {
    todo!()
}

/// `RegisterCustomMenu`
///
/// ```text
/// bool Function(string menuName, string menuPath, string rootPath, MenuData mData) native global
/// ```
fn register_custom_menu(
    caller: Caller<'_, State>,
    menu_name: Object,
    menu_path: Object,
    root_path: Object,
    m_data: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `OpenMenu`
///
/// ```text
/// bool Function(string menuName) native global
/// ```
fn open_menu(caller: Caller<'_, State>, menu_name: Object) -> Result<bool, Error> {
    todo!()
}

/// `CloseMenu`
///
/// ```text
/// bool Function(string menuName) native global
/// ```
fn close_menu(caller: Caller<'_, State>, menu_name: Object) -> Result<bool, Error> {
    todo!()
}

/// `Set`
///
/// ```text
/// bool Function(string menu, string path, Var arg) native global
/// ```
fn set(caller: Caller<'_, State>, menu: Object, path: Object, arg: Object) -> Result<bool, Error> {
    todo!()
}

/// `Get`
///
/// ```text
/// Var Function(string menu, string path) native global
/// ```
fn get(caller: Caller<'_, State>, menu: Object, path: Object) -> Result<Object, Error> {
    todo!()
}

/// `Invoke`
///
/// ```text
/// Var Function(string menu, string path, Var[] args) native global
/// ```
fn invoke(
    caller: Caller<'_, State>,
    menu: Object,
    path: Object,
    args: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `Load`
///
/// ```text
/// bool Function(string menu, string sourceVar, string assetPath, ScriptObject receiver, string callback) native global
/// ```
fn load(
    caller: Caller<'_, State>,
    menu: Object,
    source_var: Object,
    asset_path: Object,
    receiver: Object,
    callback: Object,
) -> Result<bool, Error> {
    todo!()
}
