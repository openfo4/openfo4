use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("LocationAlias", "Clear", clear);
    engine.register_native_function("LocationAlias", "GetLocation", get_location);
    engine.register_native_function("LocationAlias", "ForceLocationTo", force_location_to);
}

/// `Clear`
///
/// ```text
/// Function Clear() native
/// ```
fn clear(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `GetLocation`
///
/// ```text
/// Location Function GetLocation() native
/// ```
fn get_location(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `ForceLocationTo`
///
/// ```text
/// Function ForceLocationTo(Location akNewLocation) native
/// ```
fn force_location_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_new_location: Object,
) -> Result<(), Exception> {
    todo!()
}
