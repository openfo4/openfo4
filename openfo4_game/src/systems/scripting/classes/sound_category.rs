use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("SoundCategory", "Mute", mute);
    engine.register_if_exists("SoundCategory", "Pause", pause);
    engine.register_if_exists("SoundCategory", "SetFrequency", set_frequency);
    engine.register_if_exists("SoundCategory", "SetVolume", set_volume);
    engine.register_if_exists("SoundCategory", "UnMute", unmute);
    engine.register_if_exists("SoundCategory", "UnPause", unpause);
}

/// `Mute`
///
/// ```text
/// Function Mute() native
/// ```
fn mute(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Pause`
///
/// ```text
/// Function Pause() native
/// ```
fn pause(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SetFrequency`
///
/// ```text
/// Function SetFrequency(float afFrequencyCoeffecient) native
/// ```
fn set_frequency(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_frequency_coeffecient: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetVolume`
///
/// ```text
/// Function SetVolume(float afVolume) native
/// ```
fn set_volume(_caller: Caller<'_, State>, _self_ptr: Object, _af_volume: f32) -> Result<(), Error> {
    todo!()
}

/// `UnMute`
///
/// ```text
/// Function UnMute() native
/// ```
fn unmute(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `UnPause`
///
/// ```text
/// Function UnPause() native
/// ```
fn unpause(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}
