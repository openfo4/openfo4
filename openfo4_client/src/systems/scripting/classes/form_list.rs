use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::global::FormList;
use crate::systems::scripting::classes::EngineExt;

use super::{Bool, Int, Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("FormList", "AddForm", add_form);
    engine.register_if_exists("FormList", "Find", find);
    engine.register_if_exists("FormList", "GetAt", get_at);
    engine.register_if_exists("FormList", "GetSize", get_size);
    engine.register_if_exists("FormList", "HasForm", has_form);
    engine.register_if_exists("FormList", "RemoveAddedForm", remove_added_form);
    engine.register_if_exists("FormList", "Revert", revert);
}

/// `AddForm`
///
/// ```text
/// Function AddForm(Form apForm) native
/// ```
async fn add_form(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<(), Error> {
    let form = caller
        .data()
        .objects
        .get_by_addr(form)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    with_form_list(&mut caller, self_ptr, |list| {
        list.add(form);
    })
    .await
}

/// `Find`
///
/// ```text
/// int Function Find(Form apForm) native
/// ```
async fn find(mut caller: Caller<'_, State>, self_ptr: Object, form: Object) -> Result<Int, Error> {
    let form = caller
        .data()
        .objects
        .get_by_addr(form)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    with_form_list(&mut caller, self_ptr, |list| {
        list.index(form)
            .map(|index| i32::try_from(index).ok())
            .flatten()
            .unwrap_or(-1)
    })
    .await
}

/// `GetAt`
///
/// ```text
/// Form Function GetAt(int aiIndex) native
/// ```
async fn get_at(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    index: Int,
) -> Result<Object, Error> {
    let id = with_form_list(&mut caller, self_ptr, |list| {
        let Ok(index) = usize::try_from(index) else {
            return Err(Object::NULL);
        };

        let Some(id) = list.get(index) else {
            return Err(Object::NULL);
        };

        Ok(id)
    })
    .await?;
    let id = match id {
        Ok(id) => id,
        Err(obj) => return Ok(obj),
    };

    Ok(caller
        .data()
        .objects
        .get_by_record(id)
        .and_then(|v| v.get("Form"))
        .map(|v| v.get().object.addr())
        .unwrap_or(Object::NULL))
}

/// `GetSize`
///
/// ```text
/// int Function GetSize() native
/// ```
async fn get_size(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Error> {
    with_form_list(&mut caller, self_ptr, |list| {
        list.len().try_into().unwrap_or(i32::MAX)
    })
    .await
}

/// `HasForm`
///
/// ```text
/// bool Function HasForm(Form akForm) native
/// ```
async fn has_form(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<Bool, Error> {
    let form = caller
        .data()
        .objects
        .get_by_addr(form)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    with_form_list(&mut caller, self_ptr, |list| list.contains(form)).await
}

/// `RemoveAddedForm`
///
/// ```text
/// Function RemoveAddedForm(Form apForm) native
/// ```
async fn remove_added_form(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    form: Object,
) -> Result<(), Error> {
    let form = caller
        .data()
        .objects
        .get_by_addr(form)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    with_form_list(&mut caller, self_ptr, |list| {
        list.remove(form);
    })
    .await
}

/// `Revert`
///
/// ```text
/// Function Revert() native
/// ```
async fn revert(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    with_form_list(&mut caller, self_ptr, |list| {
        list.reset();
    })
    .await
}

async fn with_form_list<T, F>(
    caller: &mut Caller<'_, State>,
    addr: Object,
    f: F,
) -> Result<T, Error>
where
    F: FnOnce(&mut FormList) -> T,
{
    let form_list = caller
        .data()
        .objects
        .get_by_addr(addr)
        .and_then(|s| s.record())
        .ok_or_else(|| todo!())?;
    let mut globals = caller.data_mut().globals_mut().await;
    let list = globals
        .form_lists
        .get_mut(&form_list)
        .ok_or_else(|| todo!())?;
    Ok(f(list))
}
