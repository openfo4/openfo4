use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_common::components::Color;
use game_render::api::{
    Buffer, BufferInitDescriptor, FragmentStage, PipelineStage, RenderPassColorAttachment,
    RenderPassDescriptor, Texture, TextureRegion, TextureViewDescriptor, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ColorTargetState, DescriptorType, Face, FrontFace, IndexFormat, LoadOp,
    PrimitiveTopology, ShaderStages, StoreOp,
};
use game_render::graph::RenderContext;
use game_render::mesh::Mesh;
use game_render::shader::{ShaderConfig, ShaderLanguage, ShaderSource};
use glam::{Mat4, Vec3};
use parking_lot::Mutex;

use crate::camera::Camera;
use crate::passes::HDR_FORMAT;
use crate::passes::pipeline::{Pipeline, PipelineBinding};
use crate::sky::SkyMaterial;

const VS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sky/sky_vs.slang");
const FS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sky/sky_fs.slang");

pub struct SkyPass {
    pipeline: Mutex<Pipeline>,
    state: Arc<Mutex<SkyState>>,
    sky_mesh: Mutex<Option<SkyMesh>>,
    sky_material: Mutex<Option<Buffer>>,
}

impl SkyPass {
    pub fn new(state: Arc<Mutex<SkyState>>) -> Self {
        Self {
            pipeline: Mutex::new(Pipeline::new(
                vec![
                    ShaderConfig {
                        source: ShaderSource::File(VS_SHADER.into()),
                        language: ShaderLanguage::Slang,
                    },
                    ShaderConfig {
                        source: ShaderSource::File(FS_SHADER.into()),
                        language: ShaderLanguage::Slang,
                    },
                ],
                Box::new(|_queue, shaders| {
                    vec![
                        PipelineStage::Vertex(VertexStage {
                            shader: &shaders[0],
                            entry: "main",
                            topology: PrimitiveTopology::TriangleList,
                            spec_consts: Default::default(),
                        }),
                        PipelineStage::Fragment(FragmentStage {
                            shader: &shaders[1],
                            entry: "main",
                            targets: &[ColorTargetState {
                                format: HDR_FORMAT,
                                blend: None,
                            }],
                            front_face: FrontFace::Ccw,
                            cull_mode: Some(Face::Back),
                            depth_stencil_state: None,
                            spec_consts: Default::default(),
                        }),
                    ]
                }),
                &[
                    PipelineBinding {
                        kind: DescriptorType::Uniform,
                        name: "constants",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Storage,
                        name: "positions",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Storage,
                        name: "colors",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Uniform,
                        name: "sky_colors",
                        stages: ShaderStages::FRAGMENT,
                    },
                ],
            )),
            state,
            sky_material: Mutex::new(None),
            sky_mesh: Mutex::new(None),
        }
    }
}

impl SkyPass {
    pub fn run<'a>(&self, ctx: &'a RenderContext<'_, 'a>, camera: &Camera, target: &Texture) {
        // Update state
        let new_sky_mesh = { self.state.lock().atomosphere.take() };
        let new_sky_material = { self.state.lock().sky_colors.take() };

        if let Some(mesh) = new_sky_mesh {
            let positions = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(mesh.positions()),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });

            let colors = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(mesh.colors()),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });

            let indices = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(mesh.indicies().as_ref().unwrap().as_u32()),
                usage: BufferUsage::INDEX,
                flags: UsageFlags::empty(),
            });

            *self.sky_mesh.lock() = Some(SkyMesh {
                positions,
                colors,
                indices,
                num_indices: mesh.indicies().as_ref().unwrap().as_u32().len() as u32,
            });
        }

        if let Some(material) = new_sky_material {
            *self.sky_material.lock() = Some(ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&RawSkyMaterial {
                    sky_lower: material.sky_lower.as_rgb(),
                    _pad0: 0,
                    sky_upper: material.sky_upper.as_rgb(),
                    _pad1: 0,
                    horizon: material.horizon.as_rgb(),
                    _pad2: 0,
                }),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            }));
        }

        let (Some(mesh), Some(material)) = (&*self.sky_mesh.lock(), &*self.sky_material.lock())
        else {
            ctx.queue.clear_texture(
                TextureRegion {
                    texture: target,
                    mip_level: 0,
                },
                [0; 4],
            );

            return;
        };

        let mut pipeline = self.pipeline.lock();

        // Fix the camera translation at the origin so that the skybox
        // never moves relative to the camera.
        let mut static_camera = camera.clone();
        static_camera.transform.translation = Vec3::ZERO;

        let constants = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&Constants {
                camera_view_proj: static_camera.view_projection_matrix(),
                transform: Mat4::IDENTITY,
            }),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = pipeline.prepare(ctx.queue);

        pass.set("constants", ShaderStages::VERTEX, &constants);
        pass.set("positions", ShaderStages::VERTEX, &mesh.positions);
        pass.set("colors", ShaderStages::VERTEX, &mesh.colors);
        pass.set("sky_colors", ShaderStages::FRAGMENT, material);

        let target_view = target.create_view(&TextureViewDescriptor::default());
        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Sky",
            color_attachments: &[RenderPassColorAttachment {
                target: &target_view,
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        pass.set_index_buffer(&mesh.indices, IndexFormat::U32);
        pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Constants {
    camera_view_proj: Mat4,
    transform: Mat4,
}

#[derive(Debug, Default)]
pub struct SkyState {
    pub atomosphere: Option<Mesh>,
    pub sky_colors: Option<SkyMaterial>,
}

struct SkyMesh {
    positions: Buffer,
    colors: Buffer,
    indices: Buffer,
    num_indices: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawSkyMaterial {
    sky_upper: [f32; 3],
    _pad0: u32,
    sky_lower: [f32; 3],
    _pad1: u32,
    horizon: [f32; 3],
    _pad2: u32,
}
