use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
async fn get_owning_quest(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let alias = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let alias = alias.data().alias().unwrap();

    let script = caller
        .data()
        .get_or_create_record_class(alias.quest, "Quest")
        .await?;

    Ok(script.get().object.addr())
}

/// `StartObjectProfiling`
///
/// ```text
/// Function() native
/// ```
fn start_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) {}

/// `StopObjectProfiling`
///
/// ```text
/// Function() native
/// ```
fn stop_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) {}
