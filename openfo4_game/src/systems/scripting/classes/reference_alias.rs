use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::state::State;

use super::Object;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ReferenceAlias", "ApplyToRef", apply_to_ref);
    engine.register_if_exists("ReferenceAlias", "Clear", clear);
    engine.register_if_exists("ReferenceAlias", "ForceRefTo", force_ref_to);
    engine.register_if_exists("ReferenceAlias", "GetReference", get_reference);
    engine.register_if_exists("ReferenceAlias", "RemoveFromRef", remove_from_ref);
}

/// `ApplyToRef`
///
/// ```text
/// Function ApplyToRef(ObjectReference akRef) native
/// ```
fn apply_to_ref(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _object_ref: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Clear`
///
/// ```text
/// Function Clear() native
/// ```
fn clear(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    let alias = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let alias = alias.data().alias().unwrap();

    let mut aliases = caller.data().globals.quests.aliases.lock();
    let alias = aliases.get_mut(alias).unwrap();

    if !alias.is_optional {
        return Err(Error::msg("cannot clear non-optional alias"));
    }

    let ref_ = alias.data.as_mut_ref().unwrap();
    *ref_ = None;
    Ok(())
}

/// `ForceRefTo`
///
/// ```text
/// Function ForceRefTo(ObjectReference akNewRef) native
/// ```
async fn force_ref_to(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _new_ref: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetReference`
///
/// ```text
/// ObjectReference Function GetReference() native
/// ```
async fn get_reference(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let alias = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let alias = alias.data().alias().unwrap();

    let entity;
    {
        let aliases = caller.data().globals.quests.aliases.lock();

        // The quest did not fill the alias yet.
        let Some(alias) = aliases.get(alias) else {
            return Ok(Object::NULL);
        };

        let Some(e) = alias.data.as_ref().unwrap() else {
            return Ok(Object::NULL);
        };

        entity = e.clone();
    }

    let scripts = caller.data().get_or_create_entity(entity.entity()).await?;
    Ok(scripts.get("ObjectReference").unwrap().get().object.addr())
}

/// `RemoveFromRef`
///
/// ```text
/// Function RemoveFromRef(ObjectReference akRef) native
/// ```
fn remove_from_ref(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _object_ref: Object,
) -> Result<(), Error> {
    todo!()
}
