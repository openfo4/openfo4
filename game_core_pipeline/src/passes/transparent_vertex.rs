//! Vertex pipeline based transparency pass

use std::sync::Arc;

use bytemuck::Zeroable;
use game_render::api::{
    BindingResource, BufferDescriptor, BufferInitDescriptor, CommandQueue, ComputePassDescriptor,
    ComputeStage, DepthStencilAttachment, FragmentStage, PipelineStage, RenderPassDescriptor,
    Sampler, Texture, TextureRegion, TextureViewDescriptor, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BufferUsage, CompareOp, DepthStencilState, DescriptorType,
    DrawIndexedIndirectCommand, Face, FilterMode, FrontFace, IndexFormat, LoadOp,
    PrimitiveTopology, SamplerDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureFormat,
    TextureUsage,
};
use game_render::graph::{Node, RenderContext};
use game_tracing::trace_span;
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_shaderlib::{ShaderSource, include_shader};
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::update::{RawAmbientLight, RawSceneData};
use crate::passes::{DEPTH_FORMAT, HDR_TEXTURE, OPAQUE_DEPTH, State};

static DRAWGEN_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/transparent/drawcall_gen.slang");
static COLOR_VERT_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/transparent/transparent_vert.slang");
static COLOR_FRAG_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/transparent/transparent_frag.slang");
static BLEND_SHADER: ShaderSource =
    include_shader!("/game_core_pipeline/shaders/transparent/transparent_blend.slang");

const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug)]
pub struct TransparentVertexPass {
    state: Arc<Mutex<State>>,
    drawcall_gen_pipeline: Mutex<Pipeline>,
    color_pipeline: Mutex<Pipeline>,
    blend_pipeline: Mutex<Pipeline>,
    linear_sampler: Sampler,
}

impl TransparentVertexPass {
    pub fn new(state: Arc<Mutex<State>>, queue: &CommandQueue<'_>) -> Self {
        Self {
            state,
            drawcall_gen_pipeline: Mutex::new(Pipeline::new(PipelineConfig {
                shaders: vec![ShaderConfig {
                    source: &DRAWGEN_SHADER,
                }],
                bindings: vec![
                    PipelineBinding {
                        name: "instances_in",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "instances_out",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "draws",
                        kind: DescriptorType::Storage,
                        stages: ShaderStages::COMPUTE,
                    },
                    PipelineBinding {
                        name: "instance_count",
                        kind: DescriptorType::Uniform,
                        stages: ShaderStages::COMPUTE,
                    },
                ],
                builder: Box::new(|_queue, shaders, _ctx| {
                    vec![PipelineStage::Compute(ComputeStage {
                        shader: &shaders[0],
                        entry: "main",
                        spec_consts: Default::default(),
                    })]
                }),
            })),
            color_pipeline: Mutex::new(Pipeline::new(PipelineConfig {
                shaders: vec![
                    ShaderConfig {
                        source: &COLOR_VERT_SHADER,
                    },
                    ShaderConfig {
                        source: &COLOR_FRAG_SHADER,
                    },
                ],
                bindings: vec![
                    PipelineBinding {
                        name: "positions",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "normals",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "uvs",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "tangents",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "colors",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "offsets",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "instances",
                        stages: ShaderStages::VERTEX,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "materials",
                        stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "directional_lights",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "point_lights",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "spot_lights",
                        stages: ShaderStages::FRAGMENT,
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
                    PipelineBinding {
                        name: "scene",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "elements",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "heads",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::StorageTexture,
                    },
                    PipelineBinding {
                        name: "frag_counter",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "depth",
                        stages: ShaderStages::FRAGMENT,
                        kind: DescriptorType::Texture,
                    },
                    PipelineBinding {
                        name: "push_constants",
                        stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        kind: DescriptorType::Uniform,
                    },
                ],
                builder: Box::new(|_queue, shaders, _ctx| {
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
                            targets: &[],
                            front_face: FrontFace::Ccw,
                            cull_mode: Some(Face::Back),
                            depth_stencil_state: Some(DepthStencilState {
                                format: DEPTH_FORMAT,
                                depth_write_enabled: false,
                                depth_compare_op: CompareOp::Always,
                            }),
                            spec_consts: Default::default(),
                        }),
                    ]
                }),
            })),
            blend_pipeline: Mutex::new(Pipeline::new(PipelineConfig {
                shaders: vec![ShaderConfig {
                    source: &BLEND_SHADER,
                }],
                bindings: vec![
                    PipelineBinding {
                        name: "output_size",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Uniform,
                    },
                    PipelineBinding {
                        name: "elements",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Storage,
                    },
                    PipelineBinding {
                        name: "heads",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::Texture,
                    },
                    PipelineBinding {
                        name: "output",
                        stages: ShaderStages::COMPUTE,
                        kind: DescriptorType::StorageTexture,
                    },
                ],
                builder: Box::new(|_queue, shaders, _ctx| {
                    vec![PipelineStage::Compute(ComputeStage {
                        shader: &shaders[0],
                        entry: "main",
                        spec_consts: Default::default(),
                    })]
                }),
            })),
            linear_sampler: queue.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                compare: None,
            }),
        }
    }
}

impl TransparentVertexPass {
    fn render_scene_with_camera(
        &self,
        ctx: &mut RenderContext<'_, '_>,
        state: &mut State,
        scene: SceneId,
        camera: CameraId,
    ) {
        let mut drawcall_gen_pipeline = self.drawcall_gen_pipeline.lock();
        let mut color_pipeline = self.color_pipeline.lock();
        let mut blend_pipeline = self.blend_pipeline.lock();

        let scene = state.scenes.get_mut(&scene).unwrap();
        let camera = scene.cameras.get(&camera).unwrap();

        let depth_target = ctx.read::<Texture>(OPAQUE_DEPTH).unwrap();
        let color_target = ctx.read::<Texture>(HDR_TEXTURE).unwrap();
        let size = color_target.size();

        // Size for at least four layers if every pixel is covered.
        // The size of one element is 16 bytes.
        let abuffer_size = size.x as u64 * size.y as u64 * 4 * 16;

        if state.mesh.instances.num_translucent == 0 || size.x == 0 || size.y == 0 {
            return;
        }

        let instance_buffer;
        let indirect_buffer;

        // Drawcall Generation Pass
        {
            let instances_in = state.mesh.instances.translucent.buffer(ctx.queue);
            let instances_out = ctx.queue.create_buffer(&BufferDescriptor {
                size: instances_in.size(),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });
            let draws = ctx.queue.create_buffer(&BufferDescriptor {
                size: state.mesh.instances.num_translucent as u64
                    * size_of::<DrawIndexedIndirectCommand>() as u64,
                usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
                flags: UsageFlags::empty(),
            });
            let instance_count = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&state.mesh.instances.num_translucent),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            });

            let mut pass = drawcall_gen_pipeline.prepare(ctx.queue);
            pass.set("instances_in", ShaderStages::COMPUTE, instances_in);
            pass.set("instances_out", ShaderStages::COMPUTE, &instances_out);
            pass.set("draws", ShaderStages::COMPUTE, &draws);
            pass.set("instance_count", ShaderStages::COMPUTE, &instance_count);
            let mut pass = pass.run_compute_pass(&ComputePassDescriptor {
                name: "Transparent Drawcall Generation Pass",
            });

            let num_dispatches = state
                .mesh
                .instances
                .num_translucent
                .div_ceil(WORKGROUP_SIZE);

            pass.dispatch(num_dispatches, 1, 1);

            instance_buffer = instances_out;
            indirect_buffer = draws;
        }

        let abuffer;
        let heads;

        // Color Pass
        {
            let offsets = state.mesh.mesh_offsets.buffer(ctx.queue);
            let materials = state.material_slab.buffer(ctx.queue);

            let mut pass = color_pipeline.prepare(ctx.queue);
            pass.set(
                "positions",
                ShaderStages::VERTEX,
                state.mesh.positions.buffer(),
            );
            pass.set("normals", ShaderStages::VERTEX, state.mesh.normals.buffer());
            pass.set("uvs", ShaderStages::VERTEX, state.mesh.uvs.buffer());
            pass.set(
                "tangents",
                ShaderStages::VERTEX,
                state.mesh.tangents.buffer(),
            );
            pass.set("colors", ShaderStages::VERTEX, state.mesh.colors.buffer());
            pass.set("offsets", ShaderStages::VERTEX, offsets);
            pass.set("instances", ShaderStages::VERTEX, &instance_buffer);
            pass.set(
                "materials",
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                materials,
            );

            let textures = state.textures.views();

            let directional_lights = scene.directional_lights_buffer.buffer(ctx.queue);
            let point_lights = scene.point_lights_buffer.buffer(ctx.queue);
            let spot_lights = scene.spot_lights_buffer.buffer(ctx.queue);

            let mut ambient = RawAmbientLight::zeroed();
            for light in scene.ambient_lights.values() {
                ambient.top = (light.top * light.brightness).as_rgb();
                ambient.bottom = (light.bottom * light.brightness).as_rgb();
                ambient.left = (light.left * light.brightness).as_rgb();
                ambient.right = (light.right * light.brightness).as_rgb();
                ambient.front = (light.front * light.brightness).as_rgb();
                ambient.back = (light.back * light.brightness).as_rgb();
            }

            let ambient = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&RawSceneData { ambient }),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            });

            pass.set(
                "directional_lights",
                ShaderStages::FRAGMENT,
                directional_lights,
            );
            pass.set("point_lights", ShaderStages::FRAGMENT, point_lights);
            pass.set("spot_lights", ShaderStages::FRAGMENT, spot_lights);
            pass.set(
                "textures",
                ShaderStages::FRAGMENT,
                BindingResource::TextureArray(&textures),
            );
            pass.set("sampler", ShaderStages::FRAGMENT, &self.linear_sampler);
            pass.set("scene", ShaderStages::FRAGMENT, &ambient);

            abuffer = ctx.queue.create_buffer(&BufferDescriptor {
                size: abuffer_size,
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });
            heads = ctx.queue.create_texture(&TextureDescriptor {
                size: size,
                mip_levels: 1,
                format: TextureFormat::R32Uint,
                usage: TextureUsage::TRANSFER_DST
                    | TextureUsage::TEXTURE_BINDING
                    | TextureUsage::STORAGE,
            });

            // Zero out all values of `heads`.
            ctx.queue.clear_texture(
                TextureRegion {
                    texture: &heads,
                    mip_level: 0,
                },
                [0; 4],
            );

            let counter = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                // We need to initialize the counter to 1 so that the
                // value 0 is never read. The value 0 indicates the end of
                // the linked list chain.
                contents: bytemuck::bytes_of(&1_u32),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });

            pass.set("elements", ShaderStages::FRAGMENT, &abuffer);
            let heads_view = heads.create_view(&TextureViewDescriptor::default());
            pass.set("heads", ShaderStages::FRAGMENT, &heads_view);
            pass.set("frag_counter", ShaderStages::FRAGMENT, &counter);
            let depth_view = depth_target.create_view(&TextureViewDescriptor::default());
            pass.set("depth", ShaderStages::FRAGMENT, &depth_view);

            let mut push_constants = [0; 128];
            push_constants[0..80].copy_from_slice(bytemuck::bytes_of(&CameraUniform::new(
                camera.transform,
                camera.projection,
            )));
            push_constants[80..84]
                .copy_from_slice(&(scene.directional_lights.len() as u32).to_ne_bytes());
            push_constants[84..88]
                .copy_from_slice(&(scene.point_lights.len() as u32).to_ne_bytes());
            push_constants[88..92].copy_from_slice(&(scene.spot_lights.len() as u32).to_ne_bytes());

            let push_constants = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: &push_constants,
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            });

            pass.set(
                "push_constants",
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                &push_constants,
            );

            // FIXME: We are attaching a dummy texture in the render pass.
            // This is currently a workaround so that the render pass gets the size
            // from the texture size. The depth function always suceeds.
            // This can be removed once we can specify a render pass viewport size.
            let dummy_target = ctx.queue.create_texture(&TextureDescriptor {
                size: color_target.size(),
                mip_levels: 1,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::RENDER_ATTACHMENT,
            });

            let mut pass = pass.run_render_pass(&RenderPassDescriptor {
                name: "Transparent (Vertex) Color Pass",
                color_attachments: &[],
                // We use the depth buffer from the opaque pass to reject
                // as many pixels as possible early.
                // depth_stencil_attachment: Some(&DepthStencilAttachment {
                //     texture: &depth_target,
                //     load_op: LoadOp::Load,
                //     store_op: StoreOp::Discard,
                // }),
                depth_stencil_attachment: Some(&DepthStencilAttachment {
                    load_op: LoadOp::Load,
                    store_op: StoreOp::Discard,
                    texture: &dummy_target,
                }),
            });

            pass.set_index_buffer(state.mesh.indices.buffer(), IndexFormat::U32);
            pass.draw_indexed_indirect(&indirect_buffer);
        }

        // Blend Pass
        {
            let output_size = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&size),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            });

            let mut pass = blend_pipeline.prepare(ctx.queue);

            pass.set("output_size", ShaderStages::COMPUTE, &output_size);
            pass.set("elements", ShaderStages::COMPUTE, &abuffer);
            let heads_view = heads.create_view(&TextureViewDescriptor::default());
            pass.set("heads", ShaderStages::COMPUTE, &heads_view);
            let colors_view = color_target.create_view(&TextureViewDescriptor::default());
            pass.set("output", ShaderStages::COMPUTE, &colors_view);

            let mut pass = pass.run_compute_pass(&ComputePassDescriptor {
                name: "Transparent Blend Pass",
            });

            let x = size.x.div_ceil(16);
            let y = size.y.div_ceil(16);
            pass.dispatch(x, y, 1);
        }
    }
}

impl Node for TransparentVertexPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("TransparentVertexPass::render").entered();

        let mut state = self.state.lock();
        for (scene_id, scene) in state.scenes.iter() {
            for (camera_id, camera) in scene.cameras.iter() {
                if camera.target == ctx.render_target {
                    let scene_id = *scene_id;
                    let camera_id = *camera_id;
                    self.render_scene_with_camera(ctx, &mut state, scene_id, camera_id);
                    return;
                }
            }
        }
    }
}
