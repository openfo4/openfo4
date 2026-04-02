use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("VisualEffect", "Play", play);
    engine.register_native_function("VisualEffect", "Stop", stop);
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
