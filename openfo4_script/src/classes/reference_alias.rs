use crate::classes::{Object, State};
use crate::runtime::{Caller, Exception};

/// `GetReference`
///
/// ```text
/// ObjectReference Function GetReference() native
/// ```
pub fn get_reference_alias(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<Object, Exception> {
    todo!()
}
