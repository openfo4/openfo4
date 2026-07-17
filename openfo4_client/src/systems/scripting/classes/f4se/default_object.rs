use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("DefaultObject", "GetDefaultObject", get_default_object);
    engine.register_if_exists("DefaultObject", "Get", get);
    engine.register_if_exists("DefaultObject", "Set", set);
}

/// `GetDefaultObject`
///
/// ```text
/// Form Function GetDefaultObject(string editorId) native global
/// ```
fn get_default_object(caller: Caller<'_, State>, editor_id: Object) -> Result<Object, Error> {
    todo!()
}

/// `Get`
///
/// ```text
/// Form Function Get() native
/// ```
fn get(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `Set`
///
/// ```text
/// Function Set(Form newForm) native
/// ```
fn set(caller: Caller<'_, State>, self_ptr: Object, new_form: Object) -> Result<(), Error> {
    todo!()
}
