use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("EffectShader", "Play", play);
    engine.register_native_function("EffectShader", "Stop", stop);
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
) -> Result<(), Exception> {
    todo!()
}

/// `Stop`
///
/// ```text
/// Function Stop(ObjectReference akObject) native
/// ```
fn stop(caller: Caller<'_, State>, self_ptr: Object, ak_object: Object) -> Result<(), Exception> {
    todo!()
}
