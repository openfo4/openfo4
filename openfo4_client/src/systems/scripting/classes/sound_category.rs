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
    engine.register_if_exists("SoundCategory", "UnMute", un_mute);
    engine.register_if_exists("SoundCategory", "UnPause", un_pause);
}

/// `Mute`
///
/// ```text
/// Function Mute() native
/// ```
fn mute(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `Pause`
///
/// ```text
/// Function Pause() native
/// ```
fn pause(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SetFrequency`
///
/// ```text
/// Function SetFrequency(float afFrequencyCoeffecient) native
/// ```
fn set_frequency(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_frequency_coeffecient: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetVolume`
///
/// ```text
/// Function SetVolume(float afVolume) native
/// ```
fn set_volume(caller: Caller<'_, State>, self_ptr: Object, af_volume: f32) -> Result<(), Error> {
    todo!()
}

/// `UnMute`
///
/// ```text
/// Function UnMute() native
/// ```
fn un_mute(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `UnPause`
///
/// ```text
/// Function UnPause() native
/// ```
fn un_pause(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    todo!()
}
