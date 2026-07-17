use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::classes::EngineExt;

use super::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ReferenceAlias", "ApplyToRef", apply_to_ref);
    engine.register_if_exists("ReferenceAlias", "Clear", clear);
    engine.register_if_exists("ReferenceAlias", "ForceRefTo", force_ref_to);
    engine.register_if_exists("ReferenceAlias", "GetReference", get_reference);
    engine.register_if_exists("ReferenceAlias", "RemoveFromRef", remove_from_ref);
}

/// `ApplyToRef`
///
/// ```text
/// Function ApplyToRef(ObjectReference akRef) native
/// ```
fn apply_to_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    object_ref: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Clear`
///
/// ```text
/// Function Clear() native
/// ```
fn clear(mut caller: Caller<'_, State>, self_ptr: Object) {
    caller.data_mut().aliases.set(self_ptr, None);
}

/// `ForceRefTo`
///
/// ```text
/// Function ForceRefTo(ObjectReference akNewRef) native
/// ```
fn force_ref_to(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    new_ref: Object,
) -> Result<(), Error> {
    let new_ref = (!new_ref.is_null()).then_some(new_ref);
    caller.data_mut().aliases.set(self_ptr, new_ref);
    Ok(())
}

/// `GetReference`
///
/// ```text
/// ObjectReference Function GetReference() native
/// ```
fn get_reference(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let alias = caller.data().aliases.get(self_ptr).ok_or_else(|| todo!())?;

    let Some(object) = alias.object else {
        return Ok(Object::NULL);
    };

    let script = caller
        .data()
        .objects
        .get_by_addr(object)
        .ok_or_else(|| todo!())?;

    Ok(script.get().object.addr())
}

/// `RemoveFromRef`
///
/// ```text
/// Function RemoveFromRef(ObjectReference akRef) native
/// ```
fn remove_from_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    object_ref: Object,
) -> Result<(), Error> {
    todo!()
}
