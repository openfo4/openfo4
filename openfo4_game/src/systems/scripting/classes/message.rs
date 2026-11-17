use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::state::State;

use super::{Float, Int, Object};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Message", "ClearHelpMessages", clear_help_messages);
    engine.register_if_exists("Message", "ResetHelpMessage", reset_help_message);
    engine.register_if_exists("Message", "Show", show);
    engine.register_if_exists("Message", "ShowAsHelpMessage", show_as_help_message);
    engine.register_if_exists("Message", "UnshowAsHelpMessage", unshow_as_help_message);
}

/// `ClearHelpMessages`
///
/// ```text
/// Function ClearHelpMessages() native global
/// ```
pub fn clear_help_messages(_caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `ResetHelpMessage`
///
/// ```text
/// Function ResetHelpMessage(string asEvent) native global
/// ```
pub fn reset_help_message(_caller: Caller<'_, State>, _event: Object) -> Result<(), Error> {
    todo!()
}

pub fn show(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _arg1: Float,
    _arg2: Float,
    _arg3: Float,
    _arg4: Float,
    _arg5: Float,
    _arg6: Float,
    _arg7: Float,
    _arg8: Float,
    _arg9: Float,
) -> Result<Int, Error> {
    todo!()
}

/// `ShowAsHelpMessage`
///
/// ```text
/// Function ShowAsHelpMessage(string asEvent, float afDuration, float afInterval, int aiMaxTimes, string asContext, int aiPriority) native
/// ```
pub fn show_as_help_message(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _event: Object,
    _duration: Float,
    _interval: Float,
    _max_times: Int,
    _context: Object,
    _priority: Int,
) -> Result<(), Error> {
    todo!()
}

/// `UnshowAsHelpMessage`
///
/// ```text
/// Function UnshowAsHelpMessage() native
/// ```
pub fn unshow_as_help_message(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}
