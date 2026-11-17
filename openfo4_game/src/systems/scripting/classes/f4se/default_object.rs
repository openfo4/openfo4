use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
fn get_default_object(_caller: Caller<'_, State>, _editor_id: Object) -> Result<Object, Error> {
    todo!()
}

/// `Get`
///
/// ```text
/// Form Function Get() native
/// ```
fn get(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `Set`
///
/// ```text
/// Function Set(Form newForm) native
/// ```
fn set(_caller: Caller<'_, State>, _self_ptr: Object, _new_form: Object) -> Result<(), Error> {
    todo!()
}
