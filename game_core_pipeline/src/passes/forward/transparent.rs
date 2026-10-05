use game_render::api::{
    BindingResource, BufferDescriptor, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    FragmentStage, PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, ColorTargetState, CompareOp, DepthStencilState, DescriptorType,
    DrawIndirectCommand, Face, FrontFace, PrimitiveTopology, ShaderStages, StoreOp, TextureFormat,
};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::passes::DEPTH_FORMAT;
use crate::passes::forward::ForwardParams;
use crate::passes::forward::drawgen::{DrawcallGenParams, DrawcallGenPipeline};

static VS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/transparent_mask_vert.slang");
static FS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/transparent_mask_frag.slang");

pub struct TransparentForwardPass {
    drawcall_gen: DrawcallGenPipeline,
    pipeline: Pipeline,
}

impl TransparentForwardPass {
    pub fn new() -> Self {
        Self {
            drawcall_gen: DrawcallGenPipeline::new(),
            pipeline: Pipeline::new(PipelineConfig {
                shaders: vec![
                    ShaderConfig { source: &VS_SHADER },
                    ShaderConfig { source: &FS_SHADER },
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
                    PipelineBinding {
                        name: "base_out_instance",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Uniform,
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
        if params.mesh.instances.num_transparent == 0 {
            return;
        }

        let instances_in = params.mesh.instances.transparent.buffer(queue);
        let draws = queue.create_buffer(&BufferDescriptor {
            size: params.mesh.instances.num_transparent as usize * size_of::<DrawIndirectCommand>(),
            usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
            flags: UsageFlags::empty(),
        });
        let base_out_instances = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.base_out_instance),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        self.drawcall_gen.run(
            queue,
            DrawcallGenParams {
                instance_count: params.mesh.instances.num_transparent,
                instances_in,
                instances_out: params.instances_out,
                draws: &draws,
            },
        );

        let camera = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.camera),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let positions = params.mesh.positions.buffer();
        let uvs = params.mesh.uvs.buffer();
        let colors = params.mesh.colors.buffer();
        let indcies = params.mesh.indices.buffer();
        let offsets = params.mesh.mesh_offsets.buffer(queue);

        let mut pass = self.pipeline.prepare(queue);
        pass.set("camera", ShaderStages::VERTEX, &camera);
        pass.set("positions", ShaderStages::VERTEX, positions);
        pass.set("uvs", ShaderStages::VERTEX, uvs);
        pass.set("colors", ShaderStages::VERTEX, colors);
        pass.set("offsets", ShaderStages::VERTEX, offsets);
        pass.set("index_buffer", ShaderStages::VERTEX, indcies);
        pass.set(
            "instances",
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            params.instances_out,
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
        pass.set(
            "base_out_instance",
            ShaderStages::VERTEX,
            &base_out_instances,
        );

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Forward Transparent",
            color_attachments: &[RenderPassColorAttachment {
                target: params.visbuffer,
                load_op: params.visbuffer_load_op,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: params.depth,
                load_op: params.depth_load_op,
                store_op: StoreOp::Store,
            }),
        });

        pass.draw_indirect(&draws);
    }
}
