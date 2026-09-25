use game_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, ComputePassDescriptor, ComputeStage, PipelineStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{BufferUsage, DescriptorType, ShaderStages};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

static SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/opaque_vertex/drawcall_gen.slang");

const WORKGROUP_SIZE: u32 = 64;

#[derive(Copy, Clone, Debug)]
pub struct DrawcallGenParams<'a> {
    pub instance_count: u32,
    pub instances_in: &'a Buffer,
    pub instances_out: &'a Buffer,
    pub draws: &'a Buffer,
}

pub struct DrawcallGenPipeline {
    pipeline: Pipeline,
}

impl DrawcallGenPipeline {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(PipelineConfig {
                shaders: vec![ShaderConfig { source: &SHADER }],
                bindings: vec![
                    PipelineBinding {
                        name: "instance_count",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "instances_in",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "instances_out",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "draws",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                ],
                builder: Box::new(|_, shaders, _ctx| {
                    vec![PipelineStage::Compute(ComputeStage {
                        shader: &shaders[0],
                        entry: "main",
                        spec_consts: Default::default(),
                    })]
                }),
            }),
        }
    }

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: DrawcallGenParams<'_>) {
        let instance_count = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.instance_count),
            flags: UsageFlags::empty(),
            usage: BufferUsage::UNIFORM,
        });

        let mut pass = self.pipeline.prepare(queue);
        pass.set("instance_count", ShaderStages::COMPUTE, &instance_count);
        pass.set("instances_in", ShaderStages::COMPUTE, params.instances_in);
        pass.set("instances_out", ShaderStages::COMPUTE, params.instances_out);
        pass.set("draws", ShaderStages::COMPUTE, params.draws);

        let mut pass = pass.run_compute_pass(&ComputePassDescriptor {
            name: "Drawcall Generation",
        });

        let workgroups = params.instance_count.div_ceil(WORKGROUP_SIZE);
        pass.dispatch(workgroups, 1, 1);
    }
}
