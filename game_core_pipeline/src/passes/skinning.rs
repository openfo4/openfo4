use std::sync::Arc;

use game_render::api::{BufferInitDescriptor, ComputePassDescriptor, ComputeStage, PipelineStage};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{BufferUsage, DescriptorType, ShaderStages};
use game_render::graph::{Node, RenderContext};
use openfo4_shaderlib::{ShaderSource, include_shader};
use parking_lot::Mutex;

use crate::passes::State;
use crate::passes::pipeline::{Pipeline, PipelineBinding};

static SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/skinning/skinning.slang");

const WORKGROUP_SIZE: u32 = 64;

pub struct SkinningPass {
    state: Arc<Mutex<State>>,
    pipeline: Mutex<Pipeline>,
}

impl SkinningPass {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self {
            state,
            pipeline: Mutex::new(Pipeline::new(
                [&SHADER],
                Box::new(|_queue, shaders| {
                    let specs = shaders[0].spec_constants();
                    let spec = specs
                        .iter()
                        .find(|v| v.name.as_ref().is_some_and(|v| v == "NUM_THREADS"))
                        .unwrap();

                    vec![PipelineStage::Compute(ComputeStage {
                        shader: &shaders[0],
                        entry: "main",
                        spec_consts: [(spec.id, WORKGROUP_SIZE.into())].into_iter().collect(),
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
        let state = &mut *self.state.lock();
        let mut pipeline = self.pipeline.lock();

        if state.mesh.skin_instance_data.is_empty() {
            return;
        }

        let mut pass = pipeline.prepare(ctx.queue);

        let num_instances = state.mesh.skin_instance_data.len() as u32;
        let instance_count = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&num_instances),
            flags: UsageFlags::empty(),
            usage: BufferUsage::UNIFORM,
        });

        pass.set("instance_count", ShaderStages::COMPUTE, &instance_count);

        let positions_in = state.mesh.pre_skinned_positions.buffer();
        let normals_in = state.mesh.pre_skinned_normals.buffer();
        let tangents_in = state.mesh.pre_skinned_tangents.buffer();
        pass.set("positions_in", ShaderStages::COMPUTE, positions_in);
        pass.set("normals_in", ShaderStages::COMPUTE, normals_in);
        pass.set("tangents_in", ShaderStages::COMPUTE, tangents_in);

        let positions_out = state.mesh.positions.buffer();
        let normals_out = state.mesh.normals.buffer();
        let tangents_out = state.mesh.tangents.buffer();
        pass.set("positions_out", ShaderStages::COMPUTE, positions_out);
        pass.set("normals_out", ShaderStages::COMPUTE, normals_out);
        pass.set("tangents_out", ShaderStages::COMPUTE, tangents_out);

        let joint_indices = state.mesh.joint_indices.buffer();
        let joint_weights = state.mesh.joint_weights.buffer();
        pass.set("indices", ShaderStages::COMPUTE, joint_indices);
        pass.set("weights", ShaderStages::COMPUTE, joint_weights);

        let bones = state.mesh.skin_bones.buffer();
        let instances = state.mesh.skin_instances.buffer(ctx.queue);
        pass.set("bones", ShaderStages::COMPUTE, bones);
        pass.set("instances", ShaderStages::COMPUTE, instances);

        let mut pass = pass.run_compute_pass(&ComputePassDescriptor { name: "Skinning" });

        // FIXME: Maybe see if we can use dispatch indirect to prevent wasting threads.
        // Currently the vertex count of the largest skin is also used for all smaller skins.
        let vertex_count = state
            .mesh
            .skin_instance_data
            .values()
            .max_by(|a, b| a.vertex_count.cmp(&b.vertex_count))
            .map(|v| v.vertex_count)
            .unwrap();
        let vertex_count = vertex_count.div_ceil(WORKGROUP_SIZE);

        pass.dispatch(num_instances, vertex_count, 1);
    }
}
