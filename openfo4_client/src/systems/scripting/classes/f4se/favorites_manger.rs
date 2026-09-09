use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("FavoritesManager", "GetTaggedForms", get_tagged_forms);
    engine.register_if_exists("FavoritesManager", "AddTaggedForms", add_tagged_forms);
    engine.register_if_exists("FavoritesManager", "RemoveTaggedForms", remove_tagged_forms);
    engine.register_if_exists("FavoritesManager", "IsTaggedForm", is_tagged_form);
    engine.register_if_exists("FavoritesManager", "GetFavorites", get_favorites);
    engine.register_if_exists("FavoritesManager", "SetFavorites", set_favorites);
}

/// `GetTaggedForms`
///
/// ```text
/// Form[] Function GetTaggedForms() native global
/// ```
fn get_tagged_forms(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `AddTaggedForms`
///
/// ```text
/// Function AddTaggedForms(Form[] forms) native global
/// ```
fn add_tagged_forms(caller: Caller<'_, State>, forms: Object) -> Result<(), Error> {
    todo!()
}

/// `RemoveTaggedForms`
///
/// ```text
/// Function RemoveTaggedForms(Form[] forms) native global
/// ```
fn remove_tagged_forms(caller: Caller<'_, State>, forms: Object) -> Result<(), Error> {
    todo!()
}

/// `IsTaggedForm`
///
/// ```text
/// bool Function IsTaggedForm(Form akForm) native global
/// ```
fn is_tagged_form(caller: Caller<'_, State>, ak_form: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetFavorites`
///
/// ```text
/// Form[] Function GetFavorites() native global
/// ```
fn get_favorites(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `SetFavorites`
///
/// ```text
/// Function SetFavorites(Form[] favorites) native global
/// ```
fn set_favorites(caller: Caller<'_, State>, favorites: Object) -> Result<(), Error> {
    todo!()
}
