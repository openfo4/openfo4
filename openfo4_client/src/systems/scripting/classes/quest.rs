use openfo4_script::runtime::{Caller, Exception};

use crate::systems::quests::QuestState;

use super::{Int, Object, State};

/// `CompleteAllObjectives`
///
/// ```text
/// Function CompleteAllObjectives() native
/// ```
pub fn complete_all_objectives(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `CompleteQuest`
///
/// ```text
/// CompleteQuest
/// ```
pub fn complete_quest(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `FailAllObjectives`
///
/// ```text
/// Function FailAllObjectives() native
/// ```
pub fn fail_all_objectives(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `GetAlias`
///
/// ```text
/// Alias Function GetAlias(int aiAliasID) native
/// ```
pub fn get_alias(
    caller: Caller<'_, State>,
    quest: Object,
    alias_id: Int,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetCurrentStageID`
///
/// ```text
/// int Function GetCurrentStageID() native
/// ```
pub fn get_current_stage_id(
    mut caller: Caller<'_, State>,
    quest: Object,
) -> Result<Int, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.active_stage.unwrap_or_default().into())
}

/// `HasObjective`
///
/// ```text
/// bool Function HasObjective(int aiObjective) native
/// ```
pub fn has_objective(
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
pub fn is_active(caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsCompleted`
///
/// ```text
/// bool Function IsCompleted() native
/// ```
pub fn is_completed(caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsObjectiveCompleted`
///
/// ```text
/// bool Function IsObjectiveCompleted(int aiObjective) native
/// ```
pub fn is_objective_completed(
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
pub fn is_objective_displayed(
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
pub fn is_objective_failed(
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
pub fn is_running(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Running)
}

/// `IsStageDone`
///
/// ```text
/// bool Function IsStageDone(int aiStage) native
/// ```
pub fn is_stage_done(
    caller: Caller<'_, State>,
    quest: Object,
    stage: Int,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsStarting`
///
/// ```text
/// bool Function IsStarting() native
/// ```
pub fn is_starting(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Starting)
}

/// `IsStopping`
///
/// ```text
/// bool Function IsStopping() native
/// ```
pub fn is_stopping(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Stopping)
}

/// `IsStopped`
///
/// ```text
/// bool Function IsStopped() native
/// ```
pub fn is_stopped(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    let quest = caller.data_mut().quests().get(id).ok_or(Exception::Error)?;
    Ok(quest.state == QuestState::Stopped)
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
pub fn reset(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `ResetSpeechChallenges`
///
/// ```text
/// Function ResetSpeechChallenges() native
/// ```
pub fn reset_speech_challenges(caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    todo!()
}

/// `SetActive`
///
/// ```text
/// Function SetActive(bool abActive = true) native
/// ```
pub fn set_active(caller: Caller<'_, State>, quest: Object, active: bool) -> Result<(), Exception> {
    todo!()
}

/// `SetCurrentStageID`
///
/// ```text
/// bool Function SetCurrentStageID(int aiStageID) native
/// ```
pub fn set_current_stage_id(
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
pub fn set_objective_completed(
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
pub fn set_objective_displayed(
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
pub fn set_objective_failed(
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
pub fn start(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Exception> {
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
pub fn stop(mut caller: Caller<'_, State>, quest: Object) -> Result<(), Exception> {
    let id = caller.data().objects.quest(quest).ok_or(Exception::Error)?;
    caller.data_mut().quests().stop_quest(id);
    Ok(())
}

/// `UpdateCurrentInstanceGlobal`
///
/// ```text
/// bool Function UpdateCurrentInstanceGlobal(GlobalVariable aUpdateGlobal) native
/// ```
pub fn update_current_instance_global(
    caller: Caller<'_, State>,
    quest: Object,
    gobal: Object,
) -> Result<bool, Exception> {
    todo!()
}
