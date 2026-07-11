use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{EngineExt, Object, State};

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
fn register_for_key(
    caller: Caller<'_, State>,
    self_ptr: Object,
    key: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForKey`
///
/// ```text
/// Function UnregisterForKey(int key) native
/// ```
fn unregister_for_key(
    caller: Caller<'_, State>,
    self_ptr: Object,
    key: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForControl`
///
/// ```text
/// Function RegisterForControl(string control) native
/// ```
fn register_for_control(
    caller: Caller<'_, State>,
    self_ptr: Object,
    control: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForControl`
///
/// ```text
/// Function UnregisterForControl(string control) native
/// ```
fn unregister_for_control(
    caller: Caller<'_, State>,
    self_ptr: Object,
    control: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForExternalEvent`
///
/// ```text
/// Function RegisterForExternalEvent(string eventName, string callback) native
/// ```
fn register_for_external_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    event_name: Object,
    callback: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForExternalEvent`
///
/// ```text
/// Function UnregisterForExternalEvent(string eventName) native
/// ```
fn unregister_for_external_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    event_name: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForCameraState`
///
/// ```text
/// Function RegisterForCameraState() native
/// ```
fn register_for_camera_state(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForCameraState`
///
/// ```text
/// Function UnregisterForCameraState() native
/// ```
fn unregister_for_camera_state(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RegisterForFurnitureEvent`
///
/// ```text
/// Function RegisterForFurnitureEvent(Var filter) native
/// ```
fn register_for_furniture_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    filter: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `UnregisterForFurnitureEvent`
///
/// ```text
/// Function UnregisterForFurnitureEvent(Var filter) native
/// ```
fn unregister_for_furniture_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    filter: Object,
) -> Result<(), Exception> {
    todo!()
}
