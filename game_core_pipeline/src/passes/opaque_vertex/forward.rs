use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_common::components::Color;
use game_render::api::{
    BufferDescriptor, BufferInitDescriptor, CommandQueue, DepthStencilAttachment,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture, TextureViewDescriptor,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BufferUsage, CompareOp, FilterMode, LoadOp, SamplerDescriptor, StoreOp,
    TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::graph::{Node, RenderContext, SlotLabel};
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::forward::ForwardParams;
use crate::passes::forward::opaque::OpaqueForwardPass;
use crate::passes::forward::transparent::TransparentForwardPass;
use crate::passes::mesh::Instance;
use crate::passes::opaque_vertex::pipelines::visbuffer_shading::{
    ShadingParameters, ShadingPipeline,
};
use crate::passes::opaque_vertex::shadow::compute_directional_csm_shadow_params;
use crate::passes::shadow_map::{ShadowMapParams, ShadowMapPass};
use crate::passes::sky::SkyPass;
use crate::passes::update::{RawAmbientLight, RawSceneData};
use crate::passes::{DEPTH_FORMAT, HDR_FORMAT, HDR_TEXTURE, OPAQUE_DEPTH, State};

pub struct OpaqueVertexForwardPass {
    state: Arc<Mutex<State>>,
    shading_pipeline: Mutex<ShadingPipeline>,
    linear_sampler: Sampler,
    sky: SkyPass,
    shadow_sampler: Sampler,
    clamped_sampler: Sampler,
    opaque: Mutex<OpaqueForwardPass>,
    transparent: Mutex<TransparentForwardPass>,
    shadow_map: Mutex<ShadowMapPass>,
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
            compare: None,
        });

        Self {
            sky: { SkyPass::new(state.lock().sky.clone(), queue) },
            state,
            linear_sampler,
            shading_pipeline: Mutex::new(ShadingPipeline::new()),
            shadow_sampler: queue.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareOp::GreaterEqual),
            }),
            clamped_sampler: queue.create_sampler(&SamplerDescriptor {
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mipmap_filter: FilterMode::Linear,
                compare: None,
            }),
            opaque: Mutex::new(OpaqueForwardPass::new()),
            transparent: Mutex::new(TransparentForwardPass::new()),
            shadow_map: Mutex::new(ShadowMapPass::new()),
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

        if state.mesh.instances.num_opaque == 0 && state.mesh.instances.num_transparent == 0 {
            self.clear_pass(ctx, size);
            return;
        }

        let visbuffer = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: TextureFormat::Rg32Uint,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        let opauqe_instances = ctx.queue.create_buffer(&BufferDescriptor {
            size: state
                .mesh
                .instances
                .opaque
                .buffer(ctx.queue)
                .size()
                .max(size_of::<Instance>()),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        let mut shadow_light_vps = Vec::new();
        let mut shadow_maps = Vec::new();

        let mut visbuffer_load_op = LoadOp::Clear(Color::BLACK);
        let mut depth_load_op = LoadOp::Clear(0.0);

        // Opaque Forward
        if state.mesh.instances.num_opaque != 0 {
            let mut pipeline = self.opaque.lock();

            pipeline.run(
                ctx.queue,
                ForwardParams {
                    camera: CameraUniform::new(camera.transform, camera.projection),
                    mesh: &mut state.mesh,
                    depth: &depth_texture,
                    visbuffer: &visbuffer.create_view(&TextureViewDescriptor::default()),
                    instances_out: &opauqe_instances,
                    depth_load_op,
                    visbuffer_load_op,
                    materials: materials,
                    textures: &textures,
                    sampler: &self.linear_sampler,
                },
            );

            visbuffer_load_op = LoadOp::Load;
            depth_load_op = LoadOp::Load;

            // FIXME: More than one light.
            for light in scene.directional_lights.values().take(1) {
                let cascades = compute_directional_csm_shadow_params(camera, light.direction);

                let mut pipeline = self.shadow_map.lock();

                for cascade in cascades {
                    let shadow_depth = ctx.queue.create_texture(&TextureDescriptor {
                        size: cascade.size,
                        mip_levels: 1,
                        format: TextureFormat::Depth32Float,
                        usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
                    });

                    pipeline.run(
                        ctx.queue,
                        ShadowMapParams {
                            light: cascade.light_vp,
                            mesh: &mut state.mesh,
                            depth: &shadow_depth,
                            materials,
                            textures: &textures,
                            sampler: &self.linear_sampler,
                        },
                    );

                    shadow_light_vps.push(cascade.light_vp);
                    shadow_maps.push(shadow_depth.create_view(&TextureViewDescriptor::default()));
                }
            }
        }

        let alpha_mask_instances = ctx.queue.create_buffer(&BufferDescriptor {
            size: state
                .mesh
                .instances
                .transparent
                .buffer(ctx.queue)
                .size()
                .max(size_of::<Instance>()),
            flags: UsageFlags::empty(),
            usage: BufferUsage::STORAGE,
        });

        // Alpha Mask forward
        if state.mesh.instances.num_transparent != 0 {
            let mut pipeline = self.transparent.lock();

            pipeline.run(
                ctx.queue,
                ForwardParams {
                    mesh: &mut state.mesh,
                    visbuffer: &visbuffer.create_view(&TextureViewDescriptor::default()),
                    depth: &depth_texture,
                    materials,
                    camera: CameraUniform::new(camera.transform, camera.projection),
                    textures: &textures,
                    sampler: &self.linear_sampler,
                    visbuffer_load_op,
                    depth_load_op,
                    instances_out: &alpha_mask_instances,
                },
            );
        }

        let positions = state.mesh.positions.buffer();
        let normals = state.mesh.normals.buffer();
        let tangents = state.mesh.tangents.buffer();
        let uvs = state.mesh.uvs.buffer();
        let colors = state.mesh.colors.buffer();

        let offsets = state.mesh.mesh_offsets.buffer(ctx.queue);
        let index_buffer = state.mesh.indices.buffer();

        let output_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE
                | TextureUsage::TRANSFER_DST,
        });

        self.sky.run(ctx, camera, &output_texture);

        {
            let mut pipeline = self.shading_pipeline.lock();

            let mut ambient = RawAmbientLight::zeroed();
            for light in scene.ambient_lights.values() {
                ambient.top = (light.top * light.brightness).as_rgb();
                ambient.bottom = (light.bottom * light.brightness).as_rgb();
                ambient.left = (light.left * light.brightness).as_rgb();
                ambient.right = (light.right * light.brightness).as_rgb();
                ambient.front = (light.front * light.brightness).as_rgb();
                ambient.back = (light.back * light.brightness).as_rgb();
            }

            let constants = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&ShadingPipelineConstants {
                    scene: RawSceneData { ambient },
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

            let shadow_light_vps = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(&shadow_light_vps),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });
            let shadow_map_views = shadow_maps.iter().collect::<Vec<_>>();

            let terrain_layers = state.terrain_layers.buffer();

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
                    shadow_light_vps: &shadow_light_vps,
                    shadow_sampler: &self.shadow_sampler,
                    shadow_maps: &shadow_map_views,
                    terrain_layers,
                    clamped_sampler: &self.clamped_sampler,
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
    scene: RawSceneData,
    camera: CameraUniform,
    num_directional_lights: u32,
    num_point_lights: u32,
    num_spot_lights: u32,
    _pad0: u32,
    viewport_size: UVec2,
}
