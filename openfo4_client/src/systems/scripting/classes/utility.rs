//! Utility Script

use std::time::Duration;

use openfo4_script::runtime::{Caller, Exception};

use super::{Float, State};

/// `GetCurrentGameTime`
///
/// ```text
/// float Function GetCurrentGameTime() native
/// ```
pub fn get_current_game_time(caller: Caller<'_, State>) -> Result<Float, Exception> {
    todo!()
}

/// `RandomFloat`
///
/// ```text
/// float Function RandomFloat(float afMin = 0.0, float afMax = 1.0) native global
/// ```
pub fn random_float(caller: Caller<'_, State>, min: f32, max: f32) -> Result<f32, Exception> {
    todo!()
}

/// `RandomInt`
///
/// ```text
/// int Function RandomInt(int aiMin = 0, int aiMax = 100) native global
/// ```
pub fn random_int(caller: Caller<'_, State>, min: i32, max: i32) -> Result<i32, Exception> {
    todo!()
}

/// `Wait`
///
/// ```text
/// Function Wait(float afSeconds) native global
/// ```
pub fn wait(caller: Caller<'_, State>, seconds: Float) -> Result<(), Exception> {
    let duration = Duration::from_secs_f32(seconds);
    if duration.is_zero() {
        return Ok(());
    }

    Err(Exception::Suspend)
}

/// `WaitMenuMode`
///
/// ```text
/// Function WaitMenuMode(float afSeconds) Native Global
/// ```
pub fn wait_menu_mode(caller: Caller<'_, State>, seconds: Float) -> Result<(), Exception> {
    todo!()
}
