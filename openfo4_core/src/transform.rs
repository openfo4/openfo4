use glam::Mat3;

pub const NIF_TO_OPENGL: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => -Z
    [0.0, 0.0, -1.0],
    // Z => Y
    [0.0, 1.0, 0.0],
]);
// Inverse of NIF_TO_OPENGL
pub const OPENGL_TO_NIF: Mat3 = Mat3::from_cols_array_2d(&[
    // X => X
    [1.0, 0.0, 0.0],
    // Y => Z
    [0.0, 0.0, 1.0],
    // Z => -Y
    [0.0, -1.0, 0.0],
]);
