use std::collections::HashMap;
use std::num::NonZeroU32;

use game_render::api::{
    self, BindingResource, CommandQueue, ComputePass, ComputePassDescriptor, DescriptorSet,
    DescriptorSetDescriptor, DescriptorSetEntry, DescriptorSetLayout,
    DescriptorSetLayoutDescriptor, PipelineDescriptor, PipelineStage, RenderPass,
    RenderPassDescriptor,
};
use game_render::backend::{DescriptorBinding, DescriptorType, ShaderStage, ShaderStages};
use game_render::shader::{BindingId, BindingLocation, Shader};
use openfo4_shaderlib::ShaderSource;

#[derive(Clone, Debug)]
pub struct PipelineBinding {
    pub name: &'static str,
    pub stages: ShaderStages,
    pub kind: DescriptorType,
}

pub struct Pipeline {
    config: PipelineConfig,
    stages: Vec<Stage>,
    cur_pipeline: Option<PipelineInstance>,
    compiled_shaders: Option<Vec<Shader>>,
    shaders_changed: bool,
}

/// Configuration parameters for a [`Pipeline`].
pub struct PipelineConfig {
    pub shaders: Vec<ShaderConfig>,
    pub bindings: Vec<PipelineBinding>,
    pub builder:
        Box<dyn for<'a> Fn(&CommandQueue<'_>, &'a [Shader]) -> Vec<PipelineStage<'a>> + Send>,
}

/// Configuration parameters for a shader module.
#[derive(Clone, Debug)]
pub struct ShaderConfig {
    /// The source of the shader module.
    pub source: &'static ShaderSource,
}

impl Pipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            stages: Vec::new(),
            cur_pipeline: None,
            compiled_shaders: None,
            shaders_changed: false,
            config,
        }
    }

    fn rebuild_shaders(&mut self, queue: &CommandQueue<'_>) {
        let mut compiled_shaders = Vec::new();
        for shader in &mut self.config.shaders {
            let s = match shader.source.compile() {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!("failed to recompile shader: {}", err);
                    return;
                }
            };

            compiled_shaders.push(s.clone());
        }

        self.stages.clear();

        let stages = (self.config.builder)(queue, &compiled_shaders);

        let mut prepared_shaders = Vec::new();
        for (stage_index, stage) in stages.iter().enumerate() {
            let shader = match stage {
                PipelineStage::Vertex(s) => s.shader,
                PipelineStage::Fragment(s) => s.shader,
                PipelineStage::Task(s) => s.shader,
                PipelineStage::Mesh(s) => s.shader,
                PipelineStage::Compute(s) => s.shader,
            };

            let mut stage = Stage {
                stage: stage.shader_stage(),
                bindings: HashMap::new(),
            };

            // Extract named bindings from shader source.
            for (index, binding) in shader.bindings().into_iter().enumerate() {
                let Some(name) = binding.name else {
                    panic!("unnamed bindings are not supported");
                };

                let Some(pipeline_binding) = self
                    .config
                    .bindings
                    .iter()
                    .find(|v| v.name == name && v.stages.contains(stage.stage.stages()))
                else {
                    panic!("binding {} not in pipeline", name);
                };

                stage.bindings.insert(
                    name.clone(),
                    Binding {
                        id: binding.id,
                        location: index as u32,
                        ty: pipeline_binding.kind,
                        name,
                    },
                );
            }

            let mut shader = shader.clone();

            // Update shader binding locations.
            for binding in stage.bindings.values() {
                shader.set_binding_location(
                    binding.id,
                    BindingLocation {
                        group: stage_index as u32,
                        binding: binding.location,
                    },
                );
            }

            prepared_shaders.push(shader);

            self.stages.push(stage);
        }

        self.compiled_shaders = Some(prepared_shaders);
        self.shaders_changed = true;
    }

    fn compile_with_limits(
        &mut self,
        queue: &CommandQueue<'_>,
        array_counts: &HashMap<(String, ShaderStage), u32>,
    ) {
        let recreate_new_limits = self.cur_pipeline.as_ref().is_none_or(|pipeline| {
            array_counts.iter().any(|(key, count)| {
                pipeline
                    .limits
                    .get(key)
                    .is_none_or(|cur_count| cur_count < count)
            })
        });

        if !recreate_new_limits && !self.shaders_changed {
            return;
        }

        self.shaders_changed = false;

        let mut layouts = Vec::new();
        for stage in &self.stages {
            let mut bindings = Vec::new();

            for binding in stage.bindings.values() {
                let count = array_counts
                    .get(&(binding.name.clone(), stage.stage))
                    .unwrap_or(&1);

                bindings.push(DescriptorBinding {
                    binding: binding.location,
                    visibility: stage.stage.stages(),
                    kind: binding.ty,
                    count: NonZeroU32::new(*count).unwrap(),
                });
            }

            // FIXME: Backend API is currently borked if bindings are out of order.
            bindings.sort_by(|a, b| a.binding.cmp(&b.binding));

            let set = queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                bindings: &bindings,
            });

            layouts.push(set);
        }

        let layout_refs: Vec<_> = layouts.iter().collect();

        let Some(compiled_shader) = self.compiled_shaders.as_ref() else {
            panic!("initial shader compilation failed");
        };

        let stages = (self.config.builder)(queue, compiled_shader);
        let pipeline = queue.create_pipeline(&PipelineDescriptor {
            stages: &stages,
            descriptors: &layout_refs,
            push_constant_ranges: &[],
        });

        self.cur_pipeline = Some(PipelineInstance {
            limits: array_counts.clone(),
            layouts,
            pipeline,
        });
    }

    pub fn prepare<'b, 'c>(&mut self, queue: &'c CommandQueue<'c>) -> PreparePass<'_, 'b, 'c> {
        // Rebuild shaders on first call or when any shader changed.
        if self.cur_pipeline.is_none() || self.config.shaders.iter().any(|v| v.source.has_changed())
        {
            self.rebuild_shaders(queue);
        }

        PreparePass {
            pipeline: self,
            resources: HashMap::new(),
            array_counts: HashMap::new(),
            queue,
        }
    }
}

struct PipelineInstance {
    limits: HashMap<(String, ShaderStage), u32>,
    layouts: Vec<DescriptorSetLayout>,
    pipeline: api::Pipeline,
}

pub struct PreparePass<'a, 'b, 'c> {
    queue: &'a CommandQueue<'c>,
    pipeline: &'a mut Pipeline,
    resources: HashMap<(String, ShaderStage), BindingResource<'b>>,
    array_counts: HashMap<(String, ShaderStage), u32>,
}

impl<'a, 'b, 'c> PreparePass<'a, 'b, 'c> {
    #[track_caller]
    pub fn set<T>(&mut self, name: &str, stages: ShaderStages, resource: T)
    where
        T: Into<BindingResource<'b>>,
    {
        let resource = resource.into();

        for stage in &self.pipeline.stages {
            if !stage.stage.stages().intersects(stages) {
                continue;
            }

            if !stage.bindings.contains_key(name) {
                // tracing::warn!("no binding {} in stage {:?}", name, stage.stage);
                continue;
            }

            if let BindingResource::TextureArray(textures) = resource {
                self.array_counts
                    .insert((name.to_owned(), stage.stage), textures.len() as u32);
            }

            self.resources
                .insert((name.to_owned(), stage.stage), resource);
        }
    }

    pub fn run_render_pass(mut self, descriptor: &RenderPassDescriptor<'_>) -> RenderPass<'a, 'b> {
        let (pipeline, sets) = self.finalize();

        let mut pass = self.queue.run_render_pass(descriptor);
        pass.set_pipeline(&pipeline);

        for (index, set) in sets.iter().enumerate() {
            pass.set_descriptor_set(index as u32, set);
        }

        pass
    }

    pub fn run_compute_pass(mut self, descriptor: &ComputePassDescriptor) -> ComputePass<'a> {
        let (pipeline, sets) = self.finalize();

        let mut pass = self.queue.run_compute_pass(descriptor);
        pass.set_pipeline(&pipeline);

        for (index, set) in sets.iter().enumerate() {
            pass.set_descriptor_set(index as u32, set);
        }

        pass
    }

    fn finalize(&mut self) -> (api::Pipeline, Vec<DescriptorSet>) {
        self.pipeline
            .compile_with_limits(self.queue, &self.array_counts);
        let pipeline = self.pipeline.cur_pipeline.as_ref().unwrap();

        let mut descriptor_sets = Vec::new();

        for (stage, layout) in self.pipeline.stages.iter().zip(&pipeline.layouts) {
            let mut entries = Vec::new();

            for binding in stage.bindings.values() {
                let Some(resource) = self.resources.get(&(binding.name.clone(), stage.stage))
                else {
                    panic!(
                        "resource {} not defined in stage {:?}",
                        binding.name, stage.stage
                    );
                };

                entries.push(DescriptorSetEntry {
                    binding: binding.location,
                    resource: *resource,
                });
            }

            // FIXME: Backend API is currently borked if bindings are out of order.
            entries.sort_by(|a, b| a.binding.cmp(&b.binding));

            let set = self.queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout,
                entries: &entries,
            });
            descriptor_sets.push(set);
        }

        (pipeline.pipeline.clone(), descriptor_sets)
    }
}

#[derive(Clone, Debug)]
struct Stage {
    stage: ShaderStage,
    bindings: HashMap<String, Binding>,
}

#[derive(Clone, Debug)]
struct Binding {
    id: BindingId,
    location: u32,
    ty: DescriptorType,
    name: String,
}
