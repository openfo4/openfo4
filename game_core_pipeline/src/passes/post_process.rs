use std::num::NonZeroU32;
use std::sync::Arc;

use game_render::api::{
    BindingResource, CommandQueue, DescriptorSetDescriptor, DescriptorSetEntry,
    DescriptorSetLayout, DescriptorSetLayoutDescriptor, FragmentStage, Pipeline,
    PipelineDescriptor, PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, Sampler,
    Texture, TextureRegion, TextureViewDescriptor, VertexStage,
};
use game_render::backend::{
    AddressMode, ClearValue, ColorTargetState, DescriptorBinding, DescriptorType, FilterMode,
    FrontFace, ImageDataLayout, LoadOp, PrimitiveTopology, SamplerDescriptor, ShaderStages,
    StoreOp, TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use game_render::texture::MipImage;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

const SHADER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/post_process.wgsl");

pub struct PostProcessPass {
    sampler: Sampler,
    bind_group_layout: DescriptorSetLayout,
    pipelines: PipelineCache<PostProcessPipelineBuilder>,
    src: SlotLabel,
    dst: SlotLabel,
    new_lut: Arc<Mutex<Option<MipImage>>>,
    lut: Mutex<Texture>,
}

impl PostProcessPass {
    pub fn new(
        queue: &CommandQueue<'_>,
        src: SlotLabel,
        dst: SlotLabel,
        new_lut: Arc<Mutex<Option<MipImage>>>,
    ) -> Self {
        let bind_group_layout =
            queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
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
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: NonZeroU32::MIN,
                    },
                ],
            });

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            compare: None,
        });

        let pipelines = PipelineCache::new(
            PostProcessPipelineBuilder {
                descriptor_set_layout: bind_group_layout.clone(),
            },
            vec![ShaderConfig {
                source: ShaderSource::File(SHADER.into()),
                language: ShaderLanguage::Wgsl,
            }],
        );

        let lut = queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::TRANSFER_DST,
        });
        queue.write_texture(
            TextureRegion {
                texture: &lut,
                mip_level: 0,
            },
            &[0, 0, 0, 0],
            ImageDataLayout {
                format: TextureFormat::Rgba8Unorm,
            },
        );

        Self {
            bind_group_layout,
            sampler,
            pipelines,
            src,
            dst,
            new_lut,
            lut: Mutex::new(lut),
        }
    }
}

impl Node for PostProcessPass {
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let _span = trace_span!("PostProcessPass::render").entered();

        let mut lut = self.lut.lock();
        if let Some(img) = self.new_lut.lock().take() {
            *lut = ctx.queue.create_texture(&TextureDescriptor {
                format: img.format(),
                size: UVec2::new(img.root().width(), img.root().height()),
                mip_levels: 1,
                usage: TextureUsage::TEXTURE_BINDING | TextureUsage::TRANSFER_DST,
            });
            ctx.queue.write_texture(
                TextureRegion {
                    texture: &lut,
                    mip_level: 0,
                },
                img.root().as_bytes(),
                ImageDataLayout {
                    format: img.format(),
                },
            );
        }

        let input = ctx.read::<Texture>(self.src).unwrap().clone();
        let output = ctx.read::<Texture>(self.dst).unwrap().clone();

        let pipeline = self.pipelines.get(&mut ctx.queue, output.format());

        let bind_group = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::Texture(
                        &input.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                DescriptorSetEntry {
                    binding: 2,
                    resource: BindingResource::Texture(
                        &lut.create_view(&TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Post Process",
            color_attachments: &[RenderPassColorAttachment {
                target: &output.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(ClearValue::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_descriptor_set(0, &bind_group);
        render_pass.draw(0..3, 0..1);
    }
}

#[derive(Debug)]
struct PostProcessPipelineBuilder {
    descriptor_set_layout: DescriptorSetLayout,
}

impl PipelineBuilder for PostProcessPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        format: TextureFormat,
    ) -> Pipeline {
        let _span = trace_span!("PostProcessPipelineBuilder::build").entered();

        queue.create_pipeline(&PipelineDescriptor {
            descriptors: &[&self.descriptor_set_layout],
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "vs_main",
                    topology: PrimitiveTopology::TriangleList,
                    spec_consts: Default::default(),
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[0],
                    entry: "fs_main",
                    targets: &[ColorTargetState {
                        format,
                        blend: None,
                    }],
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    depth_stencil_state: None,
                    spec_consts: Default::default(),
                }),
            ],
            push_constant_ranges: &[],
        })
    }
}
