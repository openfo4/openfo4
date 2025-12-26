use game_common::components::Color;

#[derive(Clone, Debug)]
pub struct SkyMaterial {
    pub sky_lower: Color,
    pub sky_upper: Color,
    pub horizon: Color,
}
