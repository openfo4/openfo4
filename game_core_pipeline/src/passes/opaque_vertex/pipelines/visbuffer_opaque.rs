use game_common::components::Color;
use game_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, DepthStencilAttachment, FragmentStage,
    PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, Texture, TextureViewDescriptor,
    VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ColorTargetState, CompareOp, DepthStencilState, DescriptorType, Face, FrontFace,
    LoadOp, PrimitiveTopology, ShaderStages, StoreOp, TextureFormat,
};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::camera::{Camera, CameraUniform};
use crate::passes::DEPTH_FORMAT;

static VS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/visbuffer_vert.slang");
static FS_SAHDER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/visbuffer_frag.slang");

pub struct ForwardParams<'a> {
    pub camera: &'a Camera,
    pub positions: &'a Buffer,
    pub offsets: &'a Buffer,
    pub instances: &'a Buffer,
    pub index_buffer: &'a Buffer,
    pub materials: &'a Buffer,
    pub indirect_buffer: &'a Buffer,
    pub visbuffer: &'a Texture,
    pub depth: &'a Texture,
}

pub struct VisbufferForwardPipeline {
    pipeline: Pipeline,
}

impl VisbufferForwardPipeline {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(PipelineConfig {
                shaders: vec![
                    ShaderConfig { source: &VS_SHADER },
                    ShaderConfig { source: &FS_SAHDER },
                ],
                bindings: vec![
                    PipelineBinding {
                        name: "camera",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "positions",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "offsets",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "instances",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "index_buffer",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "materials",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                ],
                builder: Box::new(|_queue, shaders, _ctx| {
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
                                format: TextureFormat::Rg32Uint,
                                blend: None,
                            }],
                            front_face: FrontFace::Ccw,
                            cull_mode: Some(Face::Back),
                            depth_stencil_state: Some(DepthStencilState {
                                format: DEPTH_FORMAT,
                                depth_write_enabled: true,
                                depth_compare_op: CompareOp::Greater,
                            }),
                            spec_consts: Default::default(),
                        }),
                    ]
                }),
            }),
        }
    }

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: ForwardParams<'_>) {
        let camera = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&CameraUniform::new(
                params.camera.transform,
                params.camera.projection,
            )),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = self.pipeline.prepare(queue);
        pass.set("camera", ShaderStages::VERTEX, &camera);
        pass.set("positions", ShaderStages::VERTEX, params.positions);
        pass.set("offsets", ShaderStages::VERTEX, params.offsets);
        pass.set("instances", ShaderStages::VERTEX, params.instances);
        pass.set("index_buffer", ShaderStages::VERTEX, params.index_buffer);
        pass.set("materials", ShaderStages::VERTEX, params.materials);

        let visbuffer_view = params
            .visbuffer
            .create_view(&TextureViewDescriptor::default());

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Forward",
            color_attachments: &[RenderPassColorAttachment {
                target: &visbuffer_view,
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: params.depth,
                load_op: LoadOp::Clear(0.0),
                store_op: StoreOp::Store,
            }),
        });

        pass.draw_indirect(params.indirect_buffer);
    }
}
