//! `Math` Script
//!
//! TODO: We should directly expand these in assembly.

use crate::classes::State;
use crate::runtime::Caller;

pub fn acos(_caller: Caller<'_, State>, x: f32) -> f32 {
    x.acos()
}

pub fn asin(_caller: Caller<'_, State>, x: f32) -> f32 {
    x.asin()
}

pub fn atan(_caller: Caller<'_, State>, x: f32) -> f32 {
    x.atan()
}

pub fn cos(_caller: Caller<'_, State>, x: f32) -> f32 {
    x.cos()
}

pub fn sin(_caller: Caller<'_, State>, x: f32) -> f32 {
    x.sin()
}

pub fn tan(_caller: Caller<'_, State>, x: f32) -> f32 {
    x.tan()
}
