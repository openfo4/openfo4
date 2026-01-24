use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Scene", "ForceStart", force_start);
    engine.register_native_function("Scene", "Start", start);
    engine.register_native_function("Scene", "Stop", stop);
    engine.register_native_function("Scene", "IsPlaying", is_playing);
    engine.register_native_function("Scene", "GetOwningQuest", get_owning_quest);
    engine.register_native_function("Scene", "IsActionComplete", is_action_complete);
    engine.register_native_function("Scene", "Pause", pause);
}

/// `ForceStart`
///
/// ```text
/// Function() native
/// ```
fn force_start(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Start`
///
/// ```text
/// Function() native
/// ```
fn start(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Stop`
///
/// ```text
/// Function() native
/// ```
fn stop(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `IsPlaying`
///
/// ```text
/// bool Function() native
/// ```
fn is_playing(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function() native
/// ```
fn get_owning_quest(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `IsActionComplete`
///
/// ```text
/// bool Function(int aiActionID) native
/// ```
fn is_action_complete(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_action_id: i32,
) -> Result<bool, Exception> {
    todo!()
}

/// `Pause`
///
/// ```text
/// Function(bool abPause) native
/// ```
fn pause(caller: Caller<'_, State>, self_ptr: Object, ab_pause: bool) -> Result<(), Exception> {
    todo!()
}
