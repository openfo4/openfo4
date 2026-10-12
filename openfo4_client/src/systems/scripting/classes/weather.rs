use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
fn enable_ambient_particles(_caller: Caller<'_, State>, _ab_enable: bool) -> Result<(), Error> {
    todo!()
}

/// `FindWeather`
///
/// ```text
/// Weather Function FindWeather(int auiType) native global
/// ```
fn find_weather(_caller: Caller<'_, State>, _aui_type: i32) -> Result<Object, Error> {
    todo!()
}

/// `ForceActive`
///
/// ```text
/// Function ForceActive(bool abOverride=false) native
/// ```
fn force_active(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_override: bool,
) -> Result<(), Error> {
    todo!()
}

/// `GetClassification`
///
/// ```text
/// int Function GetClassification() native
/// ```
fn get_classification(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetCurrentWeather`
///
/// ```text
/// Weather Function GetCurrentWeather() native global
/// ```
fn get_current_weather(_caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetCurrentWeatherTransition`
///
/// ```text
/// float Function GetCurrentWeatherTransition() native global
/// ```
fn get_current_weather_transition(_caller: Caller<'_, State>) -> Result<f32, Error> {
    todo!()
}

/// `GetOutgoingWeather`
///
/// ```text
/// Weather Function GetOutgoingWeather() native global
/// ```
fn get_outgoing_weather(_caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetSkyMode`
///
/// ```text
/// int Function GetSkyMode() native global
/// ```
fn get_sky_mode(_caller: Caller<'_, State>) -> Result<i32, Error> {
    todo!()
}

/// `ReleaseOverride`
///
/// ```text
/// Function ReleaseOverride() native global
/// ```
fn release_override(_caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `SetActive`
///
/// ```text
/// Function SetActive(bool abOverride=false, bool abAccelerate=false) native
/// ```
fn set_active(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_override: bool,
    _ab_accelerate: bool,
) -> Result<(), Error> {
    todo!()
}
