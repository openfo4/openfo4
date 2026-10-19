use openfo4_render::api::{
    BufferDescriptor, BufferInitDescriptor, CommandQueue, DepthStencilAttachment, FragmentStage,
    PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, VertexStage,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{
    BufferUsage, ColorTargetState, CompareOp, DepthStencilState, DescriptorType,
    DrawIndirectCommand, Face, FrontFace, PrimitiveTopology, ShaderStages, StoreOp, TextureFormat,
};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::passes::DEPTH_FORMAT;
use crate::passes::forward::ForwardParams;
use crate::passes::forward::drawgen::{DrawcallGenParams, DrawcallGenPipeline};

static VS_SHADER: ShaderSource =
    include_shader!("/openfo4_core_pipeline/shaders/opaque_vertex/visbuffer_vert.slang");
static FS_SAHDER: ShaderSource =
    include_shader!("/openfo4_core_pipeline/shaders/opaque_vertex/visbuffer_frag.slang");

pub struct OpaqueForwardPass {
    drawcall_gen: DrawcallGenPipeline,
    forward: Pipeline,
}

impl OpaqueForwardPass {
    pub fn new() -> Self {
        Self {
            drawcall_gen: DrawcallGenPipeline::new(),
            forward: Pipeline::new(PipelineConfig {
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
        if params.mesh.instances.num_opaque == 0 {
            return;
        }

        if params.base_out_instance != 0 {
            todo!();
        }

        let instances_in = params.mesh.instances.opaque.buffer(queue);
        let draws = queue.create_buffer(&BufferDescriptor {
            size: params.mesh.instances.num_opaque as usize * size_of::<DrawIndirectCommand>(),
            usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
            flags: UsageFlags::empty(),
        });

        self.drawcall_gen.run(
            queue,
            DrawcallGenParams {
                instance_count: params.mesh.instances.num_opaque,
                instances_in: instances_in,
                instances_out: params.instances_out,
                draws: &draws,
            },
        );

        let camera = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.camera),
            flags: UsageFlags::empty(),
            usage: BufferUsage::UNIFORM,
        });

        let positions = params.mesh.positions.buffer();
        let offsets = params.mesh.mesh_offsets.buffer(queue);
        let indices = params.mesh.indices.buffer();

        let mut pass = self.forward.prepare(queue);
        pass.set("camera", ShaderStages::VERTEX, &camera);
        pass.set("positions", ShaderStages::VERTEX, positions);
        pass.set("offsets", ShaderStages::VERTEX, offsets);
        pass.set("instances", ShaderStages::VERTEX, params.instances_out);
        pass.set("index_buffer", ShaderStages::VERTEX, indices);
        pass.set("materials", ShaderStages::VERTEX, params.materials);

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Forward Opaque",
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
