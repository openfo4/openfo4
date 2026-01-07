use crate::classes::{Object, State};
use crate::runtime::{Caller, Exception};

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
