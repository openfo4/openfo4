use crate::classes::{Int, Object, State};
use crate::runtime::{Caller, Exception};

/// `GetAt`
///
/// ```text
/// ObjectReference Function GetAt(int aiIndex) native
/// ```
pub fn get_at(
    caller: Caller<'_, State>,
    self_ptr: Object,
    index: Int,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetCount`
///
/// ```text
/// int Function GetCount() native
/// ```
pub fn get_count(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}
