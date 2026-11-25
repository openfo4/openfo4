use glam::Mat4;
use openfo4_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, ComputePassDescriptor, ComputeStage, PipelineStage,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{BufferUsage, DescriptorType, ShaderStages};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::camera::Frustum;

static SHADER: ShaderSource =
    include_shader!("/openfo4_core_pipeline/shaders/lightcull/lightcull.slang");

const WORKGROUP_SIZE: u32 = 64;

pub struct LightcullParams<'a> {
    pub frustum: Frustum,
    pub view: Mat4,
    pub input: &'a Buffer,
    pub output: &'a Buffer,
    pub input_count: u32,
    pub output_count: &'a Buffer,
}

pub struct LightcullPass {
    pipeline: Pipeline,
}

impl LightcullPass {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(PipelineConfig {
                shaders: vec![ShaderConfig { source: &SHADER }],
                bindings: vec![
                    PipelineBinding {
                        name: "frustum",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "view",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "input",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "output",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "input_count",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "output_count",
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

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: LightcullParams) {
        let frustum = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.frustum),
            flags: UsageFlags::empty(),
            usage: BufferUsage::UNIFORM,
        });
        let input_count = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.input_count),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });
        let view = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&params.view),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = self.pipeline.prepare(queue);
        pass.set("frustum", ShaderStages::COMPUTE, &frustum);
        pass.set("view", ShaderStages::COMPUTE, &view);
        pass.set("input", ShaderStages::COMPUTE, params.input);
        pass.set("output", ShaderStages::COMPUTE, params.output);
        pass.set("input_count", ShaderStages::COMPUTE, &input_count);
        pass.set("output_count", ShaderStages::COMPUTE, params.output_count);

        let mut pass = pass.run_compute_pass(&ComputePassDescriptor {
            name: "Light culling",
        });

        let workgroups = params.input_count.div_ceil(WORKGROUP_SIZE);
        pass.dispatch(workgroups, 1, 1);
    }
}
