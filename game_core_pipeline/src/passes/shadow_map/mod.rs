mod opaque;
mod transparent;

use game_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, Sampler, Texture, TextureRegion, TextureView,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{BufferUsage, LoadOp, TextureFormat};
use glam::Mat4;

use crate::passes::mesh::MeshState;
use crate::passes::shadow_map::opaque::OpaqueShadowMapPass;
use crate::passes::shadow_map::transparent::TransparentShadowMapPass;

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
