use openfo4_script::runtime::{Caller, Exception};

use super::{Object, State};

/// `GetAssociatedSkill`
///
/// ```text
/// string Function GetAssociatedSkill() native
/// ```
pub fn get_assocaited_skill(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<isize, Exception> {
    todo!()
}
