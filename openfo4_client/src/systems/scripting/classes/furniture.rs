use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Furniture", "GetAssociatedForm", get_associated_form);
}

/// `GetAssociatedForm`
///
/// ```text
/// Form Function GetAssociatedForm() native
/// ```
fn get_associated_form(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
