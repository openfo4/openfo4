use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("VisualEffect", "Play", play);
    engine.register_if_exists("VisualEffect", "Stop", stop);
}

/// `Play`
///
/// ```text
/// Function Play(ObjectReference akObject, float afTime, ObjectReference akFacingObject) native
/// ```
fn play(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_object: Object,
    _af_time: f32,
    _ak_facing_object: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Stop`
///
/// ```text
/// Function Stop(ObjectReference akObject) native
/// ```
fn stop(_caller: Caller<'_, State>, _self_ptr: Object, _ak_object: Object) -> Result<(), Error> {
    todo!()
}
