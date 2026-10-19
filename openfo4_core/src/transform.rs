use glam::{Mat3, Mat4, Quat, Vec3};
use openfo4_common::components::Transform;

const NIF_TO_METERS: f32 = 1.0 / 69.99125;
const NIF_TO_OPENGL: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => -Z
    [0.0, 0.0, -1.0],
    // Z => Y
    [0.0, 1.0, 0.0],
]);
// Inverse of NIF_TO_OPENGL
const OPENGL_TO_NIF: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => Z
    [0.0, 0.0, 1.0],
    // Z => -Y
    [0.0, -1.0, 0.0],
]);

/// Extension trait for OpenGL <-> NIF coordinate system conversions.
pub trait ConvertCoordinateSystem {
    /// Converts this value from OpenGL to NIF coordinate system.
    fn to_nif(self) -> Self;

    /// Converts this value from NIF to OpenGL coordinate system.
    fn to_opengl(self) -> Self;
}

impl ConvertCoordinateSystem for Vec3 {
    fn to_nif(self) -> Self {
        OPENGL_TO_NIF * self
    }

    fn to_opengl(self) -> Self {
        NIF_TO_OPENGL * self
    }
}

impl ConvertCoordinateSystem for Quat {
    fn to_nif(self) -> Self {
        let (_, rotation, _) = Mat4::from_mat3(OPENGL_TO_NIF).to_scale_rotation_translation();
        rotation * self
    }

    fn to_opengl(self) -> Self {
        let (_, rotation, _) = Mat4::from_mat3(NIF_TO_OPENGL).to_scale_rotation_translation();
        rotation * self
    }
}

impl ConvertCoordinateSystem for Transform {
    fn to_nif(self) -> Self {
        let (scale, rotation, translation) =
            Mat4::from_mat3(OPENGL_TO_NIF).to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale: scale / NIF_TO_METERS,
        } * self
    }

    fn to_opengl(self) -> Self {
        let (scale, rotation, translation) =
            Mat4::from_mat3(NIF_TO_OPENGL).to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale: scale * NIF_TO_METERS,
        } * self
    }
}

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec3};
    use openfo4_common::components::Transform;

    use super::ConvertCoordinateSystem;

    #[track_caller]
    fn assert_quat_eq(lhs: Quat, rhs: Quat) {
        let diff = 1e-7;

        if !lhs.abs_diff_eq(rhs, diff) && !lhs.abs_diff_eq(-rhs, diff) {
            assert_eq!(lhs, rhs);
        }
    }

    #[track_caller]
    fn assert_transform_eq(lhs: Transform, rhs: Transform) {
        let diff = 1e-5;

        let translation = lhs.translation.abs_diff_eq(rhs.translation, diff);
        let rotation = lhs.rotation.abs_diff_eq(rhs.rotation, diff)
            || lhs.rotation.abs_diff_eq(-rhs.rotation, diff);
        let scale = lhs.scale.abs_diff_eq(rhs.scale, diff);

        if !translation || !rotation || !scale {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn convert_rotation_roundtrip() {
        assert_quat_eq(Quat::IDENTITY.to_nif().to_opengl(), Quat::IDENTITY);
        assert_quat_eq(Quat::IDENTITY.to_opengl().to_nif(), Quat::IDENTITY);

        let q = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0).normalize(), 0.2);
        assert_quat_eq(q.to_nif().to_opengl(), q);
        assert_quat_eq(q.to_opengl().to_nif(), q);
    }

    #[test]
    fn convert_transform_roundtrip() {
        assert_transform_eq(
            Transform::IDENTITY.to_nif().to_opengl(),
            Transform::IDENTITY,
        );
        assert_transform_eq(
            Transform::IDENTITY.to_opengl().to_nif(),
            Transform::IDENTITY,
        );

        let t1 = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_axis_angle(Vec3::new(4.0, 5.0, 6.0).normalize(), 1.5),
            scale: Vec3::new(1.7, 3.2, 5.2),
        };

        assert_transform_eq(t1.to_nif().to_opengl(), t1);
        assert_transform_eq(t1.to_opengl().to_nif(), t1);
    }
}
