use game_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, DepthStencilAttachment, FragmentStage,
    PipelineStage, RenderPassDescriptor, Texture, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    BufferUsage, CompareOp, DepthStencilState, DescriptorType, Face, FrontFace, LoadOp,
    PrimitiveTopology, ShaderStages, StoreOp, TextureFormat,
};
use glam::Mat4;
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::passes::pipeline::{Pipeline, PipelineBinding};

static VS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/shadows/shadow_map_vs.slang");
static FS_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/shadows/shadow_map_fs.slang");

const SHADOW_MAP_FORMAT: TextureFormat = TextureFormat::Depth32Float;

#[derive(Copy, Clone, Debug)]
pub struct ShadowMapParams<'a> {
    pub light: Mat4,
    pub positions: &'a Buffer,
    pub offsets: &'a Buffer,
    pub instances: &'a Buffer,
    pub index_buffer: &'a Buffer,
    pub materials: &'a Buffer,
    pub shadow_depth: &'a Texture,
    pub indirect_buffer: &'a Buffer,
}

pub struct ShadowMapPipeline {
    pipeline: Pipeline,
}

impl ShadowMapPipeline {
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
                &[
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
            ),
        }
    }

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: ShadowMapParams<'_>) {
        let camera = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.light),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = self.pipeline.prepare(queue);
        pass.set("light_view_proj", ShaderStages::VERTEX, &camera);
        pass.set("positions", ShaderStages::VERTEX, params.positions);
        pass.set("offsets", ShaderStages::VERTEX, params.offsets);
        pass.set("instances", ShaderStages::VERTEX, params.instances);
        pass.set("index_buffer", ShaderStages::VERTEX, params.index_buffer);
        pass.set("materials", ShaderStages::VERTEX, params.materials);

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Shadow Map Generation",
            color_attachments: &[],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: params.shadow_depth,
                load_op: LoadOp::Clear(0.0),
                store_op: StoreOp::Store,
            }),
        });

        pass.draw_indirect(params.indirect_buffer);
    }
}
