use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("RefCollectionAlias", "AddRef", add_ref);
    engine.register_if_exists("RefCollectionAlias", "Find", find);
    engine.register_if_exists("RefCollectionAlias", "GetAt", get_at);
    engine.register_if_exists("RefCollectionAlias", "GetCount", get_count);
    engine.register_if_exists("RefCollectionAlias", "RemoveAll", remove_all);
    engine.register_if_exists("RefCollectionAlias", "RemoveRef", remove_ref);
}

/// `AddRef`
///
/// ```text
/// Function AddRef(ObjectReference akNewRef) native
/// ```
fn add_ref(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_new_ref: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Find`
///
/// ```text
/// int Function Find(ObjectReference akFindRef) native
/// ```
fn find(_caller: Caller<'_, State>, _self_ptr: Object, _ak_find_ref: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetAt`
///
/// ```text
/// ObjectReference Function GetAt(int aiIndex) native
/// ```
fn get_at(_caller: Caller<'_, State>, _self_ptr: Object, _ai_index: i32) -> Result<Object, Error> {
    todo!()
}

/// `GetCount`
///
/// ```text
/// int Function GetCount() native
/// ```
fn get_count(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `RemoveAll`
///
/// ```text
/// Function RemoveAll() native
/// ```
fn remove_all(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `RemoveRef`
///
/// ```text
/// Function RemoveRef(ObjectReference akRemoveRef) native
/// ```
fn remove_ref(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_remove_ref: Object,
) -> Result<(), Error> {
    todo!()
}
