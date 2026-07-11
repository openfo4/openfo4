use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Weather", "EnableAmbientParticles", enable_ambient_particles);
    engine.register_if_exists("Weather", "FindWeather", find_weather);
    engine.register_if_exists("Weather", "ForceActive", force_active);
    engine.register_if_exists("Weather", "GetClassification", get_classification);
    engine.register_if_exists("Weather", "GetCurrentWeather", get_current_weather);
    engine.register_if_exists("Weather", "GetCurrentWeatherTransition", get_current_weather_transition);
    engine.register_if_exists("Weather", "GetOutgoingWeather", get_outgoing_weather);
    engine.register_if_exists("Weather", "GetSkyMode", get_sky_mode);
    engine.register_if_exists("Weather", "ReleaseOverride", release_override);
    engine.register_if_exists("Weather", "SetActive", set_active);
}

/// `EnableAmbientParticles`
///
/// ```text
/// Function EnableAmbientParticles(bool abEnable) native global
/// ```
fn enable_ambient_particles(caller: Caller<'_, State>, ab_enable: bool) -> Result<(), Exception> {
    todo!()
}

/// `FindWeather`
///
/// ```text
/// Weather Function FindWeather(int auiType) native global
/// ```
fn find_weather(caller: Caller<'_, State>, aui_type: i32) -> Result<Object, Exception> {
    todo!()
}

/// `ForceActive`
///
/// ```text
/// Function ForceActive(bool abOverride=false) native
/// ```
fn force_active(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_override: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `GetClassification`
///
/// ```text
/// int Function GetClassification() native
/// ```
fn get_classification(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetCurrentWeather`
///
/// ```text
/// Weather Function GetCurrentWeather() native global
/// ```
fn get_current_weather(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `GetCurrentWeatherTransition`
///
/// ```text
/// float Function GetCurrentWeatherTransition() native global
/// ```
fn get_current_weather_transition(caller: Caller<'_, State>) -> Result<f32, Exception> {
    todo!()
}

/// `GetOutgoingWeather`
///
/// ```text
/// Weather Function GetOutgoingWeather() native global
/// ```
fn get_outgoing_weather(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `GetSkyMode`
///
/// ```text
/// int Function GetSkyMode() native global
/// ```
fn get_sky_mode(caller: Caller<'_, State>) -> Result<i32, Exception> {
    todo!()
}

/// `ReleaseOverride`
///
/// ```text
/// Function ReleaseOverride() native global
/// ```
fn release_override(caller: Caller<'_, State>) -> Result<(), Exception> {
    todo!()
}

/// `SetActive`
///
/// ```text
/// Function SetActive(bool abOverride=false, bool abAccelerate=false) native
/// ```
fn set_active(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_override: bool,
    ab_accelerate: bool,
) -> Result<(), Exception> {
    todo!()
}
