use std::ops::{Mul, MulAssign};

use game_common::components::Transform;
use glam::{Quat, Vec3};
use hecs::Entity;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_esp::FormId;

use crate::systems::camera::CameraMode;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub mode: CameraMode,
    pub fov: f32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct NiTransform(Transform);

impl NiTransform {
    pub fn from_ni_transform(transform: Transform) -> Self {
        Self(transform)
    }

    pub fn to_ni_transform(self) -> Transform {
        self.0
    }

    pub fn from_translation(translation: Vec3) -> Self {
        Self(Transform::from_translation(translation))
    }

    pub fn translation(&self) -> Vec3 {
        self.0.translation
    }

    pub fn rotation(&self) -> Quat {
        self.0.rotation
    }

    pub fn scale(&self) -> Vec3 {
        self.0.scale
    }
}

impl From<Transform> for NiTransform {
    fn from(value: Transform) -> Self {
        Self(value.to_nif())
    }
}

impl From<NiTransform> for Transform {
    fn from(value: NiTransform) -> Self {
        value.0.to_opengl()
    }
}

impl Mul for NiTransform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl MulAssign for NiTransform {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

/// A root entity.
///
/// A root entity represents a gameplay entity (such as Actor, Object, etc.) and has no
/// [`Children`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Root;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RootParent(pub Entity);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Player;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Persistent;

/// Indicates the worldspace that the entity is contained in.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorldSpace {
    pub form: FormId,
    pub is_interior: bool,
}

/// Marker for all actor entities (including player).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ActorMarker;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LookingAtTarget(pub Entity);
