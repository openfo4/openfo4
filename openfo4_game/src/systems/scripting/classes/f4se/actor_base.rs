use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _b_top_most: bool,
) -> Result<Object, Error> {
    todo!()
}

/// `HasHeadPartOverlays`
///
/// ```text
/// bool Function HasHeadPartOverlays() native
/// ```
fn has_head_part_overlays(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetHeadParts`
///
/// ```text
/// HeadPart[] Function GetHeadParts(bool bOverlays) native
/// ```
fn get_head_parts(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _b_overlays: bool,
) -> Result<Object, Error> {
    todo!()
}

/// `GetOutfit`
///
/// ```text
/// Outfit Function GetOutfit(bool bSleepOutfit) native
/// ```
fn get_outfit(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _b_sleep_outfit: bool,
) -> Result<Object, Error> {
    todo!()
}

/// `SetBodyWeight`
///
/// ```text
/// Function SetBodyWeight(BodyWeight weight) native
/// ```
fn set_body_weight(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _weight: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetBodyWeight`
///
/// ```text
/// BodyWeight Function GetBodyWeight() native
/// ```
fn get_body_weight(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
