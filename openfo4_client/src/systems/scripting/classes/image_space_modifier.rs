use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ImageSpaceModifier", "Apply", apply);
    engine.register_if_exists("ImageSpaceModifier", "ApplyCrossFade", apply_cross_fade);
    engine.register_if_exists("ImageSpaceModifier", "PopTo", pop_to);
    engine.register_if_exists("ImageSpaceModifier", "Remove", remove);
    engine.register_if_exists("ImageSpaceModifier", "RemoveCrossFade", remove_cross_fade);
}

/// `Apply`
///
/// ```text
/// Function Apply(float afStrength) native
/// ```
fn apply(caller: Caller<'_, State>, self_ptr: Object, af_strength: f32) -> Result<(), Error> {
    todo!()
}

/// `ApplyCrossFade`
///
/// ```text
/// Function ApplyCrossFade(float afFadeDuration) native
/// ```
fn apply_cross_fade(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_fade_duration: f32,
) -> Result<(), Error> {
    todo!()
}

/// `PopTo`
///
/// ```text
/// Function PopTo(ImageSpaceModifier akNewModifier, float afStrength) native
/// ```
fn pop_to(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_new_modifier: Object,
    af_strength: f32,
) -> Result<(), Error> {
    todo!()
}

/// `Remove`
///
/// ```text
/// Function Remove() native
/// ```
fn remove(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `RemoveCrossFade`
///
/// ```text
/// Function RemoveCrossFade(float afFadeDuration) native global
/// ```
fn remove_cross_fade(caller: Caller<'_, State>, af_fade_duration: f32) -> Result<(), Error> {
    todo!()
}
