use std::time::Duration;

use openfo4_core::ecs::World;

// FIXME: Unhardcode timescale into GLOB.
const TIMESCALE: f32 = 20.0;

/// The ingame time.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameTime(Duration);

impl GameTime {
    pub fn second(&self) -> u64 {
        self.0.as_secs() % 60
    }

    pub fn minute(&self) -> u64 {
        (self.0.as_secs() / 60) % 60
    }

    /// Returns a value in range `0..24`.
    pub fn hour(&self) -> u64 {
        (self.0.as_secs() / 60 / 60) % 24
    }

    /// Sets the hour.
    ///
    /// # Panics
    ///
    /// Panics if `hour` is greater than `23`.
    pub fn set_hour(&mut self, hour: u8) {
        assert!(hour < 24);
        self.0 -= Duration::from_hours(self.hour());
        self.0 += Duration::from_hours(hour.into());
    }
}

pub fn init_time(world: &mut World) {
    world.spawn((GameTime(Duration::ZERO),));
}

pub fn update_time(world: &mut World, delta: Duration) {
    for mut time in world.query_mut::<&mut GameTime>() {
        time.0 += delta.mul_f32(TIMESCALE);
    }
}
