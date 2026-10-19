use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::quests::QuestState;
use crate::systems::scripting::EngineExt;
use crate::systems::scripting::state::State;

use super::{Int, Object};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Quest", "CompleteAllObjectives", complete_all_objectives);
    engine.register_if_exists("Quest", "CompleteQuest", complete_quest);
    engine.register_if_exists("Quest", "FailAllObjectives", fail_all_objectives);
    engine.register_if_exists("Quest", "GetAlias", get_alias);
    engine.register_if_exists("Quest", "GetCurrentStageID", get_current_stage_id);
    engine.register_if_exists("Quest", "HasObjective", has_objective);
    engine.register_if_exists("Quest", "IsActive", is_active);
    engine.register_if_exists("Quest", "IsCompleted", is_completed);
    engine.register_if_exists("Quest", "IsObjectiveCompleted", is_objective_completed);
    engine.register_if_exists("Quest", "IsObjectiveDisplayed", is_objective_displayed);
    engine.register_if_exists("Quest", "IsObjectiveFailed", is_objective_failed);
    engine.register_if_exists("Quest", "IsRunning", is_running);
    engine.register_if_exists("Quest", "IsStageDone", is_stage_done);
    engine.register_if_exists("Quest", "IsStarting", is_starting);
    engine.register_if_exists("Quest", "IsStopping", is_stopping);
    engine.register_if_exists("Quest", "IsStopped", is_stopped);
    engine.register_if_exists("Quest", "Reset", reset);
    engine.register_if_exists("Quest", "ResetSpeechChallenges", reset_speech_challenges);
    engine.register_if_exists("Quest", "SetActive", set_active);
    engine.register_if_exists("Quest", "SetCurrentStageID", set_current_stage_id);
    engine.register_if_exists("Quest", "SetObjectiveCompleted", set_objective_completed);
    engine.register_if_exists("Quest", "SetObjectiveDisplayed", set_objective_displayed);
    engine.register_if_exists("Quest", "SetObjectiveFailed", set_objective_failed);
    engine.register_if_exists("Quest", "Start", start);
    engine.register_if_exists("Quest", "stop", stop);
    engine.register_if_exists("Quest", "UpdateCurrentInstanceGlobal", update_current_instance_global);
}

/// `CompleteAllObjectives`
///
/// ```text
/// Function CompleteAllObjectives() native
/// ```
fn complete_all_objectives(_caller: Caller<'_, State>, _quest: Object) -> Result<(), Error> {
    todo!()
}

/// `CompleteQuest`
///
/// ```text
/// CompleteQuest
/// ```
fn complete_quest(_caller: Caller<'_, State>, _quest: Object) -> Result<(), Error> {
    todo!()
}

/// `FailAllObjectives`
///
/// ```text
/// Function FailAllObjectives() native
/// ```
fn fail_all_objectives(_caller: Caller<'_, State>, _quest: Object) -> Result<(), Error> {
    todo!()
}

/// `GetAlias`
///
/// ```text
/// Alias Function GetAlias(int aiAliasID) native
/// ```
fn get_alias(_caller: Caller<'_, State>, _quest: Object, _alias_id: Int) -> Result<Object, Error> {
    todo!()
}

/// `GetCurrentStageID`
///
/// ```text
/// int Function GetCurrentStageID() native
/// ```
async fn get_current_stage_id(mut caller: Caller<'_, State>, quest: Object) -> Result<Int, Error> {
    let id = caller.data().record(quest);

    let quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| todo!())?;
    Ok(quest.active_stage.into())
}

/// `HasObjective`
///
/// ```text
/// bool Function HasObjective(int aiObjective) native
/// ```
fn has_objective(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
) -> Result<bool, Error> {
    todo!()
}

/// `IsActive`
///
/// ```text
/// bool Function IsActive() native
/// ```
fn is_active(_caller: Caller<'_, State>, _quest: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsCompleted`
///
/// ```text
/// bool Function IsCompleted() native
/// ```
fn is_completed(_caller: Caller<'_, State>, _quest: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsObjectiveCompleted`
///
/// ```text
/// bool Function IsObjectiveCompleted(int aiObjective) native
/// ```
fn is_objective_completed(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
) -> Result<bool, Error> {
    todo!()
}

/// `IsObjectiveDisplayed`
///
/// ```text
/// bool Function IsObjectiveDisplayed(int aiObjective) native
/// ```
fn is_objective_displayed(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
) -> Result<bool, Error> {
    todo!()
}

/// `IsObjectiveFailed`
///
/// ```text
/// bool Function IsObjectiveFailed(int aiObjective) native
/// ```
fn is_objective_failed(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
) -> Result<bool, Error> {
    todo!()
}

/// `IsRunning`
///
/// ```text
/// bool Function IsRunning() native
/// ```
async fn is_running(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Error> {
    let id = caller.data().record(quest);

    let quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| todo!())?;
    Ok(matches!(quest.state, QuestState::Running))
}

/// `IsStageDone`
///
/// ```text
/// bool Function IsStageDone(int aiStage) native
/// ```
async fn is_stage_done(
    mut caller: Caller<'_, State>,
    quest: Object,
    stage: Int,
) -> Result<bool, Error> {
    let id = caller.data().record(quest);

    let quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| Error::msg("no quest"))?;

    let Ok(stage) = u16::try_from(stage) else {
        return Ok(false);
    };

    let Some(stage) = quest.stages.get(&stage) else {
        return Ok(false);
    };

    Ok(stage.is_done)
}

/// `IsStarting`
///
/// ```text
/// bool Function IsStarting() native
/// ```
async fn is_starting(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Error> {
    let id = caller.data().record(quest);

    let quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| todo!())?;
    Ok(matches!(quest.state, QuestState::Starting(_)))
}

/// `IsStopping`
///
/// ```text
/// bool Function IsStopping() native
/// ```
async fn is_stopping(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Error> {
    let id = caller.data().record(quest);

    let quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| todo!())?;
    Ok(matches!(quest.state, QuestState::Stopping(_)))
}

/// `IsStopped`
///
/// ```text
/// bool Function IsStopped() native
/// ```
async fn is_stopped(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Error> {
    let id = caller.data().record(quest);

    let quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| todo!())?;
    Ok(matches!(quest.state, QuestState::Stopped))
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(_caller: Caller<'_, State>, _quest: Object) -> Result<(), Error> {
    todo!()
}

/// `ResetSpeechChallenges`
///
/// ```text
/// Function ResetSpeechChallenges() native
/// ```
fn reset_speech_challenges(_caller: Caller<'_, State>, _quest: Object) -> Result<(), Error> {
    todo!()
}

/// `SetActive`
///
/// ```text
/// Function SetActive(bool abActive = true) native
/// ```
fn set_active(_caller: Caller<'_, State>, _quest: Object, _active: bool) -> Result<(), Error> {
    todo!()
}

/// `SetCurrentStageID`
///
/// ```text
/// bool Function SetCurrentStageID(int aiStageID) native
/// ```
async fn set_current_stage_id(
    mut caller: Caller<'_, State>,
    quest: Object,
    stage: Int,
) -> Result<bool, Error> {
    let Ok(stage_id) = stage.try_into() else {
        return Ok(false);
    };

    let id = caller.data().record(quest);
    let mut quests = caller.data_mut().quests().await;
    let quest = quests.get(id).ok_or_else(|| todo!())?;
    if !quest.stages.contains_key(&stage_id) {
        return Ok(false);
    }

    quests.set_quest_stage(id, stage_id);
    Ok(true)
}

/// `SetObjectiveCompleted`
///
/// ```text
/// Function SetObjectiveCompleted(int aiObjective, bool abCompleted = true) native
/// ```
fn set_objective_completed(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
    _completed: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetObjectiveDisplayed`
///
/// ```text
/// Function SetObjectiveDisplayed(int aiObjective, bool abDisplayed = true, bool abForce = false) native
/// ```
fn set_objective_displayed(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
    _displayed: bool,
    _force: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetObjectiveFailed`
///
/// ```text
/// Function SetObjectiveFailed(int aiObjective, bool abFailed = true) native
/// ```
fn set_objective_failed(
    _caller: Caller<'_, State>,
    _quest: Object,
    _objective: Int,
    _failed: bool,
) -> Result<(), Error> {
    todo!()
}

/// `Start`
///
/// ```text
/// bool Function Start() native
/// ```
async fn start(mut caller: Caller<'_, State>, quest: Object) -> Result<bool, Error> {
    let id = caller.data().record(quest);

    let mut quests = caller.data_mut().quests().await;
    quests.start_quest(id);
    // FIXME: Should we returns false if quest already running?
    Ok(true)
}

/// `Stop`
///
/// ```text
/// Function Stop() native
/// ```
async fn stop(mut caller: Caller<'_, State>, quest: Object) -> Result<(), Error> {
    let id = caller.data().record(quest);

    let mut quests = caller.data_mut().quests().await;
    quests.stop_quest(id);
    Ok(())
}

/// `UpdateCurrentInstanceGlobal`
///
/// ```text
/// bool Function UpdateCurrentInstanceGlobal(GlobalVariable aUpdateGlobal) native
/// ```
fn update_current_instance_global(
    _caller: Caller<'_, State>,
    _quest: Object,
    _gobal: Object,
) -> Result<bool, Error> {
    todo!()
}
