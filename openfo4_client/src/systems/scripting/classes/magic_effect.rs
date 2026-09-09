use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::state::State;

use super::Object;

/// `GetAssociatedSkill`
///
/// ```text
/// string Function GetAssociatedSkill() native
/// ```
pub fn get_assocaited_skill(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
