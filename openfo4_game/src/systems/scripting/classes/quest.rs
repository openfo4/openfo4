use event_listener::Event;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::global::{QuestEvent, QuestState};
use crate::systems::quests::StageId;
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
fn get_current_stage_id(caller: Caller<'_, State>, quest: Object) -> i32 {
    let quest = caller.data().record(quest);

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.cur_stage.into(),
        // Quest is not loaded, meaning it is stopped.
        None => 0,
    }
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
fn is_active(caller: Caller<'_, State>, quest: Object) -> bool {
    let quest = caller.data().record(quest);

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.is_active,
        // Quest is not loaded, meaning it is stopped.
        None => false,
    }
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
fn is_running(caller: Caller<'_, State>, quest: Object) -> bool {
    let quest = caller.data().record(quest);

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.state == QuestState::Running,
        // Quest is not loaded, meaning it is stopped.
        None => false,
    }
}

/// `IsStageDone`
///
/// ```text
/// bool Function IsStageDone(int aiStage) native
/// ```
fn is_stage_done(caller: Caller<'_, State>, quest: Object, stage: Int) -> bool {
    let quest = caller.data().record(quest);

    let Ok(stage) = StageId::try_from(stage) else {
        return false;
    };

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.stages_done.contains(&stage),
        // Quest is not loaded, meaning it is stopped.
        None => false,
    }
}

/// `IsStarting`
///
/// ```text
/// bool Function IsStarting() native
/// ```
fn is_starting(caller: Caller<'_, State>, quest: Object) -> bool {
    let quest = caller.data().record(quest);

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.state == QuestState::Starting,
        // Quest is not loaded, meaning it is stopped.
        None => false,
    }
}

/// `IsStopping`
///
/// ```text
/// bool Function IsStopping() native
/// ```
fn is_stopping(caller: Caller<'_, State>, quest: Object) -> bool {
    let quest = caller.data().record(quest);

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.state == QuestState::Stopping,
        // Quest is not loaded, meaning it is stopped.
        None => false,
    }
}

/// `IsStopped`
///
/// ```text
/// bool Function IsStopped() native
/// ```
fn is_stopped(caller: Caller<'_, State>, quest: Object) -> bool {
    let quest = caller.data().record(quest);

    let quests = caller.data().globals.quests.quests.lock();
    match quests.get(&quest) {
        Some(quest) => quest.state == QuestState::Stopped,
        // Quest is not loaded, meaning it is stopped.
        None => true,
    }
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
async fn set_current_stage_id(caller: Caller<'_, State>, quest: Object, stage: i32) -> bool {
    let quest = caller.data().record(quest);

    let Ok(stage) = StageId::try_from(stage) else {
        return false;
    };

    // Note that calling this function while the quest is stopped will
    // start it before setting the stage.
    // We cannot do this ourselves because this is an asynchronous operation
    // and another thread may immediately stop the quest again before we
    // submit our `SetStage` event.
    // The `QuestManager` needs to start the quest for us.
    let waker = Event::with_tag();
    let listen = waker.listen();
    caller
        .data()
        .globals
        .quests
        .events
        .push(QuestEvent::SetStage {
            quest,
            stage,
            waker,
        });

    listen.await
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
async fn start(caller: Caller<'_, State>, quest: Object) -> bool {
    let quest = caller.data().record(quest);

    let waker = Event::with_tag();
    let listen = waker.listen();
    caller
        .data()
        .globals
        .quests
        .events
        .push(QuestEvent::Start { quest, waker });

    let res = listen.await;
    res
}

/// `Stop`
///
/// ```text
/// Function Stop() native
/// ```
async fn stop(caller: Caller<'_, State>, quest: Object) {
    let quest = caller.data().record(quest);

    let waker = Event::new();
    let listen = waker.listen();
    caller
        .data()
        .globals
        .quests
        .events
        .push(QuestEvent::Stop { quest, waker });

    listen.await;
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
