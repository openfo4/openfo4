use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("MiscObject", "GetMiscComponents", get_misc_components);
    engine.register_if_exists("MiscObject", "SetMiscComponents", set_misc_components);
}

/// `GetMiscComponents`
///
/// ```text
/// MiscComponent[] Function GetMiscComponents() native
/// ```
fn get_misc_components(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetMiscComponents`
///
/// ```text
/// Function SetMiscComponents(MiscComponent[] components) native
/// ```
fn set_misc_components(
    caller: Caller<'_, State>,
    self_ptr: Object,
    components: Object,
) -> Result<(), Error> {
    todo!()
}
