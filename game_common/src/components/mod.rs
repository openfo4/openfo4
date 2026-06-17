//! All components

mod lights;
mod transform;

use glam::Vec3;
pub use lights::{AmbientLight, Color, DirectionalLight, PointLight, SpotLight};
pub use transform::{GlobalTransform, Transform};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// Returns a unit vector representing the axis.
    #[inline]
    pub const fn to_vec3(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }
}
