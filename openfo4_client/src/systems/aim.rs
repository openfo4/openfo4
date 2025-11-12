use std::time::{Duration, Instant};

use glam::Vec2;

#[derive(Clone, Debug)]
pub struct RecoilState {}

#[derive(Clone, Debug)]
pub struct AimProps {
    pub min_angle: f32,
    pub max_angle: f32,
    pub cur_angle: f32,
    pub increase_angle_per_shot: f32,
    pub decrease_angle_per_sec: f32,
    pub decrease_angle_wait_delay: Duration,
    pub last_shot: Instant,
}

impl AimProps {
    /// Computes the firing angle when firing the weapon.
    ///
    /// The angle is the same in both X and Y directions.
    pub fn compute_fire_angle(&mut self) -> f32 {
        0.0
    }
}
