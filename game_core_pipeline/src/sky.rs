use game_common::components::Color;
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct SkyMaterial {
    pub sky_lower: Color,
    pub sky_upper: Color,
    pub horizon: Color,
    pub sun: Color,
    pub sun_scale: f32,
    pub sun_position: Vec3,
}
