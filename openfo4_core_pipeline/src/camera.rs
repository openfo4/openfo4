use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, UVec2, Vec2, Vec3};
use openfo4_common::components::Transform;
use openfo4_common::math::Ray;
use openfo4_render::RenderTarget;

use crate::entities::SceneHandle;

/// Matrix to reverse Z for improved depth testing.
// https://developer.nvidia.com/content/depth-precision-visualized
// https://tomhultonharrop.com/mathematics/graphics/2023/08/06/reverse-z.html
const REVERSE_Z: Mat4 = Mat4::from_cols_array_2d(&[
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0, 1.0],
]);

#[derive(Clone, Debug)]
pub struct Camera {
    pub transform: Transform,
    pub projection: Projection,
    pub target: RenderTarget,
    /// The scene that should be rendered with this camera.
    pub scene: SceneHandle,
}

impl Camera {
    pub fn viewport_to_world(
        &self,
        camera_transform: Transform,
        viewport_size: Vec2,
        mut viewport_position: Vec2,
    ) -> Ray {
        viewport_position.y = viewport_size.y - viewport_position.y;
        let ndc = viewport_position * 2.0 / viewport_size - Vec2::ONE;

        let proj_matrix = self.projection.projection_matrix();

        let ndc_to_world = camera_transform.compute_matrix() * proj_matrix.inverse();
        let world_near_plane = ndc_to_world.project_point3(ndc.extend(1.0));
        // EPS instead of 0, otherwise we get NaNs.
        let world_far_plane = ndc_to_world.project_point3(ndc.extend(f32::EPSILON));

        Ray {
            origin: world_near_plane,
            direction: (world_far_plane - world_near_plane).normalize(),
        }
    }

    pub fn update_aspect_ratio(&mut self, size: UVec2) {
        self.projection.aspect_ratio = size.x as f32 / size.y as f32;
    }

    /// Computes the view matrix for the camera.
    pub(crate) fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(
            self.transform.translation,
            self.transform.rotation * -Vec3::Z,
            self.transform.rotation * Vec3::Y,
        )
    }

    /// Computes the projection matrix for the camera.
    pub(crate) fn projection_matrix(&self) -> Mat4 {
        self.projection.projection_matrix()
    }

    pub(crate) fn view_projection_matrix(&self) -> Mat4 {
        REVERSE_Z * self.projection_matrix() * self.view_matrix()
    }

    pub(crate) fn frustum(&self) -> Frustum {
        let proj = self.projection_matrix();
        let proj_t = proj.transpose();

        let frustum_x = (proj_t.w_axis + proj_t.x_axis).normalize();
        let frustum_y = (proj_t.w_axis + proj_t.y_axis).normalize();

        Frustum {
            cos_v: frustum_x.x,
            sin_v: frustum_x.z,
            cos_h: frustum_y.y,
            sin_h: frustum_y.z,
            znear: self.projection.near,
            zfar: self.projection.far,
        }
    }
}

/// Perspective camera projection paramters
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Projection {
    pub aspect_ratio: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Projection {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            aspect_ratio: 1.0,
            fov: PI / 4.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct CameraUniform {
    // We only need `[f32; 3]`, but one word for alignment.
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(transform: Transform, projection: Projection) -> Self {
        let view = Mat4::look_to_rh(
            transform.translation,
            transform.rotation * -Vec3::Z,
            transform.rotation * Vec3::Y,
        );

        let proj = projection.projection_matrix();

        Self {
            view_position: [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                0.0,
            ],
            view_proj: (REVERSE_Z * proj * view).to_cols_array_2d(),
        }
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new(Transform::default(), Projection::default())
    }
}

/// The frustum of a [`Camera`] in view space.
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct Frustum {
    cos_v: f32,
    sin_v: f32,
    cos_h: f32,
    sin_h: f32,
    znear: f32,
    zfar: f32,
}
