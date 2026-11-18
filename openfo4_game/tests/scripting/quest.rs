use openfo4_esp::records::{Quest, QuestStage};
use openfo4_game::systems::scripting::invocations::Value;

use crate::runner::{DefaultExt, TestRunner};

#[test]
fn quest_start_twice() {
    let mut builder = TestRunner::builder();

    let id = builder.add_record(Quest::default());

    let mut runner = builder.build();
    let quest = runner.load(id);

    runner.call_assert(&quest, "Start", [], true);
    runner.call_assert(&quest, "IsRunning", [], true);

    runner.call_assert(&quest, "Start", [], true);
    runner.call_assert(&quest, "IsRunning", [], true);
}

#[test]
fn quest_start_and_stop() {
    let mut builder = TestRunner::builder();

    let id = builder.add_record(Quest::default());

    let mut runner = builder.build();
    let quest = runner.load(id);

    runner.call_assert(&quest, "IsRunning", [], false);
    runner.call_assert(&quest, "Start", [], true);
    runner.call_assert(&quest, "IsRunning", [], true);
    runner.call(&quest, "Stop", []);
    runner.call_assert(&quest, "IsRunning", [], false);
}

#[test]
fn quest_set_stage_starting() {
    let mut builder = TestRunner::builder();

    let id = builder.add_record(Quest::default_with(|quest| {
        let mut stage = QuestStage::default();
        stage.index.stage_index = 20;
        quest.stages.push(stage);
    }));

    let mut runner = builder.build();
    let quest = runner.load(id);

    runner.call_assert(&quest, "SetCurrentStageID", [Value::Int(20)], true);
    runner.call_assert(&quest, "IsRunning", [], true);
    runner.call_assert(&quest, "GetCurrentStageID", [], 20);
}

#[test]
fn quest_set_stage_invalid() {
    let mut builder = TestRunner::builder();

    let id = builder.add_record(Quest::default());

    let mut runner = builder.build();
    let quest = runner.load(id);

    runner.call_assert(&quest, "SetCurrentStageID", [Value::Int(10)], false);
}
