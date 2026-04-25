use game_render::api::{
    BindingResource, Buffer, CommandQueue, ComputePassDescriptor, ComputeStage, PipelineStage,
    Sampler, Texture, TextureView, TextureViewDescriptor,
};
use game_render::backend::{DescriptorType, ShaderStages};
use game_render::shader::{ShaderConfig, ShaderLanguage, ShaderSource};
use glam::UVec2;

use crate::passes::pipeline::{Pipeline, PipelineBinding};

const SHADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/opaque_vertex/visbuffer_shading.slang"
);

const TILE_SIZE: u32 = 16;

#[derive(Copy, Clone, Debug)]
pub struct ShadingParameters<'a> {
    pub viewport_size: UVec2,
    pub constants: &'a Buffer,
    pub positions: &'a Buffer,
    pub normals: &'a Buffer,
    pub uvs: &'a Buffer,
    pub tangents: &'a Buffer,
    pub colors: &'a Buffer,
    pub offsets: &'a Buffer,
    pub instances: &'a Buffer,
    pub index_buffer: &'a Buffer,
    pub instances_transparent_mask: &'a Buffer,
    pub directional_lights: &'a Buffer,
    pub point_lights: &'a Buffer,
    pub spot_lights: &'a Buffer,
    pub materials: &'a Buffer,
    pub textures: &'a [&'a TextureView],
    pub sampler: &'a Sampler,
    pub visbuffer: &'a Texture,
    pub output: &'a Texture,
    pub shadow_sampler: &'a Sampler,
    pub shadow_light_vps: &'a Buffer,
    pub shadow_maps: &'a [&'a TextureView],
    pub terrain_layers: &'a Buffer,
    pub clamped_sampler: &'a Sampler,
}

pub struct ShadingPipeline {
    pipeline: Pipeline,
}

impl ShadingPipeline {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(
                vec![ShaderConfig {
                    language: ShaderLanguage::Slang,
                    source: ShaderSource::File(SHADER.into()),
                }],
                Box::new(|_queue, shaders| {
                    let tile_size = shaders[0]
                        .spec_constants()
                        .iter()
                        .find(|v| v.name.as_ref().is_some_and(|v| v == "TILE_SIZE"))
                        .unwrap()
                        .id;

                    vec![PipelineStage::Compute(ComputeStage {
                        shader: &shaders[0],
                        entry: "main",
                        spec_consts: [(tile_size, TILE_SIZE.into())].into_iter().collect(),
                    })]
                }),
                &[
                    PipelineBinding {
                        name: "constants",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "visbuffer",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Texture,
                    },
                    PipelineBinding {
                        name: "output",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::StorageTexture,
                    },
                    PipelineBinding {
                        name: "positions",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "normals",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "uvs",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "tangents",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "colors",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "offsets",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "instances",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "index_buffer",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "instances_transparent_mask",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "directional_lights",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "point_lights",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "spot_lights",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "materials",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "textures",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Texture,
                    },
                    PipelineBinding {
                        name: "sampler",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Sampler,
                    },
                    PipelineBinding {
                        name: "shadow_maps",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Texture,
                    },
                    PipelineBinding {
                        name: "shadow_light_vps",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "shadow_sampler",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Sampler,
                    },
                    PipelineBinding {
                        name: "terrain_layers",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "clamped_sampler",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Sampler,
                    },
                ],
            ),
        }
    }

    pub fn run(&mut self, queue: &CommandQueue<'_>, params: ShadingParameters<'_>) {
        let mut pass = self.pipeline.prepare(queue);
        pass.set("constants", ShaderStages::COMPUTE, params.constants);
        pass.set("positions", ShaderStages::COMPUTE, params.positions);
        pass.set("normals", ShaderStages::COMPUTE, params.normals);
        pass.set("uvs", ShaderStages::COMPUTE, params.uvs);
        pass.set("tangents", ShaderStages::COMPUTE, params.tangents);
        pass.set("colors", ShaderStages::COMPUTE, params.colors);
        pass.set("offsets", ShaderStages::COMPUTE, params.offsets);
        pass.set("instances", ShaderStages::COMPUTE, params.instances);
        pass.set("index_buffer", ShaderStages::COMPUTE, params.index_buffer);
        pass.set(
            "instances_transparent_mask",
            ShaderStages::COMPUTE,
            params.instances_transparent_mask,
        );
        pass.set(
            "directional_lights",
            ShaderStages::COMPUTE,
            params.directional_lights,
        );
        pass.set("point_lights", ShaderStages::COMPUTE, params.point_lights);
        pass.set("spot_lights", ShaderStages::COMPUTE, params.spot_lights);
        pass.set("materials", ShaderStages::COMPUTE, params.materials);
        pass.set(
            "textures",
            ShaderStages::COMPUTE,
            BindingResource::TextureArray(params.textures),
        );
        pass.set("sampler", ShaderStages::COMPUTE, params.sampler);
        let visbuffer_view = params
            .visbuffer
            .create_view(&TextureViewDescriptor::default());
        pass.set("visbuffer", ShaderStages::COMPUTE, &visbuffer_view);
        let output_view = params.output.create_view(&TextureViewDescriptor::default());
        pass.set("output", ShaderStages::COMPUTE, &output_view);

        // Shadow
        pass.set(
            "shadow_maps",
            ShaderStages::COMPUTE,
            BindingResource::TextureArray(params.shadow_maps),
        );
        pass.set(
            "shadow_light_vps",
            ShaderStages::COMPUTE,
            BindingResource::Buffer(params.shadow_light_vps),
        );
        pass.set(
            "shadow_sampler",
            ShaderStages::COMPUTE,
            params.shadow_sampler,
        );
        pass.set(
            "terrain_layers",
            ShaderStages::COMPUTE,
            params.terrain_layers,
        );
        pass.set(
            "clamped_sampler",
            ShaderStages::COMPUTE,
            params.clamped_sampler,
        );

        let mut pass = pass.run_compute_pass(&ComputePassDescriptor {
            name: "Visbuffer Shading",
        });

        let x = params.viewport_size.x.div_ceil(TILE_SIZE);
        let y = params.viewport_size.y.div_ceil(TILE_SIZE);
        pass.dispatch(x, y, 1);
    }
}
