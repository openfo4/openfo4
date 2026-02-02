use openfo4_script::runtime::{Caller, Exception};

use crate::systems::global::FormList;

use super::{Bool, Int, Object, State};

/// `AddForm`
///
/// ```text
/// Function AddForm(Form apForm) native
/// ```
pub fn add_form(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<(), Exception> {
    let form = caller.data().objects.form(form).ok_or(Exception::Error)?;
    let form_list = get_form_list(&mut caller, self_ptr)?;
    form_list.add(form);
    Ok(())
}

/// `Find`
///
/// ```text
/// int Function Find(Form apForm) native
/// ```
pub fn find(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<Int, Exception> {
    let form = caller.data().objects.form(form).ok_or(Exception::Error)?;
    let form_list = get_form_list(&mut caller, self_ptr)?;
    let index = form_list
        .index(form)
        .map(|index| i32::try_from(index).ok())
        .flatten()
        .unwrap_or(-1);
    Ok(index)
}

/// `GetAt`
///
/// ```text
/// Form Function GetAt(int aiIndex) native
/// ```
pub fn get_at(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    index: Int,
) -> Result<Object, Exception> {
    let form_list = get_form_list(&mut caller, self_ptr)?;
    let Ok(index) = usize::try_from(index) else {
        return Ok(Object::NULL);
    };

    let Some(id) = form_list.get(index) else {
        return Ok(Object::NULL);
    };

    Ok(caller.data().objects.addr(id).unwrap_or(Object::NULL))
}

/// `GetSize`
///
/// ```text
/// int Function GetSize() native
/// ```
pub fn get_size(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    let form_list = get_form_list(&mut caller, self_ptr)?;
    Ok(form_list.len().try_into().unwrap_or(i32::MAX))
}

/// `HasForm`
///
/// ```text
/// bool Function HasForm(Form akForm) native
/// ```
pub fn has_form(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<Bool, Exception> {
    let form = caller.data().objects.form(form).ok_or(Exception::Error)?;
    let form_list = get_form_list(&mut caller, self_ptr)?;
    Ok(form_list.contains(form))
}

/// `RemoveAddedForm`
///
/// ```text
/// Function RemoveAddedForm(Form apForm) native
/// ```
pub fn remove_added_form(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<(), Exception> {
    let form = caller.data().objects.form(form).ok_or(Exception::Error)?;
    let form_list = get_form_list(&mut caller, self_ptr)?;
    form_list.remove(form);
    Ok(())
}

/// `Revert`
///
/// ```text
/// Function Revert() native
/// ```
pub fn revert(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    let form_list = get_form_list(&mut caller, self_ptr)?;
    form_list.reset();
    Ok(())
}

fn get_form_list<'a>(
    caller: &'a mut Caller<'_, State>,
    addr: Object,
) -> Result<&'a mut FormList, Exception> {
    let form_list = caller
        .data()
        .objects
        .form_list(addr)
        .ok_or(Exception::Error)?;
    caller
        .data_mut()
        .globals_mut()
        .form_lists
        .get_mut(&form_list)
        .ok_or(Exception::Error)
}
