mod opaque;

use game_render::api::{Buffer, BufferInitDescriptor, CommandQueue, Texture, TextureRegion};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{BufferUsage, LoadOp, TextureFormat};
use glam::Mat4;

use crate::passes::mesh::MeshState;
use crate::passes::shadow_map::opaque::OpaqueShadowMapPass;

const SHADOW_MAP_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub struct ShadowMapPass {
    opaque: OpaqueShadowMapPass,
}

impl ShadowMapPass {
    pub fn new() -> Self {
        Self {
            opaque: OpaqueShadowMapPass::new(),
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

        let load_op = LoadOp::Clear(0.0);

        let camera = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.light),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        if params.mesh.instances.num_opaque != 0 {
            self.opaque.run(
                queue,
                OpaqueShadowMapParams {
                    mesh: params.mesh,
                    camera: &camera,
                    load_op,
                    depth: params.depth,
                    materials: params.materials,
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
}

struct OpaqueShadowMapParams<'a> {
    mesh: &'a mut MeshState,
    camera: &'a Buffer,
    load_op: LoadOp<f32>,
    depth: &'a Texture,
    materials: &'a Buffer,
}
