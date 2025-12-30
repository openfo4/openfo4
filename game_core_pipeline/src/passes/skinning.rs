use std::sync::Arc;

use game_render::api::{BufferInitDescriptor, ComputePassDescriptor, ComputeStage, PipelineStage};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{BufferUsage, DescriptorType, ShaderStages};
use game_render::graph::{Node, RenderContext};
use game_render::shader::{ShaderConfig, ShaderLanguage, ShaderSource};
use parking_lot::Mutex;

use crate::passes::pipeline::{Pipeline, PipelineBinding};
use crate::passes::{MeshStateImpl, State};

const SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/skinning/skinning.slang"
);

pub struct SkinningPass {
    state: Arc<Mutex<State>>,
    pipeline: Mutex<Pipeline>,
}

impl SkinningPass {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self {
            state,
            pipeline: Mutex::new(Pipeline::new(
                vec![ShaderConfig {
                    source: ShaderSource::File(SHADER.into()),
                    language: ShaderLanguage::Slang,
                }],
                Box::new(|_queue, shaders| {
                    vec![PipelineStage::Compute(ComputeStage {
                        shader: &shaders[0],
                        entry: "main",
                        spec_consts: Default::default(),
                    })]
                }),
                &[
                    PipelineBinding {
                        name: "instance_count",
                        kind: DescriptorType::Uniform,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "positions_in",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "normals_in",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "tangents_in",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "positions_out",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "normals_out",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "tangents_out",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "indices",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "weights",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "bones",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "instances",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                ],
            )),
        }
    }
}

impl Node for SkinningPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let mut state = self.state.lock();
        let mut pipeline = self.pipeline.lock();

        let state = match &mut state.mesh {
            MeshStateImpl::Vertex(state) => state,
            _ => todo!(),
        };

        if state.skin_instance_data.is_empty() {
            return;
        }

        let mut pass = pipeline.prepare(ctx.queue);

        let num_instances = state.skin_instance_data.len() as u32;
        let instance_count = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&num_instances),
            flags: UsageFlags::empty(),
            usage: BufferUsage::UNIFORM,
        });

        pass.set("instance_count", ShaderStages::COMPUTE, &instance_count);

        let positions_in = state.pre_skinned_positions.buffer();
        let normals_in = state.pre_skinned_normals.buffer();
        let tangents_in = state.pre_skinned_tangents.buffer();
        pass.set("positions_in", ShaderStages::COMPUTE, positions_in);
        pass.set("normals_in", ShaderStages::COMPUTE, normals_in);
        pass.set("tangents_in", ShaderStages::COMPUTE, tangents_in);

        let positions_out = state.positions.buffer();
        let normals_out = state.normals.buffer();
        let tangents_out = state.tangents.buffer();
        pass.set("positions_out", ShaderStages::COMPUTE, positions_out);
        pass.set("normals_out", ShaderStages::COMPUTE, normals_out);
        pass.set("tangents_out", ShaderStages::COMPUTE, tangents_out);

        let joint_indices = state.joint_indices.buffer();
        let joint_weights = state.joint_weights.buffer();
        pass.set("indices", ShaderStages::COMPUTE, joint_indices);
        pass.set("weights", ShaderStages::COMPUTE, joint_weights);

        let bones = state.skin_bones.buffer();
        let instances = state.skin_instances.buffer(ctx.queue);
        pass.set("bones", ShaderStages::COMPUTE, bones);
        pass.set("instances", ShaderStages::COMPUTE, instances);

        let mut pass = pass.run_compute_pass(&ComputePassDescriptor { name: "Skinning" });

        // FIXME: Maybe see if we can use dispatch indirect to prevent wasting threads.
        // Currently the vertex count of the largest skin is also used for all smaller skins.
        let vertex_count = state
            .skin_instance_data
            .values()
            .max_by(|a, b| a.vertex_count.cmp(&b.vertex_count))
            .map(|v| v.vertex_count)
            .unwrap();

        pass.dispatch(num_instances, vertex_count, 1);
    }
}
