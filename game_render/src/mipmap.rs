use std::num::NonZeroU32;

use game_common::components::Color;
use game_tracing::trace_span;
use hashbrown::HashMap;
use naga::back::spv::PipelineOptions;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};

use crate::api::{
    BindingResource, CommandQueue, DescriptorSetDescriptor, DescriptorSetEntry,
    DescriptorSetLayout, DescriptorSetLayoutDescriptor, FragmentStage, Pipeline,
    PipelineDescriptor, PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, Sampler,
    Texture, TextureViewDescriptor, VertexStage,
};
use crate::backend::{
    AddressMode, ColorTargetState, DescriptorBinding, DescriptorType, Face, FilterMode, FrontFace,
    LoadOp, PrimitiveTopology, SamplerDescriptor, ShaderStages, StoreOp, TextureFormat,
};
use crate::shader::Shader;

const SHADER: &'static str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/mipmap.wgsl"));

#[derive(Debug)]
pub struct MipMapGenerator {
    layout: DescriptorSetLayout,
    sampler: Sampler,
    pipelines: RwLock<HashMap<TextureFormat, Pipeline>>,
    vs_shader: Shader,
    fs_shader: Shader,
}

impl MipMapGenerator {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        let layout = queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
            bindings: &[
                DescriptorBinding {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    kind: DescriptorType::Texture,
                    count: NonZeroU32::MIN,
                },
                DescriptorBinding {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    kind: DescriptorType::Sampler,
                    count: NonZeroU32::MIN,
                },
            ],
        });

        let sampler = queue.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mipmap_filter: FilterMode::Linear,
            compare: None,
        });

        let module = naga::front::wgsl::parse_str(SHADER).unwrap();
        let info = naga::valid::Validator::new(Default::default(), Default::default())
            .validate(&module)
            .unwrap();
        let vs_shader = naga::back::spv::write_vec(
            &module,
            &info,
            &Default::default(),
            Some(&PipelineOptions {
                shader_stage: naga::ShaderStage::Vertex,
                entry_point: "vs_main".to_owned(),
            }),
        )
        .unwrap();
        let fs_shader = naga::back::spv::write_vec(
            &module,
            &info,
            &Default::default(),
            Some(&PipelineOptions {
                shader_stage: naga::ShaderStage::Fragment,
                entry_point: "fs_main".to_owned(),
            }),
        )
        .unwrap();
        let vs_shader = Shader::from_spirv(bytemuck::cast_slice(&vs_shader)).unwrap();
        let fs_shader = Shader::from_spirv(bytemuck::cast_slice(&fs_shader)).unwrap();

        Self {
            layout: layout.clone(),
            sampler,
            pipelines: RwLock::new(HashMap::new()),
            vs_shader,
            fs_shader,
        }
    }

    fn acquire_pipeline(
        &self,
        queue: &CommandQueue<'_>,
        format: TextureFormat,
    ) -> MappedRwLockReadGuard<'_, Pipeline> {
        loop {
            {
                let pipelines = self.pipelines.read();
                if let Ok(guard) = RwLockReadGuard::try_map(pipelines, |map| map.get(&format)) {
                    return guard;
                }
            }

            // This path will likely be reached only once for each unique
            // TextureFormat.
            std::hint::cold_path();

            let mut pipelines = self.pipelines.write();
            if pipelines.contains_key(&format) {
                continue;
            }

            let pipeline = queue.create_pipeline(&PipelineDescriptor {
                descriptors: &[&self.layout],
                push_constant_ranges: &[],
                stages: &[
                    PipelineStage::Vertex(VertexStage {
                        shader: &self.vs_shader,
                        entry: "vs_main",
                        topology: PrimitiveTopology::TriangleList,
                        spec_consts: Default::default(),
                    }),
                    PipelineStage::Fragment(FragmentStage {
                        shader: &self.fs_shader,
                        entry: "fs_main",
                        targets: &[ColorTargetState {
                            format,
                            blend: None,
                        }],
                        front_face: FrontFace::Ccw,
                        cull_mode: Some(Face::Back),
                        depth_stencil_state: None,
                        spec_consts: Default::default(),
                    }),
                ],
            });

            pipelines.insert(format, pipeline);
        }
    }

    pub fn generate_mipmaps(&self, queue: &CommandQueue<'_>, texture: &Texture) {
        let _span = trace_span!("MipMapGenerator::generate_mipmaps").entered();

        let pipeline = self.acquire_pipeline(queue, texture.format());

        let mut mips = Vec::new();
        for mip_level in 0..texture.mip_levels() {
            let mip = texture.create_view(&TextureViewDescriptor {
                base_mip_level: mip_level,
                mip_levels: Some(1),
            });

            mips.push(mip);
        }

        for views in mips.windows(2) {
            let src_view = &views[0];
            let dst_view = &views[1];

            let set = queue.create_descriptor_set(&DescriptorSetDescriptor {
                layout: &self.layout,
                entries: &[
                    DescriptorSetEntry {
                        binding: 0,
                        resource: BindingResource::Texture(src_view),
                    },
                    DescriptorSetEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut render_pass = queue.run_render_pass(&RenderPassDescriptor {
                name: "Mipmap Generation",
                color_attachments: &[RenderPassColorAttachment {
                    target: dst_view,
                    load_op: LoadOp::Clear(Color::BLACK),
                    store_op: StoreOp::Store,
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&pipeline);
            render_pass.set_descriptor_set(0, &set);
            render_pass.draw(0..3, 0..1);
        }
    }
}
