//! `Math` Script

use openfo4_script::engine::Engine;
use openfo4_script::runtime::Caller;

use crate::systems::scripting::classes::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Math", "abs", abs);
    engine.register_native_function("Math", "acos", acos);
    engine.register_native_function("Math", "asin", asin);
    engine.register_native_function("Math", "atan", atan);
    engine.register_native_function("Math", "Ceiling", ceiling);
    engine.register_native_function("Math", "cos", cos);
    engine.register_native_function("Math", "DegreesToRadians", degrees_to_radians);
    engine.register_native_function("Math", "Floor", floor);
    engine.register_native_function("Math", "pow", pow);
    engine.register_native_function("Math", "RadiansToDegrees", radians_to_degrees);
    engine.register_native_function("Math", "sin", sin);
    engine.register_native_function("Math", "sqrt", sqrt);
    engine.register_native_function("Math", "tan", tan);
}

/// `abs`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn abs(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.abs()
}

/// `acos`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn acos(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.acos()
}

/// `asin`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn asin(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.asin()
}

/// `atan`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn atan(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.atan()
}

/// `Ceiling`
///
/// ```text
/// int Function(float afValue) native global
/// ```
fn ceiling(_caller: Caller<'_, State>, value: f32) -> i32 {
    value.ceil() as i32
}

/// `cos`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn cos(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.cos()
}

/// `DegreesToRadians`
///
/// ```text
/// float Function(float afDegrees) native global
/// ```
fn degrees_to_radians(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.to_radians()
}

/// `Floor`
///
/// ```text
/// int Function(float afValue) native global
/// ```
fn floor(_caller: Caller<'_, State>, value: f32) -> i32 {
    value.floor() as i32
}

/// `pow`
///
/// ```text
/// float Function(float x, float y) native global
/// ```
fn pow(_caller: Caller<'_, State>, x: f32, y: f32) -> f32 {
    x.powf(y)
}

/// `RadiansToDegrees`
///
/// ```text
/// float Function(float afRadians) native global
/// ```
fn radians_to_degrees(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.to_degrees()
}

/// `sin`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn sin(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.sin()
}

/// `sqrt`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn sqrt(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.sqrt()
}

/// `tan`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn tan(_caller: Caller<'_, State>, value: f32) -> f32 {
    value.tan()
}
