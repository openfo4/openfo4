use glam::{Quat, Vec3};
use nalgebra::{Quaternion, Unit};
use rapier3d::math::Vector;

pub(crate) fn vec3(v: Vector<f32>) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub(crate) fn vector(v: Vec3) -> Vector<f32> {
    Vector::new(v.x, v.y, v.z)
}

pub(crate) fn rotation(v: Quat) -> Unit<Quaternion<f32>> {
    Unit::new_unchecked(Quaternion::new(v.w, v.x, v.y, v.z))
}

pub(crate) fn quat(v: Unit<Quaternion<f32>>) -> Quat {
    Quat::from_xyzw(v.i, v.j, v.k, v.w)
}
