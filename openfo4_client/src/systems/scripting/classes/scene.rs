use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};
use crate::systems::scripting::effects::{StartQuestSceneEffect, StopQuestSceneEffect};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Scene", "ForceStart", force_start);
    engine.register_if_exists("Scene", "Start", start);
    engine.register_if_exists("Scene", "Stop", stop);
    engine.register_if_exists("Scene", "IsPlaying", is_playing);
    engine.register_if_exists("Scene", "GetOwningQuest", get_owning_quest);
    engine.register_if_exists("Scene", "IsActionComplete", is_action_complete);
    engine.register_if_exists("Scene", "Pause", pause);
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
fn start(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    let scene = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    caller
        .data_mut()
        .run_effect_and_wait(StartQuestSceneEffect { scene })?;

    Ok(())
}

/// `Stop`
///
/// ```text
/// Function() native
/// ```
fn stop(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    let scene = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    caller
        .data_mut()
        .run_effect_and_wait(StopQuestSceneEffect { scene })?;

    Ok(())
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
fn get_owning_quest(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    let data = caller.data_mut();

    let scene = data
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
        .ok_or(Exception::Error)?;

    let scene = data.quests().get_scene(scene).ok_or(Exception::Error)?;
    let quest = scene.quest;

    let quest = data
        .objects
        .get_by_record(quest)
        .and_then(|s| s.get("Quest"))
        .ok_or(Exception::Error)?;

    Ok(quest.get().object.addr())
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
