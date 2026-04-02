use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("SoundCategorySnapshot", "Push", push);
    engine.register_native_function("SoundCategorySnapshot", "Remove", remove);
}

/// `Push`
///
/// ```text
/// Function Push(float afTransitionSecs) native
/// ```
fn push(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_transition_secs: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `Remove`
///
/// ```text
/// Function Remove() native
/// ```
fn remove(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
