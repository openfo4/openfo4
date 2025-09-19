use std::f32::consts::PI;
use std::time::Duration;

use game_common::components::PointLight;
use hecs::World;

#[derive(Clone, Debug)]
pub struct LightFlicker {
    pub mode: LightFlickerMode,
    pub period: Duration,
    /// Light intensity before amplitude.
    pub intensity: f32,
    pub intensity_amplitude: f32,
    pub state: LightFlickerState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LightFlickerMode {
    /// Regular pulse
    Pulse,
    /// Pseudorandom flickering
    Flicker,
}

#[derive(Clone, Debug, Default)]
pub struct LightFlickerState {
    time: Duration,
}

pub fn update_flickering_lights(world: &mut World, delta: Duration) {
    for (_, (light, flicker)) in world.query_mut::<(&mut PointLight, &mut LightFlicker)>() {
        flicker.state.time = Duration::from_secs_f32(
            (flicker.state.time.as_secs_f32() + delta.as_secs_f32()) % flicker.period.as_secs_f32(),
        );

        let progress = flicker.state.time.as_secs_f32() / flicker.period.as_secs_f32();

        let intensity_base = f32::max(0.0, flicker.intensity * (1.0 - flicker.intensity_amplitude));

        let factor = (progress * PI).sin() * flicker.intensity_amplitude;
        light.intensity = intensity_base + flicker.intensity * factor;
    }
}
