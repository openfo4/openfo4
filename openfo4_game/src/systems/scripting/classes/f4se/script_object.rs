use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ScriptObject", "RegisterForKey", register_for_key);
    engine.register_if_exists("ScriptObject", "UnregisterForKey", unregister_for_key);
    engine.register_if_exists("ScriptObject", "RegisterForControl", register_for_control);
    engine.register_if_exists("ScriptObject", "UnregisterForControl", unregister_for_control);
    engine.register_if_exists("ScriptObject", "RegisterForExternalEvent", register_for_external_event);
    engine.register_if_exists("ScriptObject", "UnregisterForExternalEvent", unregister_for_external_event);
    engine.register_if_exists("ScriptObject", "RegisterForCameraState", register_for_camera_state);
    engine.register_if_exists("ScriptObject", "UnregisterForCameraState", unregister_for_camera_state);
    engine.register_if_exists("ScriptObject", "RegisterForFurnitureEvent", register_for_furniture_event);
    engine.register_if_exists("ScriptObject", "UnregisterForFurnitureEvent", unregister_for_furniture_event);
}

/// `RegisterForKey`
///
/// ```text
/// Function RegisterForKey(int key) native
/// ```
fn register_for_key(_caller: Caller<'_, State>, _self_ptr: Object, _key: i32) -> Result<(), Error> {
    todo!()
}

/// `UnregisterForKey`
///
/// ```text
/// Function UnregisterForKey(int key) native
/// ```
fn unregister_for_key(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _key: i32,
) -> Result<(), Error> {
    todo!()
}

/// `RegisterForControl`
///
/// ```text
/// Function RegisterForControl(string control) native
/// ```
fn register_for_control(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _control: Object,
) -> Result<(), Error> {
    todo!()
}

/// `UnregisterForControl`
///
/// ```text
/// Function UnregisterForControl(string control) native
/// ```
fn unregister_for_control(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _control: Object,
) -> Result<(), Error> {
    todo!()
}

/// `RegisterForExternalEvent`
///
/// ```text
/// Function RegisterForExternalEvent(string eventName, string callback) native
/// ```
fn register_for_external_event(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _event_name: Object,
    _callback: Object,
) -> Result<(), Error> {
    todo!()
}

/// `UnregisterForExternalEvent`
///
/// ```text
/// Function UnregisterForExternalEvent(string eventName) native
/// ```
fn unregister_for_external_event(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _event_name: Object,
) -> Result<(), Error> {
    todo!()
}

/// `RegisterForCameraState`
///
/// ```text
/// Function RegisterForCameraState() native
/// ```
fn register_for_camera_state(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `UnregisterForCameraState`
///
/// ```text
/// Function UnregisterForCameraState() native
/// ```
fn unregister_for_camera_state(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `RegisterForFurnitureEvent`
///
/// ```text
/// Function RegisterForFurnitureEvent(Var filter) native
/// ```
fn register_for_furniture_event(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _filter: Object,
) -> Result<(), Error> {
    todo!()
}

/// `UnregisterForFurnitureEvent`
///
/// ```text
/// Function UnregisterForFurnitureEvent(Var filter) native
/// ```
fn unregister_for_furniture_event(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _filter: Object,
) -> Result<(), Error> {
    todo!()
}
