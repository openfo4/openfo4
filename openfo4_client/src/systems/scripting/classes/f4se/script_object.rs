use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("ScriptObject", "RegisterForKey", register_for_key);
    engine.register_native_function("ScriptObject", "UnregisterForKey", unregister_for_key);
    engine.register_native_function("ScriptObject", "RegisterForControl", register_for_control);
    engine.register_native_function("ScriptObject", "UnregisterForControl", unregister_for_control);
    engine.register_native_function("ScriptObject", "RegisterForExternalEvent", register_for_external_event);
    engine.register_native_function("ScriptObject", "UnregisterForExternalEvent", unregister_for_external_event);
    engine.register_native_function("ScriptObject", "RegisterForCameraState", register_for_camera_state);
    engine.register_native_function("ScriptObject", "UnregisterForCameraState", unregister_for_camera_state);
    engine.register_native_function("ScriptObject", "RegisterForFurnitureEvent", register_for_furniture_event);
    engine.register_native_function("ScriptObject", "UnregisterForFurnitureEvent", unregister_for_furniture_event);
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
    control: isize,
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
    control: isize,
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
    event_name: isize,
    callback: isize,
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
    event_name: isize,
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
