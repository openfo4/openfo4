use openfo4_render::api::{
    BufferDescriptor, CommandQueue, DepthStencilAttachment, FragmentStage, PipelineStage,
    RenderPassDescriptor, VertexStage,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{
    BufferUsage, CompareOp, DepthStencilState, DescriptorType, DrawIndirectCommand, Face,
    FrontFace, PrimitiveTopology, ShaderStages, StoreOp,
};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::passes::forward::drawgen::{DrawcallGenParams, DrawcallGenPipeline};
use crate::passes::shadow_map::{SHADOW_MAP_FORMAT, ShadowMapParamsInner};

static VS_SHADER: ShaderSource =
    include_shader!("/openfo4_core_pipeline/shaders/shadows/shadow_map_vs.slang");
static FS_SHADER: ShaderSource =
    include_shader!("/openfo4_core_pipeline/shaders/shadows/shadow_map_fs.slang");

pub struct OpaqueShadowMapPass {
    drawcall_gen: DrawcallGenPipeline,
    pipeline: Pipeline,
}

impl OpaqueShadowMapPass {
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
                        name: "light_view_proj",
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
                            targets: &[],
                            front_face: FrontFace::Ccw,
                            cull_mode: Some(Face::Back),
                            depth_stencil_state: Some(DepthStencilState {
                                format: SHADOW_MAP_FORMAT,
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

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: ShadowMapParamsInner<'_>) {
        if params.mesh.instances.num_opaque == 0 {
            return;
        }

        let instances_in = params.mesh.instances.opaque.buffer(queue);
        let instances_out = queue.create_buffer(&BufferDescriptor {
            size: instances_in.size(),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });
        let draws = queue.create_buffer(&BufferDescriptor {
            size: params.mesh.instances.num_opaque as usize * size_of::<DrawIndirectCommand>(),
            usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
            flags: UsageFlags::empty(),
        });

        self.drawcall_gen.run(
            queue,
            DrawcallGenParams {
                instance_count: params.mesh.instances.num_opaque,
                instances_in,
                instances_out: instances_out.slice(..),
                draws: &draws,
            },
        );

        let positions = params.mesh.positions.buffer();
        let offsets = params.mesh.mesh_offsets.buffer(queue);
        let indices = params.mesh.indices.buffer();

        let mut pass = self.pipeline.prepare(queue);
        pass.set("light_view_proj", ShaderStages::VERTEX, params.camera);
        pass.set("positions", ShaderStages::VERTEX, positions);
        pass.set("offsets", ShaderStages::VERTEX, offsets);
        pass.set("instances", ShaderStages::VERTEX, &instances_out);
        pass.set("index_buffer", ShaderStages::VERTEX, indices);
        pass.set("materials", ShaderStages::VERTEX, params.materials);

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Shadow Map Generation (Opaque)",
            color_attachments: &[],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: params.depth,
                load_op: params.load_op,
                store_op: StoreOp::Store,
            }),
        });

        pass.draw_indirect(&draws);
    }
}
