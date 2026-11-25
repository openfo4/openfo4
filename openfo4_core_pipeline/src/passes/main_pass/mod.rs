mod forward;
mod lightcull;
mod shadow_map;
mod visbuffer_shading;

use std::mem::offset_of;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, UVec2};
use openfo4_render::api::{
    Buffer, BufferDescriptor, BufferInitDescriptor, CommandQueue, Sampler, Texture, TextureView,
    TextureViewDescriptor,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{
    AddressMode, BufferUsage, ClearValue, CompareOp, FilterMode, LoadOp, SamplerDescriptor,
    TextureDescriptor, TextureFormat, TextureUsage,
};

use crate::camera::{Camera, CameraUniform};
use crate::entities::SceneId;
use crate::passes::main_pass::forward::ForwardParams;
use crate::passes::main_pass::forward::opaque::OpaqueForwardPass;
use crate::passes::main_pass::forward::transparent::TransparentForwardPass;
use crate::passes::main_pass::lightcull::{LightcullParams, LightcullPass};
use crate::passes::main_pass::shadow_map::{
    ShadowMapParams, ShadowMapPass, compute_directional_csm_shadow_params,
};
use crate::passes::main_pass::visbuffer_shading::{ShadingParameters, ShadingPipeline};
use crate::passes::mesh::{Instance, MeshState};
use crate::passes::update::{RawAmbientLight, RawSceneData};
use crate::passes::{SceneData, State};

const VISBUFFER_FORMAT: TextureFormat = TextureFormat::Rg32Uint;

pub struct MainPass {
    linear_sampler: Sampler,
    shadow_sampler: Sampler,
    clamped_sampler: Sampler,
    forward_opaque: OpaqueForwardPass,
    forward_transparent: TransparentForwardPass,
    shadow_map: ShadowMapPass,
    light_cull: LightcullPass,
    visbuffer_shading: ShadingPipeline,
}

impl MainPass {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        let linear_sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            compare: None,
        });
        let shadow_sampler = queue.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareOp::GreaterEqual),
        });
        let clamped_sampler = queue.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mipmap_filter: FilterMode::Linear,
            compare: None,
        });

        Self {
            linear_sampler,
            shadow_sampler,
            clamped_sampler,
            forward_opaque: OpaqueForwardPass::new(),
            forward_transparent: TransparentForwardPass::new(),
            shadow_map: ShadowMapPass::new(),
            light_cull: LightcullPass::new(),
            visbuffer_shading: ShadingPipeline::new(),
        }
    }

    pub fn render(
        &mut self,
        queue: &CommandQueue<'_>,
        size: UVec2,
        state: &mut State,
        camera: Camera,
        scene: SceneId,
        output: &Texture,
        depth: &Texture,
    ) {
        let scene = state.scenes.get_mut(scene).unwrap();
        let textures = state.textures.views();
        let materials = state.material_slab.buffer(queue);
        let terrain_layers = state.terrain_layers.buffer();

        let visbuffer = queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: VISBUFFER_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        let instances = self.render_opaque_meshes(
            queue,
            &mut state.mesh,
            &camera,
            materials,
            &textures,
            &visbuffer,
            &depth,
        );

        let (shadow_maps, shadow_vps) =
            self.render_shadow_maps(queue, &mut state.mesh, scene, &camera, materials, &textures);

        self.shade_visbuffer(
            queue,
            &mut state.mesh,
            scene,
            &camera,
            &visbuffer,
            &shadow_vps,
            &shadow_maps,
            materials,
            &textures,
            terrain_layers,
            &instances,
            output,
        );
    }

    fn render_opaque_meshes(
        &mut self,
        queue: &CommandQueue<'_>,
        meshes: &mut MeshState,
        camera: &Camera,
        materials: &Buffer,
        textures: &[&TextureView],
        visbuffer: &Texture,
        depth: &Texture,
    ) -> Buffer {
        let mut visbuffer_load_op = LoadOp::Clear(ClearValue::BLACK);
        let mut depth_load_op = LoadOp::Clear(0.0);

        let mut instances_out_offset = 0;

        let num_instances = meshes.instances.num_opaque + meshes.instances.num_transparent;
        let instances_out = queue.create_buffer(&BufferDescriptor {
            size: usize::max(num_instances as usize, 1) * size_of::<Instance>(),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        let camera = CameraUniform::new(camera.transform, camera.projection);
        let visbuffer = visbuffer.create_view(&TextureViewDescriptor::default());

        if meshes.instances.num_opaque != 0 {
            let offset = instances_out_offset * size_of::<Instance>();
            let count = meshes.instances.num_opaque as usize * size_of::<Instance>();
            let instances_out = instances_out.slice(offset..offset + count);

            self.forward_opaque.run(
                queue,
                ForwardParams {
                    camera,
                    mesh: meshes,
                    depth: &depth,
                    visbuffer: &visbuffer,
                    instances_out,
                    depth_load_op,
                    visbuffer_load_op,
                    materials: materials,
                    textures: &textures,
                    sampler: &self.linear_sampler,
                    base_out_instance: instances_out_offset as u32,
                },
            );

            instances_out_offset += meshes.instances.num_opaque as usize;
            visbuffer_load_op = LoadOp::Load;
            depth_load_op = LoadOp::Load;
        }

        if meshes.instances.num_transparent != 0 {
            let offset = instances_out_offset * size_of::<Instance>();
            let count = meshes.instances.num_transparent as usize * size_of::<Instance>();
            let instances_out = instances_out.slice(offset..offset + count);

            self.forward_transparent.run(
                queue,
                ForwardParams {
                    mesh: meshes,
                    visbuffer: &visbuffer,
                    depth: &depth,
                    materials,
                    camera,
                    textures: &textures,
                    sampler: &self.linear_sampler,
                    visbuffer_load_op,
                    depth_load_op,
                    instances_out,
                    base_out_instance: instances_out_offset as u32,
                },
            );
        }

        instances_out
    }

    fn render_shadow_maps(
        &mut self,
        queue: &CommandQueue<'_>,
        meshes: &mut MeshState,
        scene: &mut SceneData,
        camera: &Camera,
        materials: &Buffer,
        textures: &[&TextureView],
    ) -> (Vec<TextureView>, Vec<Mat4>) {
        let mut vps = Vec::new();
        let mut maps = Vec::new();

        // FIXME: More than one light.
        for light in scene.directional_lights.values().take(1) {
            let cascades = compute_directional_csm_shadow_params(camera, light.direction);

            for cascade in cascades {
                let shadow_depth = queue.create_texture(&TextureDescriptor {
                    size: cascade.size,
                    mip_levels: 1,
                    format: TextureFormat::Depth32Float,
                    usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
                });

                self.shadow_map.run(
                    queue,
                    ShadowMapParams {
                        light: cascade.light_vp,
                        mesh: meshes,
                        depth: &shadow_depth,
                        materials,
                        textures,
                        sampler: &self.linear_sampler,
                    },
                );

                vps.push(cascade.light_vp);
                maps.push(shadow_depth.create_view(&TextureViewDescriptor::default()));
            }
        }

        // We always need to bind at least one entry.
        if vps.is_empty() {
            let shadow_depth = queue.create_texture(&TextureDescriptor {
                size: UVec2 { x: 1, y: 1 },
                mip_levels: 1,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
            });
            maps.push(shadow_depth.create_view(&TextureViewDescriptor::default()));
            vps.push(Mat4::ZERO);
        }

        (maps, vps)
    }

    fn shade_visbuffer(
        &mut self,
        queue: &CommandQueue<'_>,
        meshes: &mut MeshState,
        scene: &mut SceneData,
        camera: &Camera,
        visbuffer: &Texture,
        shadow_vps: &[Mat4],
        shadow_maps: &[TextureView],
        materials: &Buffer,
        textures: &[&TextureView],
        terrain_layers: &Buffer,
        instances: &Buffer,
        output: &Texture,
    ) {
        let positions = meshes.positions.buffer();
        let normals = meshes.normals.buffer();
        let tangents = meshes.tangents.buffer();
        let uvs = meshes.uvs.buffer();
        let colors = meshes.colors.buffer();

        let offsets = meshes.mesh_offsets.buffer(queue);
        let index_buffer = meshes.indices.buffer();

        let num_point_lights = scene.lights.num_point_lights();
        let directional_lights = scene.directional_lights_buffer.buffer(queue);
        let point_lights = scene.lights.point_lights.buffer(queue);
        let spot_lights = scene.spot_lights_buffer.buffer(queue);

        let output_count = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&0_u32),
            usage: BufferUsage::STORAGE | BufferUsage::TRANSFER_SRC,
            flags: UsageFlags::empty(),
        });
        let point_light_indices = queue.create_buffer(&BufferDescriptor {
            size: (num_point_lights as usize * size_of::<u32>()).max(1),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });
        if num_point_lights != 0 {
            self.light_cull.run(
                queue,
                LightcullParams {
                    frustum: camera.frustum(),
                    view: camera.view_matrix(),
                    input: scene.lights.point_lights_sphere.buffer(queue),
                    input_count: num_point_lights,
                    output: &point_light_indices,
                    output_count: &output_count,
                },
            );
        }

        {
            let mut ambient = RawAmbientLight::zeroed();
            for light in scene.ambient_lights.values() {
                ambient.top = (light.top * light.brightness).as_rgb();
                ambient.bottom = (light.bottom * light.brightness).as_rgb();
                ambient.left = (light.left * light.brightness).as_rgb();
                ambient.right = (light.right * light.brightness).as_rgb();
                ambient.front = (light.front * light.brightness).as_rgb();
                ambient.back = (light.back * light.brightness).as_rgb();
            }

            let constants = queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&ShadingPipelineConstants {
                    scene: RawSceneData { ambient },
                    camera: CameraUniform::new(camera.transform, camera.projection),
                    num_directional_lights: scene.directional_lights.len() as u32,
                    num_point_lights,
                    num_spot_lights: scene.spot_lights.len() as u32,
                    _pad0: 0,
                    viewport_size: visbuffer.size(),
                }),
                usage: BufferUsage::UNIFORM | BufferUsage::TRANSFER_DST,
                flags: UsageFlags::empty(),
            });

            queue.copy_buffer_to_buffer(
                output_count,
                constants.slice(
                    offset_of!(ShadingPipelineConstants, num_point_lights)
                        ..offset_of!(ShadingPipelineConstants, num_point_lights) + size_of::<u32>(),
                ),
            );

            let shadow_light_vps = queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(&shadow_vps),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });
            let shadow_map_views = shadow_maps.iter().collect::<Vec<_>>();

            self.visbuffer_shading.run(
                queue,
                ShadingParameters {
                    viewport_size: visbuffer.size(),
                    constants: &constants,
                    positions,
                    normals,
                    uvs,
                    tangents,
                    colors,
                    offsets,
                    instances,
                    index_buffer,
                    directional_lights,
                    point_lights,
                    spot_lights,
                    materials,
                    textures: &textures,
                    sampler: &self.linear_sampler,
                    visbuffer: &visbuffer,
                    output,
                    shadow_light_vps: &shadow_light_vps,
                    shadow_sampler: &self.shadow_sampler,
                    shadow_maps: &shadow_map_views,
                    terrain_layers,
                    clamped_sampler: &self.clamped_sampler,
                    point_light_indices: &point_light_indices,
                },
            );
        }
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
