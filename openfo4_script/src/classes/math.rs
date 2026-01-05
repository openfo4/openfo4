//! `Math` Script
//!
//! TODO: We should directly expand these in assembly.
//!
//! Note that some functions are stubbed out. The calls are replaced with compiler intrinsics,
//! but we still need to declare the functions to satisfy the linker.

use crate::classes::State;
use crate::runtime::{Caller, Exception};

pub fn abs(_: Caller<'_, State>, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn acos(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.acos())
}

pub fn asin(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.asin())
}

pub fn atan(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.atan())
}

pub fn ceiling(_: Caller<'_, State>, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn cos(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.cos())
}

pub fn degrees_to_radians(_: Caller<'_, State>, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn floor(_: Caller<'_, State>, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn max(_: Caller<'_, State>, _: f32, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn min(_: Caller<'_, State>, _: f32, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn pow(_: Caller<'_, State>, _: f32, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn radians_to_degrees(_: Caller<'_, State>, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn sin(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.sin())
}
pub fn sqrt(_: Caller<'_, State>, _: f32) -> Result<f32, Exception> {
    unreachable!();
}

pub fn tan(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.tan())
}
