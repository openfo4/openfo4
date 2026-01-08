use openfo4_script::runtime::{Caller, Exception};

use super::{Int, Object, State};

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
