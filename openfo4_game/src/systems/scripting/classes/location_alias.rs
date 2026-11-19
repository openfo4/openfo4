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
fn clear(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `GetLocation`
///
/// ```text
/// Location Function GetLocation() native
/// ```
fn get_location(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `ForceLocationTo`
///
/// ```text
/// Function ForceLocationTo(Location akNewLocation) native
/// ```
fn force_location_to(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _new_location: Object,
) -> Result<(), Error> {
    todo!()
}
