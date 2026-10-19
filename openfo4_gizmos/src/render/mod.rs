pub(crate) mod pipeline;

use glam::Vec3;
use openfo4_common::components::Color;

#[derive(Copy, Clone, Debug)]
pub(crate) struct DrawCommand {
    pub(crate) start: Vec3,
    pub(crate) end: Vec3,
    pub(crate) color: Color,
}
