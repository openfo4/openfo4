use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("FavoritesManager", "GetTaggedForms", get_tagged_forms);
    engine.register_native_function("FavoritesManager", "AddTaggedForms", add_tagged_forms);
    engine.register_native_function("FavoritesManager", "RemoveTaggedForms", remove_tagged_forms);
    engine.register_native_function("FavoritesManager", "IsTaggedForm", is_tagged_form);
    engine.register_native_function("FavoritesManager", "GetFavorites", get_favorites);
    engine.register_native_function("FavoritesManager", "SetFavorites", set_favorites);
}

/// `GetTaggedForms`
///
/// ```text
/// Form[] Function GetTaggedForms() native global
/// ```
fn get_tagged_forms(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `AddTaggedForms`
///
/// ```text
/// Function AddTaggedForms(Form[] forms) native global
/// ```
fn add_tagged_forms(caller: Caller<'_, State>, forms: Object) -> Result<(), Exception> {
    todo!()
}

/// `RemoveTaggedForms`
///
/// ```text
/// Function RemoveTaggedForms(Form[] forms) native global
/// ```
fn remove_tagged_forms(caller: Caller<'_, State>, forms: Object) -> Result<(), Exception> {
    todo!()
}

/// `IsTaggedForm`
///
/// ```text
/// bool Function IsTaggedForm(Form akForm) native global
/// ```
fn is_tagged_form(caller: Caller<'_, State>, ak_form: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetFavorites`
///
/// ```text
/// Form[] Function GetFavorites() native global
/// ```
fn get_favorites(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `SetFavorites`
///
/// ```text
/// Function SetFavorites(Form[] favorites) native global
/// ```
fn set_favorites(caller: Caller<'_, State>, favorites: Object) -> Result<(), Exception> {
    todo!()
}
