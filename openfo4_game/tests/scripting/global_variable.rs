use openfo4_esp::records::GLOB;
use openfo4_game::systems::scripting::invocations::Value;

use crate::runner::{DefaultExt, TestRunner};

#[test]
fn global_variable_get_value() {
    let mut builder = TestRunner::builder();

    let global = builder.add_record(GLOB::default_with(|global| {
        global.fltv = 2.0;
    }));

    let mut runner = builder.build();

    let global = runner.load(global);
    runner.call_assert(global, "GetValue", [], 2.0);
}

#[test]
fn global_variable_set_value() {
    let mut builder = TestRunner::builder();

    let global = builder.add_record(GLOB::default());

    let mut runner = builder.build();

    let global = runner.load(global);
    runner.call(&global, "SetValue", [Value::Float(3.0)]);
    runner.call_assert(global, "GetValue", [], 3.0);
}
