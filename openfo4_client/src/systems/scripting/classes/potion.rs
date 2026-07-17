use openfo4_script::runtime::{Caller, Error};

use super::{Bool, Object, State};

/// `IsHostile`
///
/// ```text
/// bool Function IsHostile() native
/// ```
pub fn is_hostile(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Error> {
    todo!()
}
