use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_common::components::Color;
use game_render::api::{
    BufferDescriptor, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BufferUsage, DrawIndirectCommand, FilterMode, LoadOp, SamplerDescriptor, StoreOp,
    TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::opaque_vertex::pipelines::drawcall_gen::{
    DrawcallGenParams, DrawcallGenPipeline,
};
use crate::passes::opaque_vertex::pipelines::visbuffer_alpha_masked::{
    AlphaMaskParams, VisbufferAlphaMaskPipeline,
};
use crate::passes::opaque_vertex::pipelines::visbuffer_opaque::{
    ForwardParams, VisbufferForwardPipeline,
};
use crate::passes::opaque_vertex::pipelines::visbuffer_shading::{
    ShadingParameters, ShadingPipeline,
};
use crate::passes::opaque_vertex::state::Instance;
use crate::passes::{DEPTH_FORMAT, HDR_FORMAT, HDR_TEXTURE, MeshStateImpl, OPAQUE_DEPTH, State};

pub struct OpaqueVertexForwardPass {
    state: Arc<Mutex<State>>,
    drawcall_gen_pipeline: Mutex<DrawcallGenPipeline>,
    forward_opaque: Mutex<VisbufferForwardPipeline>,
    forward_alpha_mask: Mutex<VisbufferAlphaMaskPipeline>,
    shading_pipeline: Mutex<ShadingPipeline>,
    linear_sampler: Sampler,
}

impl OpaqueVertexForwardPass {
    pub fn new(queue: &CommandQueue<'_>, state: Arc<Mutex<State>>) -> Self {
        let linear_sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
        });

        Self {
            state,
            linear_sampler,
            forward_opaque: Mutex::new(VisbufferForwardPipeline::new()),
            shading_pipeline: Mutex::new(ShadingPipeline::new()),
            forward_alpha_mask: Mutex::new(VisbufferAlphaMaskPipeline::new()),
            drawcall_gen_pipeline: Mutex::new(DrawcallGenPipeline::new()),
        }
    }

    fn render_scene_with_camera(
        &self,
        ctx: &mut RenderContext<'_, '_>,
        state: &mut State,
        scene: SceneId,
        camera: CameraId,
        size: UVec2,
    ) {
        let _span = trace_span!("ForwardPass::render_scene_with_camera").entered();

        let depth_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        let scene = state.scenes.get_mut(&scene).unwrap();
        let camera = scene.cameras.get(&camera).unwrap();

        let textures = state.textures.views();
        let materials = state.material_slab.buffer(ctx.queue);

        let directional_lights = scene.directional_lights_buffer.buffer(ctx.queue);
        let point_lights = scene.point_lights_buffer.buffer(ctx.queue);
        let spot_lights = scene.spot_lights_buffer.buffer(ctx.queue);

        let MeshStateImpl::Vertex(mesh_state) = &mut state.mesh else {
            unreachable!();
        };

        if mesh_state.num_opauqe_instances == 0 && mesh_state.num_transparent_mask_instances == 0 {
            self.clear_pass(ctx, size);
            return;
        }

        let positions = mesh_state.positions.buffer();
        let normals = mesh_state.normals.buffer();
        let tangents = mesh_state.tangents.buffer();
        let uvs = mesh_state.uvs.buffer();
        let colors = mesh_state.colors.buffer();

        let offsets = mesh_state.mesh_offsets.buffer(ctx.queue);
        let index_buffer = mesh_state.index_buffer.buffer();

        let visbuffer = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: TextureFormat::Rg32Uint,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        let opauqe_instances = ctx.queue.create_buffer(&BufferDescriptor {
            size: mesh_state
                .opaque_instances
                .buffer(ctx.queue)
                .size()
                .max(size_of::<Instance>() as u64),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        // Opaque Forward
        if mesh_state.num_opauqe_instances != 0 {
            let mut pipeline = self.drawcall_gen_pipeline.lock();

            let instances_in = mesh_state.opaque_instances.buffer(ctx.queue);
            let draws = ctx.queue.create_buffer(&BufferDescriptor {
                size: mesh_state.num_opauqe_instances as u64
                    * size_of::<DrawIndirectCommand>() as u64,
                usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
                flags: UsageFlags::empty(),
            });

            pipeline.run(
                ctx.queue,
                DrawcallGenParams {
                    instance_count: mesh_state.num_opauqe_instances,
                    instances_in,
                    instances_out: &opauqe_instances,
                    draws: &draws,
                },
            );

            let mut pipeline = self.forward_opaque.lock();

            pipeline.run(
                ctx.queue,
                ForwardParams {
                    camera: &camera,
                    positions,
                    offsets,
                    instances: &opauqe_instances,
                    index_buffer,
                    materials,
                    indirect_buffer: &draws,
                    visbuffer: &visbuffer,
                    depth: &depth_texture,
                },
            );
        }

        let alpha_mask_instances = ctx.queue.create_buffer(&BufferDescriptor {
            size: mesh_state
                .transparent_mask_instances
                .buffer(ctx.queue)
                .size()
                .max(size_of::<Instance>() as u64),
            flags: UsageFlags::empty(),
            usage: BufferUsage::STORAGE,
        });

        // Alpha Mask forward
        if mesh_state.num_transparent_mask_instances != 0 {
            let instance_in = mesh_state.transparent_mask_instances.buffer(ctx.queue);

            let draws = ctx.queue.create_buffer(&BufferDescriptor {
                size: mesh_state.num_transparent_mask_instances as u64
                    * size_of::<DrawIndirectCommand>() as u64,
                usage: BufferUsage::STORAGE | BufferUsage::INDIRECT,
                flags: UsageFlags::empty(),
            });

            {
                let mut pipeline = self.drawcall_gen_pipeline.lock();
                pipeline.run(
                    ctx.queue,
                    DrawcallGenParams {
                        instance_count: mesh_state.num_transparent_mask_instances,
                        instances_in: instance_in,
                        instances_out: &alpha_mask_instances,
                        draws: &draws,
                    },
                );
            }

            {
                let mut pipeline = self.forward_alpha_mask.lock();
                pipeline.run(
                    ctx.queue,
                    AlphaMaskParams {
                        camera,
                        positions,
                        uvs,
                        colors,
                        index_buffer,
                        materials,
                        instances: &alpha_mask_instances,
                        textures: &textures,
                        sampler: &self.linear_sampler,
                        visbuffer: &visbuffer,
                        depth: &depth_texture,
                        indirect_buffer: &draws,
                        offsets,
                    },
                );
            }
        }

        let output_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE,
        });

        {
            let mut pipeline = self.shading_pipeline.lock();

            let constants = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&ShadingPipelineConstants {
                    camera: CameraUniform::new(camera.transform, camera.projection),
                    num_directional_lights: scene.directional_lights.len() as u32,
                    num_point_lights: scene.point_lights.len() as u32,
                    num_spot_lights: scene.spot_lights.len() as u32,
                    _pad0: 0,
                    viewport_size: size,
                }),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            });

            pipeline.run(
                ctx.queue,
                ShadingParameters {
                    viewport_size: size,
                    constants: &constants,
                    positions,
                    normals,
                    uvs,
                    tangents,
                    colors,
                    offsets,
                    instances: &opauqe_instances,
                    index_buffer,
                    instances_transparent_mask: &alpha_mask_instances,
                    directional_lights,
                    point_lights,
                    spot_lights,
                    materials,
                    textures: &textures,
                    sampler: &self.linear_sampler,
                    visbuffer: &visbuffer,
                    output: &output_texture,
                },
            );
        }

        ctx.write(HDR_TEXTURE, output_texture).unwrap();
        ctx.write(OPAQUE_DEPTH, depth_texture).unwrap();
    }

    fn clear_pass(&self, ctx: &mut RenderContext<'_, '_>, size: UVec2) {
        let texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE,
        });

        let depth_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Forward",
            color_attachments: &[RenderPassColorAttachment {
                target: &texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: &depth_texture,
                load_op: LoadOp::Clear(0.0),
                store_op: StoreOp::Store,
            }),
        });

        ctx.write(HDR_TEXTURE, texture).unwrap();
        ctx.write(OPAQUE_DEPTH, depth_texture).unwrap();
    }
}

impl Node for OpaqueVertexForwardPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("OpaqueVertexForwardPass::render").entered();

        let size = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().size();

        let mut state = self.state.lock();
        for (scene_id, scene) in state.scenes.iter() {
            for (camera_id, camera) in scene.cameras.iter() {
                if camera.target == ctx.render_target {
                    let scene_id = *scene_id;
                    let camera_id = *camera_id;
                    self.render_scene_with_camera(ctx, &mut state, scene_id, camera_id, size);
                    return;
                }
            }
        }

        // If we don't have any camera to render we just
        // emit a black texture.
        self.clear_pass(ctx, size);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct ShadingPipelineConstants {
    camera: CameraUniform,
    num_directional_lights: u32,
    num_point_lights: u32,
    num_spot_lights: u32,
    _pad0: u32,
    viewport_size: UVec2,
}
