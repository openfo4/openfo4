use crate::classes::{Int, Object, State};
use crate::runtime::Caller;

/// `GetPlayer`
///
/// ```text
/// Actor Function GetPlayer() native global
/// ```
pub fn get_player(caller: Caller<'_, State>) -> Object {
    todo!()
}

/// `GetForm`
///
/// ```text
/// Form Function GetForm(int aiFormID) native global
/// ```
pub fn get_form(caller: Caller<'_, State>, form_id: Int) -> Object {
    todo!()
}
