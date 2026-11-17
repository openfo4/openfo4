use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Location", "GetParent", get_parent);
    engine.register_if_exists("Location", "SetParent", set_parent);
    engine.register_if_exists("Location", "GetEncounterZone", get_encounter_zone);
    engine.register_if_exists("Location", "SetEncounterZone", set_encounter_zone);
}

/// `GetParent`
///
/// ```text
/// Location Function GetParent() native
/// ```
fn get_parent(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `SetParent`
///
/// ```text
/// Function SetParent(Location akLocation) native
/// ```
fn set_parent(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_location: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetEncounterZone`
///
/// ```text
/// EncounterZone Function GetEncounterZone(bool recursive) native
/// ```
fn get_encounter_zone(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _recursive: bool,
) -> Result<Object, Error> {
    todo!()
}

/// `SetEncounterZone`
///
/// ```text
/// Function SetEncounterZone(EncounterZone ez) native
/// ```
fn set_encounter_zone(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ez: Object,
) -> Result<(), Error> {
    todo!()
}
