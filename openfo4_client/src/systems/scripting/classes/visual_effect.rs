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
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_object: Object,
    af_time: f32,
    ak_facing_object: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Stop`
///
/// ```text
/// Function Stop(ObjectReference akObject) native
/// ```
fn stop(caller: Caller<'_, State>, self_ptr: Object, ak_object: Object) -> Result<(), Error> {
    todo!()
}
