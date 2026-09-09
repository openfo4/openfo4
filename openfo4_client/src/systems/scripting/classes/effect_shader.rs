use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("EffectShader", "Play", play);
    engine.register_if_exists("EffectShader", "Stop", stop);
}

/// `Play`
///
/// ```text
/// Function Play(ObjectReference akObject, float afDuration) native
/// ```
fn play(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_object: Object,
    af_duration: f32,
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
