use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("LocationAlias", "Clear", clear);
    engine.register_if_exists("LocationAlias", "GetLocation", get_location);
    engine.register_if_exists("LocationAlias", "ForceLocationTo", force_location_to);
}

/// `Clear`
///
/// ```text
/// Function Clear() native
/// ```
fn clear(mut caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    caller.data_mut().aliases.set(self_ptr, None);
    Ok(())
}

/// `GetLocation`
///
/// ```text
/// Location Function GetLocation() native
/// ```
fn get_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let alias = caller.data().aliases.get(self_ptr).ok_or_else(|| todo!())?;

    let Some(object) = alias.object else {
        return Ok(Object::NULL);
    };

    let script = caller
        .data()
        .objects
        .get_by_addr(object)
        .ok_or_else(|| todo!())?;

    Ok(script.get().object.addr())
}

/// `ForceLocationTo`
///
/// ```text
/// Function ForceLocationTo(Location akNewLocation) native
/// ```
fn force_location_to(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    new_location: Object,
) -> Result<(), Error> {
    let new_location = (!new_location.is_null()).then_some(new_location);
    caller.data_mut().aliases.set(self_ptr, new_location);
    Ok(())
}
