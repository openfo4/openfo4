use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("TopicInfo", "GetOwningQuest", get_owning_quest);
    engine.register_native_function("TopicInfo", "HasBeenSaid", has_been_said);
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function GetOwningQuest() native
/// ```
fn get_owning_quest(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `HasBeenSaid`
///
/// ```text
/// bool Function HasBeenSaid() native
/// ```
fn has_been_said(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}
