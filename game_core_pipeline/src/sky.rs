use game_common::color::LinearRgb;
use game_common::components::Color;
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct SkyMaterial {
    pub sky_lower: LinearRgb,
    pub sky_upper: LinearRgb,
    pub horizon: LinearRgb,
    pub sun: Color,
    pub sun_scale: f32,
    pub sun_position: Vec3,
    pub sky_scale: f32,
}
