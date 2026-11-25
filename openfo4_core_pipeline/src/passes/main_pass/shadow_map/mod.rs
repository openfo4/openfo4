mod opaque;
mod transparent;

use glam::{Mat4, UVec2, Vec3, Vec4};
use openfo4_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, Sampler, Texture, TextureRegion, TextureView,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{BufferUsage, LoadOp, TextureFormat};

use crate::camera::Camera;
use crate::passes::main_pass::shadow_map::opaque::OpaqueShadowMapPass;
use crate::passes::main_pass::shadow_map::transparent::TransparentShadowMapPass;
use crate::passes::mesh::MeshState;

const SHADOW_MAP_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub struct ShadowMapPass {
    opaque: OpaqueShadowMapPass,
    transparent: TransparentShadowMapPass,
}

impl ShadowMapPass {
    pub fn new() -> Self {
        Self {
            opaque: OpaqueShadowMapPass::new(),
            transparent: TransparentShadowMapPass::new(),
        }
    }

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: ShadowMapParams<'_>) {
        if params.mesh.instances.num_opaque == 0 && params.mesh.instances.num_transparent == 0 {
            queue.clear_texture(
                TextureRegion {
                    texture: params.depth,
                    mip_level: 0,
                },
                [0; 4],
            );

            return;
        }

        let mut load_op = LoadOp::Clear(0.0);

        let camera = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.light),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        if params.mesh.instances.num_opaque != 0 {
            self.opaque.run(
                queue,
                ShadowMapParamsInner {
                    mesh: params.mesh,
                    camera: &camera,
                    load_op,
                    depth: params.depth,
                    materials: params.materials,
                    textures: params.textures,
                    sampler: params.sampler,
                },
            );

            load_op = LoadOp::Load;
        }

        if params.mesh.instances.num_transparent != 0 {
            self.transparent.run(
                queue,
                ShadowMapParamsInner {
                    mesh: params.mesh,
                    camera: &camera,
                    load_op,
                    depth: params.depth,
                    materials: params.materials,
                    textures: params.textures,
                    sampler: params.sampler,
                },
            );
        }
    }
}

pub struct ShadowMapParams<'a> {
    pub light: Mat4,
    pub mesh: &'a mut MeshState,
    pub depth: &'a Texture,
    pub materials: &'a Buffer,
    pub textures: &'a [&'a TextureView],
    pub sampler: &'a Sampler,
}

struct ShadowMapParamsInner<'a> {
    mesh: &'a mut MeshState,
    camera: &'a Buffer,
    load_op: LoadOp<f32>,
    depth: &'a Texture,
    materials: &'a Buffer,
    textures: &'a [&'a TextureView],
    sampler: &'a Sampler,
}

const CSM_SHADOW_TEXTURES_SIZES: [u32; 4] = [4096, 4096, 4096, 4096];
const CSM_SHADOW_TEXTURES_FAR: [f32; 4] = [10.0, 50.0, 300.0, 500.0];

/// Properties of a shadow cascade.
#[derive(Copy, Clone, Debug)]
pub struct ShadowCascade {
    /// The view-projection matrix of the light.
    pub light_vp: Mat4,
    /// The size of the shadow texture.
    pub size: UVec2,
}

pub fn compute_directional_csm_shadow_params(
    camera: &Camera,
    direction: Vec3,
) -> [ShadowCascade; 4] {
    let mut cascades = [ShadowCascade {
        light_vp: Mat4::default(),
        size: UVec2::ZERO,
    }; 4];

    let mut near = camera.projection.near;

    let camera_view_matrix = camera.view_matrix();
    let camera_proj_matrix = camera.projection_matrix();

    for ((texture_size, far), cascade) in CSM_SHADOW_TEXTURES_SIZES
        .iter()
        .zip(CSM_SHADOW_TEXTURES_FAR)
        .zip(&mut cascades)
    {
        let light_vp = compute_dir_light_space_matrix(
            camera_view_matrix,
            camera_proj_matrix,
            camera.projection.far,
            direction,
            *texture_size,
            near,
            far,
        );

        cascade.light_vp = light_vp;
        cascade.size = UVec2::splat(*texture_size);

        near = far;
    }

    cascades
}

fn compute_dir_light_space_matrix(
    camera_view_matrix: Mat4,
    camera_proj_matrix: Mat4,
    camera_proj_far: f32,
    direction: Vec3,
    texture_size: u32,
    near: f32,
    far: f32,
) -> Mat4 {
    // The only properties that change between cascades are
    // the near and far distances.
    // We don't have to recompute the entire projection matrix,
    // which is expensive due to trig.
    // Taken from glam's `Mat4::persective_rh` implementation.
    let mut proj_matrix = camera_proj_matrix;
    let r = far / (near - far);
    proj_matrix.z_axis.z = r;
    proj_matrix.w_axis.z = r * near;

    let corners = frustum_corners(camera_view_matrix, proj_matrix);

    let center = corners.iter().sum::<Vec3>() / corners.len() as f32;
    let radius = corners
        .iter()
        .max_by(|a, b| center.distance(**a).total_cmp(&center.distance(**b)))
        .unwrap();
    let radius = center.distance(*radius);

    let texels_per_unit = (texture_size as f32) / (radius * 2.0);
    let scale = Mat4::from_scale(Vec3::splat(texels_per_unit));

    let look_at = Mat4::look_at_rh(Vec3::ZERO, -direction, direction.any_orthonormal_vector());
    let look_at = scale * look_at;
    let look_at_inv = look_at.inverse();

    let mut aligned_center = look_at.transform_point3(center);
    aligned_center.x = aligned_center.x.floor();
    aligned_center.y = aligned_center.y.floor();
    aligned_center = look_at_inv.transform_point3(aligned_center);

    let view = Mat4::look_at_rh(
        aligned_center + (direction * radius * 2.0),
        aligned_center,
        direction.any_orthonormal_vector(),
    );

    let proj = Mat4::orthographic_rh(
        -radius,
        radius,
        -radius,
        radius,
        -radius - camera_proj_far,
        radius + camera_proj_far,
    );
    proj * view
}

fn frustum_corners(view: Mat4, proj: Mat4) -> [Vec3; 8] {
    let inv = (proj * view).inverse();

    [
        (-1.0, -1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, 1.0, 1.0),
        (1.0, -1.0, -1.0),
        (1.0, -1.0, 1.0),
        (1.0, 1.0, -1.0),
        (1.0, 1.0, 1.0),
    ]
    .map(|(x, y, z)| {
        let v = inv * Vec4::new(x, y, z, 1.0);
        (v / v.w).truncate()
    })
}
