use crate::classes::{Object, State};
use crate::runtime::{Caller, Exception};

/// `Dispel`
///
/// ```text
/// Function Dispel() native
/// ```
pub fn dispel(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
