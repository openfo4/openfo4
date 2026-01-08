use openfo4_script::runtime::{Caller, Exception};

use super::{Object, State};

/// `Dispel`
///
/// ```text
/// Function Dispel() native
/// ```
pub fn dispel(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
