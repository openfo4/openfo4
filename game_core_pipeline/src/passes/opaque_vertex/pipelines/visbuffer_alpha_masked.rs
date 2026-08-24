use game_common::components::Color;
use game_render::api::{
    BindingResource, Buffer, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    FragmentStage, PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, Sampler,
    Texture, TextureView, TextureViewDescriptor, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ColorTargetState, CompareOp, DepthStencilState, DescriptorType, Face, FrontFace,
    LoadOp, PrimitiveTopology, ShaderStages, StoreOp, TextureFormat,
};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::camera::{Camera, CameraUniform};
use crate::passes::DEPTH_FORMAT;
use crate::passes::pipeline::{Pipeline, PipelineBinding};

static VS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/transparent_mask_vert.slang");
static FS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/transparent_mask_frag.slang");

#[derive(Copy, Clone, Debug)]
pub struct AlphaMaskParams<'a> {
    pub camera: &'a Camera,
    pub positions: &'a Buffer,
    pub uvs: &'a Buffer,
    pub colors: &'a Buffer,
    pub index_buffer: &'a Buffer,
    pub materials: &'a Buffer,
    pub instances: &'a Buffer,
    pub textures: &'a [&'a TextureView],
    pub sampler: &'a Sampler,
    pub visbuffer: &'a Texture,
    pub depth: &'a Texture,
    pub indirect_buffer: &'a Buffer,
    pub offsets: &'a Buffer,
    pub clear_output_and_depth: bool,
}

pub struct VisbufferAlphaMaskPipeline {
    pipeline: Pipeline,
}

impl VisbufferAlphaMaskPipeline {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(
                [&VS_SHADER, &FS_SHADER],
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
                &[
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
                        stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "index_buffer",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "materials",
                        stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "uvs",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "colors",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "textures",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                    },
                    PipelineBinding {
                        name: "sampler",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Sampler,
                    },
                ],
            ),
        }
    }

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: AlphaMaskParams<'_>) {
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
        pass.set("uvs", ShaderStages::VERTEX, params.uvs);
        pass.set("colors", ShaderStages::VERTEX, params.colors);
        pass.set("index_buffer", ShaderStages::VERTEX, params.index_buffer);
        pass.set("offsets", ShaderStages::VERTEX, params.offsets);
        pass.set(
            "instances",
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            params.instances,
        );
        pass.set(
            "materials",
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            params.materials,
        );
        pass.set(
            "textures",
            ShaderStages::FRAGMENT,
            BindingResource::TextureArray(params.textures),
        );
        pass.set("sampler", ShaderStages::FRAGMENT, params.sampler);

        let visbuffer_view = params
            .visbuffer
            .create_view(&TextureViewDescriptor::default());
        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Visbuffer Alpha Mask",
            color_attachments: &[RenderPassColorAttachment {
                target: &visbuffer_view,
                load_op: if params.clear_output_and_depth {
                    LoadOp::Clear(Color::BLACK)
                } else {
                    LoadOp::Load
                },
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: params.depth,
                load_op: if params.clear_output_and_depth {
                    LoadOp::Clear(0.0)
                } else {
                    LoadOp::Load
                },
                store_op: StoreOp::Store,
            }),
        });

        pass.draw_indirect(params.indirect_buffer);
    }
}
