use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ActorBase", "GetTemplate", get_template);
    engine.register_if_exists("ActorBase", "HasHeadPartOverlays", has_head_part_overlays);
    engine.register_if_exists("ActorBase", "GetHeadParts", get_head_parts);
    engine.register_if_exists("ActorBase", "GetOutfit", get_outfit);
    engine.register_if_exists("ActorBase", "SetBodyWeight", set_body_weight);
    engine.register_if_exists("ActorBase", "GetBodyWeight", get_body_weight);
}

/// `GetTemplate`
///
/// ```text
/// ActorBase Function GetTemplate(bool bTopMost) native
/// ```
fn get_template(
    caller: Caller<'_, State>,
    self_ptr: Object,
    b_top_most: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `HasHeadPartOverlays`
///
/// ```text
/// bool Function HasHeadPartOverlays() native
/// ```
fn has_head_part_overlays(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetHeadParts`
///
/// ```text
/// HeadPart[] Function GetHeadParts(bool bOverlays) native
/// ```
fn get_head_parts(
    caller: Caller<'_, State>,
    self_ptr: Object,
    b_overlays: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `GetOutfit`
///
/// ```text
/// Outfit Function GetOutfit(bool bSleepOutfit) native
/// ```
fn get_outfit(
    caller: Caller<'_, State>,
    self_ptr: Object,
    b_sleep_outfit: bool,
) -> Result<Object, Exception> {
    todo!()
}

/// `SetBodyWeight`
///
/// ```text
/// Function SetBodyWeight(BodyWeight weight) native
/// ```
fn set_body_weight(
    caller: Caller<'_, State>,
    self_ptr: Object,
    weight: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetBodyWeight`
///
/// ```text
/// BodyWeight Function GetBodyWeight() native
/// ```
fn get_body_weight(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}
