use openfo4_script::runtime::{Caller, Exception};

use super::{Int, Object, State};

/// `ApplyToRef`
///
/// ```text
/// Function ApplyToRef(ObjectReference akRef) native
/// ```
pub fn apply_to_ref(
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
pub fn clear(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `ForceRefTo`
///
/// ```text
/// Function ForceRefTo(ObjectReference akNewRef) native
/// ```
pub fn force_ref_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    new_ref: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetReference`
///
/// ```text
/// ObjectReference Function GetReference() native
/// ```
pub fn get_reference(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `RemoveFromRef`
///
/// ```text
/// Function RemoveFromRef(ObjectReference akRef) native
/// ```
pub fn remove_from_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    object_ref: Object,
) -> Result<(), Exception> {
    todo!()
}
