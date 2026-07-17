use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("F4SE", "GetVersion", get_version);
    engine.register_if_exists("F4SE", "GetVersionMinor", get_version_minor);
    engine.register_if_exists("F4SE", "GetVersionBeta", get_version_beta);
    engine.register_if_exists("F4SE", "GetVersionRelease", get_version_release);
    engine.register_if_exists("F4SE", "GetPluginVersion", get_plugin_version);
}

/// `GetVersion`
///
/// ```text
/// int Function GetVersion() native global
/// ```
fn get_version(caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}

/// `GetVersionMinor`
///
/// ```text
/// int Function GetVersionMinor() native global
/// ```
fn get_version_minor(caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}

/// `GetVersionBeta`
///
/// ```text
/// int Function GetVersionBeta() native global
/// ```
fn get_version_beta(caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}

/// `GetVersionRelease`
///
/// ```text
/// int Function GetVersionRelease() native global
/// ```
fn get_version_release(caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}

/// `GetPluginVersion`
///
/// ```text
/// int Function GetPluginVersion(string name) native global
/// ```
fn get_plugin_version(caller: Caller<'_, State>, name: Object) -> Result<i32, Error> {
    todo!()
}
