use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::state::State;

use super::{Bool, Object};

/// `IsHostile`
///
/// ```text
/// bool Function IsHostile() native
/// ```
pub fn is_hostile(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Error> {
    todo!()
}
