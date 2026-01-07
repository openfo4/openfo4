use std::cell::OnceCell;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;

use crate::classes::{Float, Int, Object, State};
use crate::runtime::{Caller, Exception};

static X: OnceLock<()> = OnceLock::new();

/// `ClearHelpMessages`
///
/// ```text
/// Function ClearHelpMessages() native global
/// ```
pub fn clear_help_messages(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `ResetHelpMessage`
///
/// ```text
/// Function ResetHelpMessage(string asEvent) native global
/// ```
pub fn reset_help_message(caller: Caller<'_, State>, event: isize) -> Result<(), Exception> {
    todo!()
}

pub fn show(
    caller: Caller<'_, State>,
    self_ptr: isize,
    arg1: Float,
    arg2: Float,
    arg3: Float,
    arg4: Float,
    arg5: Float,
    arg6: Float,
    arg7: Float,
    arg8: Float,
    arg9: Float,
) -> Result<Int, Exception> {
    if X.get().is_some() {
        Ok(0)
    } else {
        X.set(()).unwrap();

        Err(Exception::Suspend)
    }
}

/// `ShowAsHelpMessage`
///
/// ```text
/// Function ShowAsHelpMessage(string asEvent, float afDuration, float afInterval, int aiMaxTimes, string asContext, int aiPriority) native
/// ```
pub fn show_as_help_message(
    caller: Caller<'_, State>,
    self_ptr: Object,
    event: isize,
    duration: Float,
    interval: Float,
    max_times: Int,
    context: isize,
    priority: Int,
) -> Result<(), Exception> {
    todo!()
}

/// `UnshowAsHelpMessage`
///
/// ```text
/// Function UnshowAsHelpMessage() native
/// ```
pub fn unshow_as_help_message(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<(), Exception> {
    todo!()
}
