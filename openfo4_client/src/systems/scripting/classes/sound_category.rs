use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("SoundCategory", "Mute", mute);
    engine.register_native_function("SoundCategory", "Pause", pause);
    engine.register_native_function("SoundCategory", "SetFrequency", set_frequency);
    engine.register_native_function("SoundCategory", "SetVolume", set_volume);
    engine.register_native_function("SoundCategory", "UnMute", un_mute);
    engine.register_native_function("SoundCategory", "UnPause", un_pause);
}

/// `Mute`
///
/// ```text
/// Function Mute() native
/// ```
fn mute(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `Pause`
///
/// ```text
/// Function Pause() native
/// ```
fn pause(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
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
) -> Result<(), Exception> {
    todo!()
}

/// `SetVolume`
///
/// ```text
/// Function SetVolume(float afVolume) native
/// ```
fn set_volume(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_volume: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `UnMute`
///
/// ```text
/// Function UnMute() native
/// ```
fn un_mute(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `UnPause`
///
/// ```text
/// Function UnPause() native
/// ```
fn un_pause(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
