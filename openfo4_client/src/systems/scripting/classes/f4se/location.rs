use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{EngineExt, Object, State};

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
fn get_parent(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `SetParent`
///
/// ```text
/// Function SetParent(Location akLocation) native
/// ```
fn set_parent(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_location: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetEncounterZone`
///
/// ```text
/// EncounterZone Function GetEncounterZone(bool recursive) native
/// ```
fn get_encounter_zone(
    caller: Caller<'_, State>,
    self_ptr: Object,
    recursive: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `SetEncounterZone`
///
/// ```text
/// Function SetEncounterZone(EncounterZone ez) native
/// ```
fn set_encounter_zone(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ez: Object,
) -> Result<(), Exception> {
    todo!()
}
