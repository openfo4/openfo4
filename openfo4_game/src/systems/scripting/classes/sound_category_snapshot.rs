use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("SoundCategorySnapshot", "Push", push);
    engine.register_if_exists("SoundCategorySnapshot", "Remove", remove);
}

/// `Push`
///
/// ```text
/// Function Push(float afTransitionSecs) native
/// ```
fn push(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_transition_secs: f32,
) -> Result<(), Error> {
    todo!()
}

/// `Remove`
///
/// ```text
/// Function Remove() native
/// ```
fn remove(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}
