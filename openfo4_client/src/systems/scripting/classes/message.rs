use std::sync::OnceLock;

use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use super::{Float, Int, Object, State};

static X: OnceLock<()> = OnceLock::new();

pub fn register(engine: &mut Engine) {
    engine.register_native_function("Message", "ClearHelpMessages", clear_help_messages);
    engine.register_native_function("Message", "ResetHelpMessage", reset_help_message);
    engine.register_native_function("Message", "Show", show);
    engine.register_native_function("Message", "ShowAsHelpMessage", show_as_help_message);
    engine.register_native_function("Message", "UnshowAsHelpMessage", unshow_as_help_message);
}

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
pub fn reset_help_message(caller: Caller<'_, State>, event: Object) -> Result<(), Exception> {
    todo!()
}

pub fn show(
    caller: Caller<'_, State>,
    self_ptr: Object,
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
    event: Object,
    duration: Float,
    interval: Float,
    max_times: Int,
    context: Object,
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
