use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::effects::{StartQuestSceneEffect, StopQuestSceneEffect};
use crate::systems::scripting::state::State;

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
fn force_start(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Start`
///
/// ```text
/// Function() native
/// ```
async fn start(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    let scene = caller.data().record(self_ptr);

    caller
        .data_mut()
        .run_effect_and_wait(StartQuestSceneEffect { scene })
        .await?;

    Ok(())
}

/// `Stop`
///
/// ```text
/// Function() native
/// ```
async fn stop(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    let scene = caller.data().record(self_ptr);

    caller
        .data_mut()
        .run_effect_and_wait(StopQuestSceneEffect { scene })
        .await?;

    Ok(())
}

/// `IsPlaying`
///
/// ```text
/// bool Function() native
/// ```
fn is_playing(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function() native
/// ```
fn get_owning_quest(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
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
) -> Result<bool, Error> {
    todo!()
}

/// `Pause`
///
/// ```text
/// Function(bool abPause) native
/// ```
fn pause(caller: Caller<'_, State>, self_ptr: Object, ab_pause: bool) -> Result<(), Error> {
    todo!()
}
