use bytemuck::{Pod, Zeroable};
use game_common::components::Color;
use game_render::api::{
    BindingResource, Buffer, BufferInitDescriptor, CommandQueue, FragmentStage, PipelineStage,
    RenderPassColorAttachment, RenderPassDescriptor, Sampler, TextureRegion, TextureView,
    TextureViewDescriptor, VertexStage,
};
use game_render::backend::allocator::UsageFlags;
use game_render::backend::{
    AddressMode, BlendState, BufferUsage, ColorTargetState, DescriptorType,
    DrawIndexedIndirectCommand, Face, FilterMode, FrontFace, ImageDataLayout, IndexFormat, LoadOp,
    PrimitiveTopology, SamplerDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureFormat,
    TextureUsage,
};
use game_render::buffer::texture_slab::{TextureSlab, TextureSlabIndex};
use game_render::mesh::Mesh;
use game_render::texture::image::MipImage;
use glam::{Mat4, UVec2, Vec2};
use openfo4_shaderlib::{ShaderSource, include_shader};

use crate::camera::Camera;
use crate::passes::HDR_FORMAT;
use crate::passes::pipeline::{Pipeline, PipelineBinding};

static VS_SHADER: ShaderSource = include_shader!("/game_core_pipeline/shaders/sky/clouds_vs.slang");
static FS_SHADER: ShaderSource = include_shader!("/game_core_pipeline/shaders/sky/clouds_fs.slang");

pub struct CloudsPass {
    pipeline: Pipeline,
    meshes: Option<MeshData>,
    textures: TextureSlab,
    layers: Vec<CloudLayer>,
    sampler: Sampler,
}

impl CloudsPass {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        let pipeline = Pipeline::new(
            [&VS_SHADER, &FS_SHADER],
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
                    kind: DescriptorType::Storage,
                    name: "uvs",
                    stages: ShaderStages::VERTEX,
                },
                PipelineBinding {
                    kind: DescriptorType::Storage,
                    name: "offsets",
                    stages: ShaderStages::VERTEX,
                },
                PipelineBinding {
                    kind: DescriptorType::Storage,
                    name: "layers",
                    stages: ShaderStages::FRAGMENT,
                },
                PipelineBinding {
                    kind: DescriptorType::Texture,
                    name: "textures",
                    stages: ShaderStages::FRAGMENT,
                },
                PipelineBinding {
                    kind: DescriptorType::Sampler,
                    name: "sampler",
                    stages: ShaderStages::FRAGMENT,
                },
            ],
        );

        let default_texture = queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::TRANSFER_DST,
        });
        queue.write_texture(
            TextureRegion {
                texture: &default_texture,
                mip_level: 0,
            },
            &[0u8; 4],
            ImageDataLayout {
                format: default_texture.format(),
            },
        );

        let textures =
            TextureSlab::new(default_texture.create_view(&TextureViewDescriptor::default()));

        let sampler = queue.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            compare: None,
        });

        Self {
            pipeline,
            meshes: None,
            textures,
            layers: Vec::new(),
            sampler,
        }
    }

    pub fn update_meshes(&mut self, queue: &CommandQueue<'_>, meshes: &[Mesh]) {
        let mut positions = Vec::<f32>::new();
        let mut colors = Vec::<f32>::new();
        let mut uvs = Vec::<f32>::new();
        let mut indices = Vec::<u32>::new();

        let mut offsets = Vec::new();
        let mut indirect = Vec::new();

        for mesh in meshes {
            assert_eq!(mesh.positions().len(), mesh.colors().len());
            let mesh_indices = mesh.indicies().unwrap().as_u32();

            offsets.push(CloudMeshData {
                positions_offset: positions.len() as u32,
                colors_offset: colors.len() as u32,
                uvs_offset: uvs.len() as u32,
            });
            indirect.push(DrawIndexedIndirectCommand {
                index_count: mesh_indices.len() as u32,
                first_index: indices.len() as u32,
                vertex_offset: 0,
                instance_count: 1,
                first_instance: 0,
            });

            positions.extend_from_slice(bytemuck::must_cast_slice(mesh.positions()));
            colors.extend_from_slice(bytemuck::must_cast_slice(mesh.colors()));
            uvs.extend_from_slice(bytemuck::must_cast_slice(mesh.uvs()));
            indices.extend_from_slice(mesh_indices);
        }

        let positions = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&positions),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });
        let colors = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&colors),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });
        let uvs = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&uvs),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });
        let offsets = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&offsets),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });
        let indices = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsage::INDEX,
            flags: UsageFlags::empty(),
        });
        let indirect = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&indirect),
            usage: BufferUsage::INDIRECT,
            flags: UsageFlags::empty(),
        });

        self.meshes = Some(MeshData {
            positions,
            colors,
            indices,
            offsets,
            indirect,
            uvs,
        });

        self.layers.resize_with(meshes.len(), || CloudLayer {
            color: [0.0; 4],
            uv_offset: [0.0; 2],
            texture: TextureSlabIndex(u32::MAX),
            _pad: [0; 1],
        });
    }

    pub fn update_layer_texture(
        &mut self,
        queue: &CommandQueue<'_>,
        layer: usize,
        image: Option<&MipImage>,
    ) {
        let Some(layer) = self.layers.get_mut(layer) else {
            return;
        };

        if layer.texture.0 != u32::MAX {
            self.textures.remove(layer.texture);
            layer.texture.0 = u32::MAX;
        }

        let Some(image) = image else {
            return;
        };

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

        let view = texture.create_view(&TextureViewDescriptor::default());
        let index = self.textures.insert(view);

        layer.texture = index;
    }

    pub fn update_layer_color(&mut self, layer: usize, color: Color) {
        let Some(layer) = self.layers.get_mut(layer) else {
            return;
        };

        layer.color = color.as_rgba();
    }

    pub fn update_layer_uv_offset(&mut self, layer: usize, offset: Vec2) {
        let Some(layer) = self.layers.get_mut(layer) else {
            return;
        };

        layer.uv_offset = offset.to_array();
    }

    pub fn render(&mut self, queue: &CommandQueue<'_>, camera: &Camera, target: &TextureView) {
        let Some(meshes) = &self.meshes else {
            return;
        };

        let constants = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::bytes_of(&Constants {
                camera_view_proj: camera.view_projection_matrix(),
                transform: Mat4::IDENTITY,
            }),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let mut pass = self.pipeline.prepare(queue);
        pass.set("constants", ShaderStages::VERTEX, &constants);
        pass.set("positions", ShaderStages::VERTEX, &meshes.positions);
        pass.set("colors", ShaderStages::VERTEX, &meshes.colors);
        pass.set("uvs", ShaderStages::VERTEX, &meshes.uvs);
        pass.set("offsets", ShaderStages::VERTEX, &meshes.offsets);

        let textures = self.textures.views();

        let layers = queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::must_cast_slice(&self.layers),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        pass.set("layers", ShaderStages::FRAGMENT, &layers);
        pass.set("sampler", ShaderStages::FRAGMENT, &self.sampler);
        pass.set(
            "textures",
            ShaderStages::FRAGMENT,
            BindingResource::TextureArray(&textures),
        );

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Clouds",
            color_attachments: &[RenderPassColorAttachment {
                target,
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        pass.set_index_buffer(&meshes.indices, IndexFormat::U32);
        pass.draw_indexed_indirect(&meshes.indirect);
    }
}

#[derive(Debug)]
struct MeshData {
    positions: Buffer,
    colors: Buffer,
    uvs: Buffer,
    indices: Buffer,
    offsets: Buffer,
    indirect: Buffer,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct CloudMeshData {
    positions_offset: u32,
    colors_offset: u32,
    uvs_offset: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Constants {
    camera_view_proj: Mat4,
    transform: Mat4,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct CloudLayer {
    color: [f32; 4],
    uv_offset: [f32; 2],
    texture: TextureSlabIndex,
    _pad: [u32; 1],
}
