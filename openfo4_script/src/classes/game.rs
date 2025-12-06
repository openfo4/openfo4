use crate::classes::{Int, Object, State};
use crate::runtime::{Caller, Exception, ObjectAddr};

/// `GetPlayer`
///
/// ```text
/// Actor Function GetPlayer() native global
/// ```
pub fn get_player(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().player.addr())
}

/// `GetForm`
///
/// ```text
/// Form Function GetForm(int aiFormID) native global
/// ```
pub fn get_form(caller: Caller<'_, State>, form_id: Int) -> Result<Object, Exception> {
    todo!()
}
