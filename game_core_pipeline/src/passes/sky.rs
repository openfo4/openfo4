mod clouds;

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use game_common::components::{Color, Transform};
use game_render::api::{
    Buffer, BufferInitDescriptor, CommandQueue, FragmentStage, PipelineStage,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, Texture, TextureRegion, TextureView,
    TextureViewDescriptor, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BlendState, BufferUsage, ColorTargetState, DescriptorType, Face, FilterMode,
    FrontFace, ImageDataLayout, IndexFormat, LoadOp, PrimitiveTopology, SamplerDescriptor,
    ShaderStages, StoreOp, TextureDescriptor, TextureUsage,
};
use game_render::graph::RenderContext;
use game_render::mesh::Mesh;
use game_render::shader::{ShaderConfig, ShaderLanguage, ShaderSource};
use game_render::texture::image::MipImage;
use glam::{Mat4, UVec2, Vec3};
use parking_lot::Mutex;

use crate::camera::Camera;
use crate::passes::HDR_FORMAT;
use crate::passes::pipeline::{Pipeline, PipelineBinding};
use crate::passes::sky::clouds::CloudsPass;
use crate::sky::SkyMaterial;

const VS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sky/sky_vs.slang");
const FS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sky/sky_fs.slang");

const SUN_VS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sky/sun_vs.slang");
const SUN_FS_SHADER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sky/sun_fs.slang");

pub struct SkyPass {
    pipeline: Mutex<Pipeline>,
    sun_pipeline: Mutex<Pipeline>,
    state: Arc<Mutex<SkyState>>,
    sky_mesh: Mutex<Option<SkyMesh>>,
    sky_material: Mutex<Option<Buffer>>,
    sun_texture: Mutex<Option<Texture>>,
    sun_sampler: Sampler,
    sun_material: Mutex<Option<Buffer>>,
    sun_position: Mutex<Vec3>,
    clouds: Mutex<CloudsPass>,
}

impl SkyPass {
    pub fn new(state: Arc<Mutex<SkyState>>, queue: &CommandQueue<'_>) -> Self {
        Self {
            pipeline: Mutex::new(Pipeline::new(
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
                Box::new(|_queue, shaders| {
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
                            targets: &[ColorTargetState {
                                format: HDR_FORMAT,
                                blend: None,
                            }],
                            front_face: FrontFace::Ccw,
                            cull_mode: Some(Face::Back),
                            depth_stencil_state: None,
                            spec_consts: Default::default(),
                        }),
                    ]
                }),
                &[
                    PipelineBinding {
                        kind: DescriptorType::Uniform,
                        name: "constants",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Storage,
                        name: "positions",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Storage,
                        name: "colors",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Uniform,
                        name: "sky_colors",
                        stages: ShaderStages::FRAGMENT,
                    },
                ],
            )),
            sun_pipeline: Mutex::new(Pipeline::new(
                vec![
                    ShaderConfig {
                        source: ShaderSource::File(SUN_VS_SHADER.into()),
                        language: ShaderLanguage::Slang,
                    },
                    ShaderConfig {
                        source: ShaderSource::File(SUN_FS_SHADER.into()),
                        language: ShaderLanguage::Slang,
                    },
                ],
                Box::new(|_queue, shaders| {
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
                            targets: &[ColorTargetState {
                                format: HDR_FORMAT,
                                blend: Some(BlendState::NON_PREMULTIPLIED_ALPHA),
                            }],
                            front_face: FrontFace::Ccw,
                            cull_mode: None,
                            depth_stencil_state: None,
                            spec_consts: Default::default(),
                        }),
                    ]
                }),
                &[
                    PipelineBinding {
                        kind: DescriptorType::Uniform,
                        name: "constants",
                        stages: ShaderStages::VERTEX,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Texture,
                        name: "sun",
                        stages: ShaderStages::FRAGMENT,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Sampler,
                        name: "sampler",
                        stages: ShaderStages::FRAGMENT,
                    },
                    PipelineBinding {
                        kind: DescriptorType::Uniform,
                        name: "material",
                        stages: ShaderStages::FRAGMENT,
                    },
                ],
            )),
            state,
            sky_material: Mutex::new(None),
            sky_mesh: Mutex::new(None),
            sun_texture: Mutex::default(),
            sun_sampler: queue.create_sampler(&SamplerDescriptor {
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
            }),
            sun_material: Mutex::default(),
            sun_position: Mutex::default(),
            clouds: Mutex::new(CloudsPass::new(queue)),
        }
    }

    fn render_sun<'a>(
        &self,
        ctx: &'a RenderContext<'_, 'a>,
        static_camera: &Camera,
        target: &TextureView,
    ) {
        let Some(sun) = &*self.sun_texture.lock() else {
            return;
        };
        let Some(material) = &*self.sun_material.lock() else {
            return;
        };

        let mut pipeline = self.sun_pipeline.lock();

        let position = *self.sun_position.lock();
        let mut transform = Transform::from_translation(position).looking_at(Vec3::ZERO, Vec3::Y);
        transform.scale *= 0.5;

        let constants = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&Constants {
                camera_view_proj: static_camera.view_projection_matrix(),
                transform: transform.compute_matrix(),
            }),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = pipeline.prepare(ctx.queue);
        let sun_view = sun.create_view(&TextureViewDescriptor::default());
        pass.set("constants", ShaderStages::VERTEX, &constants);
        pass.set("sun", ShaderStages::FRAGMENT, &sun_view);
        pass.set("sampler", ShaderStages::FRAGMENT, &self.sun_sampler);
        pass.set("material", ShaderStages::FRAGMENT, material);

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Sun",
            color_attachments: &[RenderPassColorAttachment {
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
                target,
            }],
            depth_stencil_attachment: None,
        });

        pass.draw(0..6, 0..1);
    }
}

impl SkyPass {
    pub fn run<'a>(&self, ctx: &'a RenderContext<'_, 'a>, camera: &Camera, target: &Texture) {
        // Update state
        let new_sky_mesh = { self.state.lock().atomosphere.take() };
        let new_sky_material = { self.state.lock().sky_colors.take() };
        let new_sun_texture = { self.state.lock().sun.take() };
        let new_cloud_meshes = { self.state.lock().cloud_meshes.take() };
        let new_cloud_textures = { core::mem::take(&mut self.state.lock().cloud_textures) };
        let new_cloud_colors = { core::mem::take(&mut self.state.lock().cloud_colors) };

        if let Some(meshes) = new_cloud_meshes {
            self.clouds.lock().update_meshes(ctx.queue, &meshes);
        }

        for (layer, image) in new_cloud_textures {
            self.clouds
                .lock()
                .update_layer_texture(ctx.queue, layer, image.as_ref());
        }

        for (layer, color) in new_cloud_colors {
            self.clouds.lock().update_layer_color(layer, color);
        }

        if let Some(mesh) = new_sky_mesh {
            let positions = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(mesh.positions()),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });

            let colors = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(mesh.colors()),
                usage: BufferUsage::STORAGE,
                flags: UsageFlags::empty(),
            });

            let indices = ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::must_cast_slice(mesh.indicies().as_ref().unwrap().as_u32()),
                usage: BufferUsage::INDEX,
                flags: UsageFlags::empty(),
            });

            *self.sky_mesh.lock() = Some(SkyMesh {
                positions,
                colors,
                indices,
                num_indices: mesh.indicies().as_ref().unwrap().as_u32().len() as u32,
            });
        }

        if let Some(img) = new_sun_texture {
            let texture = ctx.queue.create_texture(&TextureDescriptor {
                size: UVec2::new(img.root().width(), img.root().height()),
                mip_levels: img.mip_levels(),
                format: img.format(),
                usage: TextureUsage::TRANSFER_DST | TextureUsage::TEXTURE_BINDING,
            });

            for (mip_level, mip) in img.mips().iter().enumerate() {
                ctx.queue.write_texture(
                    TextureRegion {
                        texture: &texture,
                        mip_level: mip_level as u32,
                    },
                    mip.as_bytes(),
                    ImageDataLayout {
                        format: img.format(),
                    },
                );
            }

            *self.sun_texture.lock() = Some(texture);
        }

        if let Some(material) = new_sky_material {
            *self.sky_material.lock() = Some(ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&RawSkyMaterial {
                    sky_lower: material.sky_lower.as_rgb(),
                    sky_scale: material.sky_scale,
                    sky_upper: material.sky_upper.as_rgb(),
                    _pad1: 0,
                    horizon: material.horizon.as_rgb(),
                    _pad2: 0,
                }),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            }));

            *self.sun_material.lock() = Some(ctx.queue.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::bytes_of(&RawSunMaterial {
                    color: material.sun.as_rgb(),
                    scale: material.sun_scale,
                }),
                usage: BufferUsage::UNIFORM,
                flags: UsageFlags::empty(),
            }));

            *self.sun_position.lock() = material.sun_position;
        }

        let (Some(mesh), Some(material)) = (&*self.sky_mesh.lock(), &*self.sky_material.lock())
        else {
            ctx.queue.clear_texture(
                TextureRegion {
                    texture: target,
                    mip_level: 0,
                },
                [0; 4],
            );

            return;
        };

        let mut pipeline = self.pipeline.lock();

        // Fix the camera translation at the origin so that the skybox
        // never moves relative to the camera.
        let mut static_camera = camera.clone();
        static_camera.transform.translation = Vec3::ZERO;

        let constants = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&Constants {
                camera_view_proj: static_camera.view_projection_matrix(),
                transform: Mat4::IDENTITY,
            }),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = pipeline.prepare(ctx.queue);

        pass.set("constants", ShaderStages::VERTEX, &constants);
        pass.set("positions", ShaderStages::VERTEX, &mesh.positions);
        pass.set("colors", ShaderStages::VERTEX, &mesh.colors);
        pass.set("sky_colors", ShaderStages::FRAGMENT, material);

        let target_view = target.create_view(&TextureViewDescriptor::default());
        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Sky",
            color_attachments: &[RenderPassColorAttachment {
                target: &target_view,
                load_op: LoadOp::Clear(Color::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        pass.set_index_buffer(&mesh.indices, IndexFormat::U32);
        pass.draw_indexed(0..mesh.num_indices, 0, 0..1);

        drop(pass);

        self.render_sun(ctx, &static_camera, &target_view);

        self.clouds
            .lock()
            .render(ctx.queue, &static_camera, &target_view);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Constants {
    camera_view_proj: Mat4,
    transform: Mat4,
}

#[derive(Debug, Default)]
pub struct SkyState {
    pub atomosphere: Option<Mesh>,
    pub sky_colors: Option<SkyMaterial>,
    pub sun: Option<MipImage>,
    pub cloud_meshes: Option<Vec<Mesh>>,
    pub cloud_textures: Vec<(usize, Option<MipImage>)>,
    pub cloud_colors: Vec<(usize, Color)>,
}

struct SkyMesh {
    positions: Buffer,
    colors: Buffer,
    indices: Buffer,
    num_indices: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawSkyMaterial {
    sky_upper: [f32; 3],
    sky_scale: f32,
    sky_lower: [f32; 3],
    _pad1: u32,
    horizon: [f32; 3],
    _pad2: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct SunConstants {
    camera_view_proj: Mat4,
    transform: Mat4,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawSunMaterial {
    color: [f32; 3],
    scale: f32,
}
