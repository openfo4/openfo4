use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Package", "GetOwningQuest", get_owning_quest);
    engine.register_if_exists("Package", "GetTemplate", get_template);
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function GetOwningQuest() native
/// ```
fn get_owning_quest(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetTemplate`
///
/// ```text
/// Package Function GetTemplate() native
/// ```
fn get_template(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
