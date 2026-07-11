use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{EngineExt, Object, State};

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
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_new_ref: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `Find`
///
/// ```text
/// int Function Find(ObjectReference akFindRef) native
/// ```
fn find(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_find_ref: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetAt`
///
/// ```text
/// ObjectReference Function GetAt(int aiIndex) native
/// ```
fn get_at(caller: Caller<'_, State>, self_ptr: Object, ai_index: i32) -> Result<Object, Exception> {
    todo!()
}

/// `GetCount`
///
/// ```text
/// int Function GetCount() native
/// ```
fn get_count(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `RemoveAll`
///
/// ```text
/// Function RemoveAll() native
/// ```
fn remove_all(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `RemoveRef`
///
/// ```text
/// Function RemoveRef(ObjectReference akRemoveRef) native
/// ```
fn remove_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_remove_ref: Object,
) -> Result<(), Exception> {
    todo!()
}
