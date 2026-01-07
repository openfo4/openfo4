use crate::classes::{Bool, Int, Object, State};
use crate::runtime::{Caller, Exception};

/// `AddForm`
///
/// ```text
/// Function AddForm(Form apForm) native
/// ```
pub fn add_form(
    caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `Find`
///
/// ```text
/// int Function Find(Form apForm) native
/// ```
pub fn find(caller: Caller<'_, State>, self_ptr: Object, form: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetAt`
///
/// ```text
/// Form Function GetAt(int aiIndex) native
/// ```
pub fn get_at(
    caller: Caller<'_, State>,
    self_ptr: Object,
    index: Int,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetSize`
///
/// ```text
/// int Function GetSize() native
/// ```
pub fn get_size(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `HasForm`
///
/// ```text
/// bool Function HasForm(Form akForm) native
/// ```
pub fn has_form(
    caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `RemoveAddedForm`
///
/// ```text
/// Function RemoveAddedForm(Form apForm) native
/// ```
pub fn remove_added_form(
    caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `Revert`
///
/// ```text
/// Function Revert() native
/// ```
pub fn revert(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
