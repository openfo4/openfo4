use std::time::Duration;

use hecs::World;

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
}

pub fn init_time(world: &mut World) {
    world.spawn((GameTime(Duration::ZERO),));
}

pub fn update_time(world: &mut World, delta: Duration) {
    for (_, time) in world.query_mut::<&mut GameTime>() {
        time.0 += delta.mul_f32(TIMESCALE);
    }
}
