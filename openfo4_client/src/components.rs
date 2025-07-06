use std::ops::{Mul, MulAssign};

use game_common::components::Transform;
use glam::{Mat3, Mat4, Quat, Vec3};
use openfo4_hkx::mesh::TriMesh;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub fov: f32,
}

const NIF_TO_METERS: f32 = 1.0 / 69.99125;
const NIF_TO_OPENGL: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => -Z
    [0.0, 0.0, -1.0],
    // Z => Y
    [0.0, 1.0, 0.0],
]);
// Invese of NIF_TO_OPENGL
const OPENGL_TO_NIF: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => Z
    [0.0, 0.0, 1.0],
    // Z => -Y
    [0.0, -1.0, 0.0],
]);

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
        let (scale, rotation, translation) =
            Mat4::from_mat3(OPENGL_TO_NIF).to_scale_rotation_translation();

        Self(
            Transform {
                translation,
                rotation,
                scale: scale / NIF_TO_METERS,
            } * value,
        )
    }
}

impl From<NiTransform> for Transform {
    fn from(value: NiTransform) -> Self {
        let (scale, rotation, translation) =
            Mat4::from_mat3(NIF_TO_OPENGL).to_scale_rotation_translation();

        Self {
            translation,
            rotation,
            scale: scale * NIF_TO_METERS,
        } * value.0
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
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Root;
