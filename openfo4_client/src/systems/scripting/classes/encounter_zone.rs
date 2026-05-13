use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("EncounterZone", "CountActors", count_actors);
    engine.register_native_function("EncounterZone", "GetActors", get_actors);
    engine.register_native_function("EncounterZone", "Reset", reset);
}

/// `CountActors`
///
/// ```text
/// int Function CountActors(Keyword apRequiredLinkedRefKeyword, Keyword apExcludeLinkedRefKeyword) native
/// ```
fn count_actors(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_required_linked_ref_keyword: Object,
    ap_exclude_linked_ref_keyword: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetActors`
///
/// ```text
/// Actor[] Function GetActors(Keyword apRequiredLinkedRefKeyword, Keyword apExcludeLinkedRefKeyword) native
/// ```
fn get_actors(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ap_required_linked_ref_keyword: Object,
    ap_exclude_linked_ref_keyword: Object,
) -> Result<Object, Exception> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
