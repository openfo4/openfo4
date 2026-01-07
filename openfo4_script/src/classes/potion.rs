use crate::classes::{Bool, Object, State};
use crate::runtime::{Caller, Exception};

/// `IsHostile`
///
/// ```text
/// bool Function IsHostile() native
/// ```
pub fn is_hostile(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}
