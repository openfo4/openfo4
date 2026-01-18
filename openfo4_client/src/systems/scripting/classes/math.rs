//! `Math` Script
//!
//! TODO: We should directly expand these in assembly.
//!
//! Note that some functions are stubbed out. The calls are replaced with compiler intrinsics,
//! but we still need to declare the functions to satisfy the linker.

use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

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
fn abs(_caller: Caller<'_, State>, _value: f32) -> Result<f32, Exception> {
    unreachable!()
}

/// `acos`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn acos(_caller: Caller<'_, State>, value: f32) -> Result<f32, Exception> {
    Ok(value.acos())
}

/// `asin`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn asin(_caller: Caller<'_, State>, value: f32) -> Result<f32, Exception> {
    Ok(value.asin())
}

/// `atan`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn atan(_caller: Caller<'_, State>, value: f32) -> Result<f32, Exception> {
    Ok(value.atan())
}

/// `Ceiling`
///
/// ```text
/// int Function(float afValue) native global
/// ```
fn ceiling(_caller: Caller<'_, State>, _value: f32) -> Result<i32, Exception> {
    unreachable!()
}

/// `cos`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn cos(_caller: Caller<'_, State>, value: f32) -> Result<f32, Exception> {
    Ok(value.cos())
}

/// `DegreesToRadians`
///
/// ```text
/// float Function(float afDegrees) native global
/// ```
fn degrees_to_radians(_caller: Caller<'_, State>, _value: f32) -> Result<f32, Exception> {
    unreachable!()
}

/// `Floor`
///
/// ```text
/// int Function(float afValue) native global
/// ```
fn floor(_caller: Caller<'_, State>, _value: f32) -> Result<i32, Exception> {
    unreachable!()
}

/// `pow`
///
/// ```text
/// float Function(float x, float y) native global
/// ```
fn pow(_caller: Caller<'_, State>, x: f32, y: f32) -> Result<f32, Exception> {
    Ok(x.powf(y))
}

/// `RadiansToDegrees`
///
/// ```text
/// float Function(float afRadians) native global
/// ```
fn radians_to_degrees(_caller: Caller<'_, State>, _value: f32) -> Result<f32, Exception> {
    unreachable!()
}

/// `sin`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn sin(_caller: Caller<'_, State>, value: f32) -> Result<f32, Exception> {
    Ok(value.sin())
}

/// `sqrt`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn sqrt(_caller: Caller<'_, State>, _value: f32) -> Result<f32, Exception> {
    unreachable!()
}

/// `tan`
///
/// ```text
/// float Function(float afValue) native global
/// ```
fn tan(_caller: Caller<'_, State>, value: f32) -> Result<f32, Exception> {
    Ok(value.tan())
}
