use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Mutex;

use bytemuck::{Pod, Zeroable};
use crossbeam_channel::Receiver;
use game_render::api::{
    BindingResource, CommandQueue, DescriptorSetDescriptor, DescriptorSetEntry,
    DescriptorSetLayout, DescriptorSetLayoutDescriptor, FragmentStage, Pipeline,
    PipelineDescriptor, PipelineStage, RenderPassColorAttachment, RenderPassDescriptor, Sampler,
    Texture, TextureRegion, TextureViewDescriptor, VertexStage,
};
use game_render::backend::{
    AddressMode, BlendState, BufferUsage, ColorTargetState, DescriptorBinding, DescriptorType,
    DrawIndexedIndirectCommand, FilterMode, FrontFace, ImageDataLayout, IndexFormat, LoadOp,
    PrimitiveTopology, PushConstantRange, SamplerDescriptor, ShaderStages, StoreOp,
    TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{CompactSlabBuffer, SlabBuffer, SlabIndex};
use game_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use game_render::buffer::texture_slab::{TextureSlab, TextureSlabIndex};
use game_render::camera::RenderTarget;
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_render::pipeline_cache::{PipelineBuilder, PipelineCache};
use game_render::shader::{Shader, ShaderConfig, ShaderLanguage, ShaderSource};
use glam::UVec2;

use crate::{Event, MeshId, TextureId};

const VS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/ui_vs.slang");
const FS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/ui_fs.slang");

/// The default texture array capacity.
const DEFAULT_TEXTURE_CAPACITY: NonZeroU32 = NonZeroU32::new(8).unwrap();

#[derive(Debug)]
struct UiPipeline {
    descriptor_set_layout: DescriptorSetLayout,
    sampler: Sampler,
    capacity: NonZeroU32,
    pipeline: PipelineCache<UiPipelineBuilder>,
}

impl UiPipeline {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        Self::new_with_capacity(queue, DEFAULT_TEXTURE_CAPACITY)
    }

    fn new_with_capacity(queue: &CommandQueue<'_>, capacity: NonZeroU32) -> Self {
        let descriptor_set_layout =
            queue.create_descriptor_set_layout(&DescriptorSetLayoutDescriptor {
                bindings: &[
                    DescriptorBinding {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                    DescriptorBinding {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                        count: NonZeroU32::MIN,
                    },
                    DescriptorBinding {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                        count: capacity,
                    },
                    DescriptorBinding {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Sampler,
                        count: NonZeroU32::MIN,
                    },
                ],
            });

        let sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
        });

        let pipeline = PipelineCache::new(
            UiPipelineBuilder {
                descriptor_set_layout: descriptor_set_layout.clone(),
            },
            vec![
                ShaderConfig {
                    source: ShaderSource::File(VS_SHADER.into()),
                    language: ShaderLanguage::Slang,
                },
                ShaderConfig {
                    source: ShaderSource::File(FS_SHADER.into()),
                    language: ShaderLanguage::Slang,
                },
            ],
        );

        Self {
            descriptor_set_layout,
            sampler,
            capacity,
            pipeline,
        }
    }
}

#[derive(Debug)]
struct UiPipelineBuilder {
    descriptor_set_layout: DescriptorSetLayout,
}

impl PipelineBuilder for UiPipelineBuilder {
    fn build(
        &self,
        queue: &CommandQueue<'_>,
        shaders: &[Shader],
        format: TextureFormat,
    ) -> Pipeline {
        queue.create_pipeline(&PipelineDescriptor {
            stages: &[
                PipelineStage::Vertex(VertexStage {
                    shader: &shaders[0],
                    entry: "main",
                    topology: PrimitiveTopology::TriangleList,
                }),
                PipelineStage::Fragment(FragmentStage {
                    shader: &shaders[1],
                    entry: "main",
                    targets: &[ColorTargetState {
                        format,
                        blend: Some(BlendState::PREMULTIPLIED_ALPHA),
                    }],
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    depth_stencil_state: None,
                }),
            ],
            descriptors: &[&self.descriptor_set_layout],
            push_constant_ranges: &[PushConstantRange {
                range: 0..8,
                stages: ShaderStages::VERTEX,
            }],
        })
    }
}

pub struct UiPass {
    pipeline: Mutex<UiPipeline>,
    state: Mutex<State>,
}

impl UiPass {
    pub fn new(queue: &CommandQueue<'_>, events: Receiver<Event>) -> Self {
        let default_texture = queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::TRANSFER_DST,
        });

        queue.write_texture(
            TextureRegion {
                texture: &default_texture,
                mip_level: 0,
            },
            &[255, 255, 255, 255],
            ImageDataLayout {
                format: TextureFormat::Rgba8UnormSrgb,
            },
        );

        let mut textures =
            TextureSlab::new(default_texture.create_view(&TextureViewDescriptor::default()));
        let default_texture =
            textures.insert(default_texture.create_view(&TextureViewDescriptor::default()));

        Self {
            pipeline: Mutex::new(UiPipeline::new(queue)),
            state: Mutex::new(State {
                events,
                meshes: HashMap::new(),
                textures,
                texture_id_to_index: HashMap::new(),
                targets: HashMap::new(),
                vertex_buffer: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
                index_buffer: SubAllocatedGrowableBuffer::new(queue, BufferUsage::INDEX),
                instance_buffer: SlabBuffer::new(BufferUsage::STORAGE),
                draw_calls: CompactSlabBuffer::new(BufferUsage::INDIRECT),
                default_texture,
            }),
        }
    }
}

impl Node for UiPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let target = ctx.read::<Texture>(SlotLabel::SURFACE).cloned().unwrap();

        let mut pipeline = self.pipeline.lock().unwrap();
        let state = &mut *self.state.lock().unwrap();

        state.update(ctx.queue);

        if state.meshes.is_empty() {
            return;
        }

        // We have to recreate the pipeline with increased capacity if we have
        // more textures than we can store with the current pipeline layout.
        if state.textures.len() as u32 > pipeline.capacity.get() {
            // We cannot place more than u32::MAX textures.
            assert!(state.textures.len() <= u32::MAX as usize);

            let new_capacity = (state.textures.len() as u32)
                .checked_next_power_of_two()
                .unwrap_or(state.textures.len() as u32);
            let new_capacity = NonZeroU32::new(new_capacity).unwrap();

            tracing::debug!("recreating UiPipeline with capacity {}", new_capacity);
            *pipeline = UiPipeline::new_with_capacity(&mut ctx.queue, new_capacity);
        }

        let textures = state.textures.views();

        let instance_buffer = state.instance_buffer.buffer(ctx.queue);
        let vertex_buffer = state.vertex_buffer.buffer();
        let index_buffer = state.index_buffer.buffer();

        let descriptor_set = ctx.queue.create_descriptor_set(&DescriptorSetDescriptor {
            layout: &pipeline.descriptor_set_layout,
            entries: &[
                DescriptorSetEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(vertex_buffer),
                },
                DescriptorSetEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(instance_buffer),
                },
                DescriptorSetEntry {
                    binding: 2,
                    resource: BindingResource::TextureArray(&textures),
                },
                DescriptorSetEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });

        let indirect_buffer = state
            .draw_calls
            .buffer(ctx.queue)
            .slice(..(state.meshes.len() * size_of::<DrawIndexedIndirectCommand>()) as u64);

        let pipeline = pipeline.pipeline.get(ctx.queue, target.format());

        let mut render_pass = ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "UI",
            color_attachments: &[RenderPassColorAttachment {
                target: &target.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&target.size()));
        render_pass.set_descriptor_set(0, &descriptor_set);
        render_pass.set_index_buffer(index_buffer, IndexFormat::U32);
        render_pass.draw_indexed_indirect(&indirect_buffer);
    }
}

struct State {
    events: Receiver<Event>,
    meshes: HashMap<MeshId, MeshState>,
    textures: TextureSlab,
    texture_id_to_index: HashMap<TextureId, TextureSlabIndex>,
    targets: HashMap<RenderTarget, Vec<MeshId>>,
    vertex_buffer: SubAllocatedGrowableBuffer<RawVertex>,
    index_buffer: SubAllocatedGrowableBuffer<u32>,
    instance_buffer: SlabBuffer<RawInstance>,
    draw_calls: CompactSlabBuffer<DrawIndexedIndirectCommand>,
    default_texture: TextureSlabIndex,
}

impl State {
    fn update(&mut self, queue: &CommandQueue<'_>) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::CreateMesh(mesh, id) => {
                    let vertices: Vec<_> = mesh
                        .vertices
                        .iter()
                        .map(|v| RawVertex {
                            position: v.position.to_array(),
                            uv: v.uv.to_array(),
                            color: v.color.to_array(),
                        })
                        .collect();

                    let vertex_buffer_offset = self.vertex_buffer.insert(queue, &vertices);
                    let index_buffer_offset = self.index_buffer.insert(queue, &mesh.indices);

                    let texture_index = mesh
                        .texture
                        .map(|texture| *self.texture_id_to_index.get(&texture).unwrap())
                        .unwrap_or(self.default_texture);

                    let instance = self.instance_buffer.insert(&RawInstance { texture_index });

                    let draw_call = self.draw_calls.insert(&DrawIndexedIndirectCommand {
                        index_count: mesh.indices.len() as u32,
                        instance_count: 1,
                        first_index: index_buffer_offset as u32,
                        vertex_offset: vertex_buffer_offset as u32,
                        first_instance: 0,
                    });

                    self.meshes.insert(
                        id,
                        MeshState {
                            vertex_buffer: vertex_buffer_offset,
                            index_buffer: index_buffer_offset,
                            instance,
                            draw_call,
                        },
                    );
                }
                Event::DestroyMesh(id) => {
                    let state = self.meshes.remove(&id).unwrap();

                    self.draw_calls.remove(state.draw_call);
                    self.instance_buffer.remove(state.instance);
                    self.index_buffer.remove(state.index_buffer);
                    self.vertex_buffer.remove(state.vertex_buffer);
                }
                Event::CreateTexture(image, id) => {
                    let texture = queue.create_texture(&TextureDescriptor {
                        format: image.format(),
                        mip_levels: image.mip_levels(),
                        size: UVec2::new(image.root().width(), image.root().height()),
                        usage: TextureUsage::TEXTURE_BINDING | TextureUsage::TRANSFER_DST,
                    });

                    for (mip_level, mip) in image.mips().iter().enumerate() {
                        queue.write_texture(
                            TextureRegion {
                                texture: &texture,
                                mip_level: mip_level as u32,
                            },
                            mip.as_bytes(),
                            ImageDataLayout {
                                format: mip.format(),
                            },
                        );
                    }

                    let index = self
                        .textures
                        .insert(texture.create_view(&TextureViewDescriptor::default()));
                    self.texture_id_to_index.insert(id, index);
                }
                Event::DestroyTexture(id) => {
                    let index = self.texture_id_to_index.remove(&id).unwrap();
                    self.textures.remove(index);
                }
            }
        }
    }
}

struct MeshState {
    vertex_buffer: u64,
    index_buffer: u64,
    instance: SlabIndex,
    draw_call: SlabIndex,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawInstance {
    texture_index: TextureSlabIndex,
}

impl GpuBuffer for RawInstance {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = align_of::<Self>();
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}
