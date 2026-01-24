use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use super::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("ReferenceAlias", "ApplyToRef", apply_to_ref);
    engine.register_native_function("ReferenceAlias", "Clear", clear);
    engine.register_native_function("ReferenceAlias", "ForceRefTo", force_ref_to);
    engine.register_native_function("ReferenceAlias", "GetReference", get_reference);
    engine.register_native_function("ReferenceAlias", "RemoveFromRef", remove_from_ref);
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
) -> Result<(), Exception> {
    todo!()
}

/// `Clear`
///
/// ```text
/// Function Clear() native
/// ```
fn clear(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `ForceRefTo`
///
/// ```text
/// Function ForceRefTo(ObjectReference akNewRef) native
/// ```
fn force_ref_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    new_ref: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetReference`
///
/// ```text
/// ObjectReference Function GetReference() native
/// ```
fn get_reference(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
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
) -> Result<(), Exception> {
    todo!()
}
