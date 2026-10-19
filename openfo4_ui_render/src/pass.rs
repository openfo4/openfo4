use std::collections::HashMap;
use std::sync::Mutex;

use bytemuck::{Pod, Zeroable};
use crossbeam_channel::Receiver;
use glam::UVec2;
use openfo4_render::api::{
    BindingResource, BufferInitDescriptor, CommandQueue, FragmentStage, PipelineStage,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture, TextureRegion,
    TextureViewDescriptor, VertexStage,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{
    AddressMode, BlendState, BufferUsage, ColorTargetState, DescriptorType,
    DrawIndexedIndirectCommand, FilterMode, FrontFace, ImageDataLayout, IndexFormat, LoadOp,
    PrimitiveTopology, SamplerDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureFormat,
    TextureUsage,
};
use openfo4_render::buffer::GpuBuffer;
use openfo4_render::buffer::slab::{CompactSlabBuffer, SlabBuffer, SlabIndex};
use openfo4_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use openfo4_render::buffer::texture_slab::{TextureSlab, TextureSlabIndex};
use openfo4_render::graph::{Node, RenderContext, SlotLabel};
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::{Event, MeshId, TextureId};

static VS_SHADER: ShaderSource = include_shader!("/game_ui_render/shaders/ui_vs.slang");
static FS_SHADER: ShaderSource = include_shader!("/game_ui_render/shaders/ui_fs.slang");

pub struct UiPass {
    state: Mutex<State>,
    pipelines: Mutex<HashMap<TextureFormat, Pipeline<ColorTargetState>>>,
    sampler: Sampler,
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
            state: Mutex::new(State {
                events,
                meshes: HashMap::new(),
                textures,
                texture_id_to_index: HashMap::new(),
                vertex_buffer: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
                index_buffer: SubAllocatedGrowableBuffer::new(queue, BufferUsage::INDEX),
                instance_buffer: SlabBuffer::new(BufferUsage::STORAGE),
                draw_calls: CompactSlabBuffer::new(BufferUsage::INDIRECT),
                default_texture,
            }),
            pipelines: Mutex::default(),
            sampler: queue.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
                compare: None,
            }),
        }
    }
}

impl Node for UiPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let target = ctx.read::<Texture>(SlotLabel::SURFACE).cloned().unwrap();

        let state = &mut *self.state.lock().unwrap();
        state.update(ctx.queue);

        let mut pipelines = self.pipelines.lock().unwrap();
        let pipeline = pipelines
            .entry(target.format())
            .or_insert_with(|| make_pipeline(target.format()));

        if state.meshes.is_empty() {
            return;
        }

        let viewport_size = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&target.size()),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let index_buffer = state.index_buffer.buffer();
        let indirect_buffer = state
            .draw_calls
            .buffer(ctx.queue)
            .slice(..state.meshes.len() * size_of::<DrawIndexedIndirectCommand>());

        let mut pass = pipeline.prepare(ctx.queue);
        pass.set("viewport_size", ShaderStages::VERTEX, &viewport_size);
        pass.set(
            "vertices",
            ShaderStages::VERTEX,
            state.vertex_buffer.buffer(),
        );
        pass.set(
            "instances",
            ShaderStages::VERTEX,
            state.instance_buffer.buffer(ctx.queue),
        );
        let views = state.textures.views();
        pass.set(
            "textures",
            ShaderStages::FRAGMENT,
            BindingResource::TextureArray(&views),
        );
        pass.set("sampler", ShaderStages::FRAGMENT, &self.sampler);

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "UI",
            color_attachments: &[RenderPassColorAttachment {
                target: &target.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        pass.set_index_buffer(index_buffer, IndexFormat::U32);
        pass.draw_indexed_indirect(&indirect_buffer);
    }
}

struct State {
    events: Receiver<Event>,
    meshes: HashMap<MeshId, MeshState>,
    textures: TextureSlab,
    texture_id_to_index: HashMap<TextureId, TextureSlabIndex>,
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
                // We must wait for the next frame to continue.
                Event::Barrier => break,
            }
        }
    }
}

struct MeshState {
    vertex_buffer: usize,
    index_buffer: usize,
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

fn make_pipeline(format: TextureFormat) -> Pipeline<ColorTargetState> {
    let target = ColorTargetState {
        format,
        blend: Some(BlendState::PREMULTIPLIED_ALPHA),
    };

    Pipeline::new_with_ctx(
        PipelineConfig {
            shaders: vec![
                ShaderConfig { source: &VS_SHADER },
                ShaderConfig { source: &FS_SHADER },
            ],
            bindings: vec![
                PipelineBinding {
                    name: "viewport_size",
                    stages: ShaderStages::VERTEX,
                    kind: DescriptorType::Uniform,
                },
                PipelineBinding {
                    name: "vertices",
                    stages: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                },
                PipelineBinding {
                    name: "instances",
                    stages: ShaderStages::VERTEX,
                    kind: DescriptorType::Storage,
                },
                PipelineBinding {
                    name: "textures",
                    stages: ShaderStages::FRAGMENT,
                    kind: DescriptorType::Texture,
                },
                PipelineBinding {
                    name: "sampler",
                    stages: ShaderStages::FRAGMENT,
                    kind: DescriptorType::Sampler,
                },
            ],
            builder: Box::new(|_queue, shaders, target| {
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
                        targets: core::slice::from_ref(target),
                        front_face: FrontFace::Ccw,
                        cull_mode: None,
                        depth_stencil_state: None,
                        spec_consts: Default::default(),
                    }),
                ]
            }),
        },
        target,
    )
}
