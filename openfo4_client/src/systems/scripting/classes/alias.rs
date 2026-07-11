use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Alias", "GetOwningQuest", get_owning_quest);
    engine.register_if_exists("Alias", "StartObjectProfiling", start_object_profiling);
    engine.register_if_exists("Alias", "StopObjectProfiling", stop_object_profiling);
}

/// `GetOwningQuest`
///
/// ```text
/// Quest Function() native
/// ```
fn get_owning_quest(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    let alias = caller
        .data()
        .aliases
        .get(self_ptr)
        .ok_or(Exception::Error)?;
    let quest = caller
        .data()
        .objects
        .get_by_record(alias.quest)
        .ok_or(Exception::Error)?;
    let script = quest.get("Quest").ok_or(Exception::Error)?;
    Ok(script.get().object.addr())
}

/// `StartObjectProfiling`
///
/// ```text
/// Function() native
/// ```
fn start_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Exception> {
    Ok(())
}

/// `StopObjectProfiling`
///
/// ```text
/// Function() native
/// ```
fn stop_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Exception> {
    Ok(())
}
