//! `Math` Script
//!
//! TODO: We should directly expand these in assembly.

use crate::classes::State;
use crate::runtime::{Caller, Exception};

pub fn acos(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.acos())
}

pub fn asin(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.asin())
}

pub fn atan(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.atan())
}

pub fn cos(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.cos())
}

pub fn sin(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.sin())
}

pub fn tan(_caller: Caller<'_, State>, x: f32) -> Result<f32, Exception> {
    Ok(x.tan())
}
