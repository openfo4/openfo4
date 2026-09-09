use openfo4_script::engine::Engine;
use openfo4_script::runtime::Caller;

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Math", "exp", exp);
    engine.register_if_exists("Math", "log", log);
    engine.register_if_exists("Math", "LeftShift", left_shift);
    engine.register_if_exists("Math", "RightShift", right_shift);
    engine.register_if_exists("Math", "LogicalNot", logical_not);
    engine.register_if_exists("Math", "LogicalAnd", logical_and);
    engine.register_if_exists("Math", "LogicalOr", logical_or);
    engine.register_if_exists("Math", "LogicalXor", logical_xor);
}

/// `Exp`
///
/// ```text
/// float Function Exp(float afArg1) native global
/// ```
fn exp(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.exp()
}

/// `LeftShift`
///
/// ```text
/// int Function LeftShift(int value, int shiftBy) native global
/// ```
fn left_shift(_caller: Caller<'_, State>, lhs: i32, rhs: i32) -> i32 {
    lhs.unbounded_shl(rhs.cast_unsigned())
}

fn log(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.ln()
}

/// `LogicalAnd`
///
/// ```text
/// int Function LogicalAnd(int arg1, int arg2) native global
/// ```
///
fn logical_and(_caller: Caller<'_, State>, lhs: i32, rhs: i32) -> i32 {
    lhs & rhs
}

/// `LogicalNot`
///
/// ```text
/// int Function LogicalNot(int arg1) native global
/// ```
fn logical_not(_caller: Caller<'_, State>, value: i32) -> i32 {
    !value
}

/// `LogicalOr`
///
/// ```text
/// int Function LogicalOr(int arg1, int arg2) native global
/// ```
fn logical_or(_caller: Caller<'_, State>, lhs: i32, rhs: i32) -> i32 {
    lhs | rhs
}

/// `LogicalXor`
///
/// ```text
/// int Function LogicalXor(int arg1, int arg2) native global
/// ```
fn logical_xor(_caller: Caller<'_, State>, lhs: i32, rhs: i32) -> i32 {
    lhs ^ rhs
}

/// `RightShift`
///
/// ```text
/// int Function RightShift(int value, int shiftBy) native global
/// ```
fn right_shift(_caller: Caller<'_, State>, lhs: i32, rhs: i32) -> i32 {
    lhs.unbounded_shr(rhs.cast_unsigned())
}
