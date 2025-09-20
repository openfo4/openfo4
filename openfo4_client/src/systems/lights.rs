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

        // Factor between -1 and 1.
        let factor = match flicker.mode {
            LightFlickerMode::Pulse => (progress * 2.0 * PI).sin(),
            LightFlickerMode::Flicker => {
                // Based on https://stackoverflow.com/questions/8798771/perlin-noise-for-1d
                ((2.0 * progress).sin() + (PI * progress).sin()) / 2.0
            }
        };

        light.intensity =
            flicker.intensity + flicker.intensity * flicker.intensity_amplitude * factor;
    }
}
