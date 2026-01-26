use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::quests::QuestState;

use super::{Int, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Quest", "CompleteAllObjectives", complete_all_objectives);
    engine.register_native_function("Quest", "CompleteQuest", complete_quest);
    engine.register_native_function("Quest", "FailAllObjectives", fail_all_objectives);
    engine.register_native_function("Quest", "GetAlias", get_alias);
    engine.register_native_function("Quest", "GetCurrentStageID", get_current_stage_id);
    engine.register_native_function("Quest", "HasObjective", has_objective);
    engine.register_native_function("Quest", "IsActive", is_active);
    engine.register_native_function("Quest", "IsCompleted", is_completed);
    engine.register_native_function("Quest", "IsObjectiveCompleted", is_objective_completed);
    engine.register_native_function("Quest", "IsObjectiveDisplayed", is_objective_displayed);
    engine.register_native_function("Quest", "IsObjectiveFailed", is_objective_failed);
    engine.register_native_function("Quest", "IsRunning", is_running);
    engine.register_native_function("Quest", "IsStageDone", is_stage_done);
    engine.register_native_function("Quest", "IsStarting", is_starting);
    engine.register_native_function("Quest", "IsStopping", is_stopping);
    engine.register_native_function("Quest", "IsStopped", is_stopped);
    engine.register_native_function("Quest", "Reset", reset);
    engine.register_native_function("Quest", "ResetSpeechChallenges", reset_speech_challenges);
    engine.register_native_function("Quest", "SetActive", set_active);
    engine.register_native_function("Quest", "SetCurrentStageID", set_current_stage_id);
    engine.register_native_function("Quest", "SetObjectiveCompleted", set_objective_completed);
    engine.register_native_function("Quest", "SetObjectiveDisplayed", set_objective_displayed);
    engine.register_native_function("Quest", "SetObjectiveFailed", set_objective_failed);
    engine.register_native_function("Quest", "Start", start);
    engine.register_native_function("Quest", "stop", stop);
    engine.register_native_function("Quest", "UpdateCurrentInstanceGlobal", update_current_instance_global);
}

/// `CompleteAllObjectives`
///
/// ```text
/// Function CompleteAllObjectives() native
/// ```
fn complete_all_objectives(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `CompleteQuest`
///
/// ```text
/// CompleteQuest
/// ```
fn complete_quest(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `FailAllObjectives`
///
/// ```text
/// Function FailAllObjectives() native
/// ```
fn fail_all_objectives(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `GetAlias`
///
/// ```text
/// Alias Function GetAlias(int aiAliasID) native
/// ```
fn get_alias(caller: Caller<'_, State>, quest: Object, alias_id: Int) -> Result<Object, Exception> {
    todo!()
}

/// `GetCurrentStageID`
///
/// ```text
/// int Function GetCurrentStageID() native
/// ```
fn get_current_stage_id(mut caller: Caller<'_, State>, quest: Object) -> Result<Int, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.active_stage.unwrap_or_default().into())
}

/// `HasObjective`
///
/// ```text
/// bool Function HasObjective(int aiObjective) native
/// ```
fn has_objective(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsActive`
///
/// ```text
/// bool Function IsActive() native
/// ```
fn is_active(caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsCompleted`
///
/// ```text
/// bool Function IsCompleted() native
/// ```
fn is_completed(caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsObjectiveCompleted`
///
/// ```text
/// bool Function IsObjectiveCompleted(int aiObjective) native
/// ```
fn is_objective_completed(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsObjectiveDisplayed`
///
/// ```text
/// bool Function IsObjectiveDisplayed(int aiObjective) native
/// ```
fn is_objective_displayed(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsObjectiveFailed`
///
/// ```text
/// bool Function IsObjectiveFailed(int aiObjective) native
/// ```
fn is_objective_failed(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsRunning`
///
/// ```text
/// bool Function IsRunning() native
/// ```
fn is_running(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Running)
}

/// `IsStageDone`
///
/// ```text
/// bool Function IsStageDone(int aiStage) native
/// ```
fn is_stage_done(caller: Caller<'_, State>, quest: Object, stage: Int) -> Result<bool, Exception> {
    todo!()
}

/// `IsStarting`
///
/// ```text
/// bool Function IsStarting() native
/// ```
fn is_starting(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Starting)
}

/// `IsStopping`
///
/// ```text
/// bool Function IsStopping() native
/// ```
fn is_stopping(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Stopping)
}

/// `IsStopped`
///
/// ```text
/// bool Function IsStopped() native
/// ```
fn is_stopped(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Stopped)
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `ResetSpeechChallenges`
///
/// ```text
/// Function ResetSpeechChallenges() native
/// ```
fn reset_speech_challenges(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `SetActive`
///
/// ```text
/// Function SetActive(bool abActive = true) native
/// ```
fn set_active(caller: Caller<'_, State>, quest: Object, active: bool) -> Result<(), Exception> {
    todo!()
}

/// `SetCurrentStageID`
///
/// ```text
/// bool Function SetCurrentStageID(int aiStageID) native
/// ```
fn set_current_stage_id(
    mut caller: Caller<'_, State>,
    quest: Object,
    stage: Int,
) -> Result<bool, Exception> {
    let Ok(stage_id) = stage.try_into() else {
        return Ok(false);
    };

    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    if !quest.stages.contains_key(&stage_id) {
        return Ok(false);
    }

    caller.data_mut().quests().set_quest_stage(id, stage_id);
    Ok(true)
}

/// `SetObjectiveCompleted`
///
/// ```text
/// Function SetObjectiveCompleted(int aiObjective, bool abCompleted = true) native
/// ```
fn set_objective_completed(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
    completed: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetObjectiveDisplayed`
///
/// ```text
/// Function SetObjectiveDisplayed(int aiObjective, bool abDisplayed = true, bool abForce = false) native
/// ```
fn set_objective_displayed(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
    displayed: bool,
    force: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetObjectiveFailed`
///
/// ```text
/// Function SetObjectiveFailed(int aiObjective, bool abFailed = true) native
/// ```
fn set_objective_failed(
    caller: Caller<'_, State>,
    quest: Object,
    objective: Int,
    failed: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `Start`
///
/// ```text
/// bool Function Start() native
/// ```
fn start(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    caller.data_mut().quests().start_quest(id);
    // FIXME: Should we returns false if quest already running?
    Ok(true)
}

/// `Stop`
///
/// ```text
/// Function Stop() native
/// ```
fn stop(mut caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    caller.data_mut().quests().stop_quest(id);
    Ok(())
}

/// `UpdateCurrentInstanceGlobal`
///
/// ```text
/// bool Function UpdateCurrentInstanceGlobal(GlobalVariable aUpdateGlobal) native
/// ```
fn update_current_instance_global(
    caller: Caller<'_, State>,
    quest: Object,
    gobal: Object,
) -> Result<bool, Exception> {
    todo!()
}
