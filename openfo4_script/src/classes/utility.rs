//! Utility Script

use std::time::Duration;

use crate::classes::{Float, State};
use crate::runtime::Caller;

/// `Wait`
///
/// ```text
/// Function Wait(float afSeconds) native global
/// ```
pub fn wait(caller: Caller<'_, State>, seconds: Float) {
    let duration = Duration::from_secs_f32(seconds);
    if duration.is_zero() {
        return;
    }
}
