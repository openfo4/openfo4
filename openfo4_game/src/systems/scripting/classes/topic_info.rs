use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("TopicInfo", "GetOwningQuest", get_owning_quest);
    engine.register_if_exists("TopicInfo", "HasBeenSaid", has_been_said);
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function GetOwningQuest() native
/// ```
fn get_owning_quest(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `HasBeenSaid`
///
/// ```text
/// bool Function HasBeenSaid() native
/// ```
fn has_been_said(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}
