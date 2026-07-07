mod mesh;
mod opaque_vertex;
mod pipeline;
mod post_process;
mod skinning;
pub mod sky;
mod transparent_vertex;
mod update;

use std::collections::HashMap;
use std::sync::Arc;

use bitflags::bitflags;
use bytemuck::{NoUninit, Pod, Zeroable};
use crossbeam::channel::Receiver;
use game_render::api::{CommandQueue, TextureRegion, TextureViewDescriptor};
use game_render::backend::{
    BufferUsage, ImageDataLayout, TextureDescriptor, TextureFormat, TextureUsage,
};
use game_render::buffer::GpuBuffer;
use game_render::buffer::slab::{CompactSlabBuffer, SlabBuffer, SlabIndex};
use game_render::buffer::suballocated::SubAllocatedGrowableBuffer;
use game_render::buffer::texture_slab::{TextureSlab, TextureSlabIndex};
use game_render::graph::{NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};
use glam::{UVec2, Vec3};
use parking_lot::Mutex;
use post_process::PostProcessPass;
use update::{TransformUniform, UpdatePass};

use crate::camera::Camera;
use crate::entities::{CameraId, Event, ImageId, LightId, MaterialId, MeshId, ObjectId, SceneId};
use crate::lights::{AmbientLight, DirectionalLightUniform, PointLightUniform, SpotLightUniform};
use crate::material::AlphaMode;
use crate::passes::mesh::{InstanceKey, MeshKey, MeshState};
use crate::passes::skinning::SkinningPass;
use crate::passes::sky::SkyState;
use crate::passes::transparent_vertex::TransparentVertexPass;
use crate::passes::update::RawTerrainLayer;

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

const TRANSPARENT_VERTEX_PASS: NodeLabel = NodeLabel::new("TRANSPARENT_VERTEX_PASS");
const UPDATE_PASS: NodeLabel = NodeLabel::new("UPDATE_PASS");
const FORWARD_PASS: NodeLabel = NodeLabel::new("FORWARD_PASS");
const POST_PROCESS_PASS: NodeLabel = NodeLabel::new("POST_PROCESS_PASS");
const SKINNING_PASS: NodeLabel = NodeLabel::new("SKINNING_PASS");

pub const FINAL_RENDER_PASS: NodeLabel = NodeLabel::new("FINAL_RENDER_PASS");

const OPAQUE_DEPTH: SlotLabel = SlotLabel::new("OPAQUE_DEPTH");
const HDR_TEXTURE: SlotLabel = SlotLabel::new("HDR_TEXTURE");

pub(crate) fn init(
    graph: &mut RenderGraph,
    queue: &CommandQueue<'_>,
    events: Receiver<Event>,
    sky: Arc<Mutex<SkyState>>,
) {
    let state = Arc::new(Mutex::new(State::new(queue, sky)));

    let update_pass = UpdatePass::new(queue, state.clone(), events);
    graph.add_node(UPDATE_PASS, update_pass);

    let skinning_pass = SkinningPass::new(state.clone());
    graph.add_node(SKINNING_PASS, skinning_pass);
    graph.add_node_dependency(SKINNING_PASS, UPDATE_PASS);

    opaque_vertex::init(graph, queue, state.clone());

    graph.add_node(
        TRANSPARENT_VERTEX_PASS,
        TransparentVertexPass::new(state.clone(), queue),
    );
    graph.add_slot_dependency(
        TRANSPARENT_VERTEX_PASS,
        OPAQUE_DEPTH,
        SlotKind::Texture,
        SlotFlags::READ,
    );
    graph.add_slot_dependency(
        TRANSPARENT_VERTEX_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::READ | SlotFlags::WRITE,
    );

    let post_process_pass = PostProcessPass::new(queue, HDR_TEXTURE, SlotLabel::SURFACE);
    graph.add_node(POST_PROCESS_PASS, post_process_pass);
    graph.add_node_dependency(POST_PROCESS_PASS, TRANSPARENT_VERTEX_PASS);
    graph.add_slot_dependency(
        POST_PROCESS_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::READ,
    );
    graph.add_slot_dependency(
        POST_PROCESS_PASS,
        SlotLabel::SURFACE,
        SlotKind::Texture,
        SlotFlags::READ | SlotFlags::WRITE,
    );
}

#[derive(Debug)]
struct State {
    mesh: MeshState,
    default_textures: DefaultTextures,

    meshes: HashMap<MeshId, MeshKey>,
    images: HashMap<ImageId, TextureSlabIndex>,
    materials: HashMap<MaterialId, MaterialState>,

    scenes: HashMap<SceneId, SceneData>,

    textures: TextureSlab,
    material_slab: SlabBuffer<RawMaterialData>,

    sky: Arc<Mutex<SkyState>>,
    terrain_layers: SubAllocatedGrowableBuffer<RawTerrainLayer>,
    terrain_material: HashMap<MaterialId, TerrainMaterialState>,
}

impl State {
    fn new(queue: &CommandQueue<'_>, sky: Arc<Mutex<SkyState>>) -> Self {
        let placeholder_texture = queue.create_texture(&TextureDescriptor {
            size: UVec2::ONE,
            mip_levels: 1,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::TEXTURE_BINDING,
        });

        let mut textures =
            TextureSlab::new(placeholder_texture.create_view(&TextureViewDescriptor::default()));

        let default_textures = DefaultTextures::new(queue, &mut textures);

        Self {
            default_textures,
            meshes: HashMap::new(),
            materials: HashMap::new(),
            scenes: HashMap::new(),
            images: HashMap::new(),
            textures,
            material_slab: SlabBuffer::new(BufferUsage::STORAGE),
            mesh: MeshState::new(queue),
            sky,
            terrain_layers: SubAllocatedGrowableBuffer::new(queue, BufferUsage::STORAGE),
            terrain_material: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct DefaultTextures {
    base_color: TextureSlabIndex,
    normal: TextureSlabIndex,
    metallic_roughness: TextureSlabIndex,
    specular_glossiness: TextureSlabIndex,
}

impl DefaultTextures {
    fn new(queue: &CommandQueue<'_>, textures: &mut TextureSlab) -> Self {
        let [base_color, normal, metallic_roughness, specular_glossiness] = [
            // Base color: white
            (TextureFormat::Rgba8UnormSrgb, [255, 255, 255, 255]),
            // Normal: B channel facing towards local Z.
            (TextureFormat::Rgba8Unorm, [127, 127, 255, 255]),
            // Roughness (channel G): Factor 1.0.
            // Meallic (channel B): Factor 1.0.
            // Other channels are ignored.
            (TextureFormat::Rgba8UnormSrgb, [0, 255, 255, 0]),
            // Specular (channel R): Factor 1.0.
            // Glossiness (channel G): 0.0, results in roughness of 1.0.
            // Other channels are ignored.
            (TextureFormat::Rgba8UnormSrgb, [255, 0, 0, 0]),
        ]
        .map(|(format, data)| {
            let texture = queue.create_texture(&TextureDescriptor {
                size: UVec2::ONE,
                mip_levels: 1,
                format,
                usage: TextureUsage::TRANSFER_DST | TextureUsage::TEXTURE_BINDING,
            });

            queue.write_texture(
                TextureRegion {
                    texture: &texture,
                    mip_level: 0,
                },
                &data,
                ImageDataLayout { format },
            );

            textures.insert(texture.create_view(&TextureViewDescriptor::default()))
        });

        Self {
            base_color,
            normal,
            metallic_roughness,
            specular_glossiness,
        }
    }
}

#[derive(Debug)]
struct SceneData {
    cameras: HashMap<CameraId, Camera>,
    objects: HashMap<ObjectId, ObjectState>,
    directional_lights: HashMap<LightId, DirectionalLightState>,
    point_lights: HashMap<LightId, SlabIndex>,
    spot_lights: HashMap<LightId, SlabIndex>,
    ambient_lights: HashMap<LightId, AmbientLight>,

    directional_lights_buffer: CompactSlabBuffer<DirectionalLightUniform>,
    point_lights_buffer: CompactSlabBuffer<PointLightUniform>,
    spot_lights_buffer: CompactSlabBuffer<SpotLightUniform>,
}

#[derive(Copy, Clone, Debug)]
struct DirectionalLightState {
    index: SlabIndex,
    /// Facing direction
    direction: Vec3,
}

impl SceneData {
    fn new() -> Self {
        Self {
            cameras: HashMap::new(),
            objects: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
            spot_lights: HashMap::new(),
            ambient_lights: HashMap::new(),
            directional_lights_buffer: CompactSlabBuffer::new(BufferUsage::STORAGE),
            point_lights_buffer: CompactSlabBuffer::new(BufferUsage::STORAGE),
            spot_lights_buffer: CompactSlabBuffer::new(BufferUsage::STORAGE),
        }
    }
}

#[derive(Debug)]
struct ObjectState {
    id: InstanceKey,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawMaterialData {
    flags: MaterialFlags,
    alpha_mask: f32,
    depth_bias: f32,
    _pad0: u32,
    base_color: [f32; 4],
    roughness: f32,
    metallic: f32,
    reflectance: f32,
    specular_strength: f32,
    specular_color: [f32; 4],
    base_color_texture_index: TextureSlabIndex,
    normal_texture_index: TextureSlabIndex,
    metallic_roughness_texture_index: TextureSlabIndex,
    specular_glossiness_texture_index: TextureSlabIndex,
    grayscale_palette_texture_index: TextureSlabIndex,
    grayscale_palette_scale: f32,
    uv_offset: [f32; 2],
    uv_scale: [f32; 2],
    terrain_layers_index: u32,
    terrain_layers_count: u32,
}

impl GpuBuffer for RawMaterialData {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    #[repr(transparent)]
    struct MaterialFlags: u32 {
        const UNLIT = 1 << 0;
        const FLIP_NORMAL_Y = 1 << 1;
        const METALLIC_FROM_SPECULAR = 1 << 2;
        const ROUGHNESS_FROM_GLOSSINESS = 1 << 3;
        const NORMAL_ENCODING_TWO_CHANNEL = 1 << 4;
        const GRAYSCALE_PALETTE = 1 << 5;
        const TERRAIN = 1 << 6;
    }
}

#[derive(Copy, Clone, Debug, NoUninit)]
#[repr(C)]
struct RawObjectData {
    pub transform: TransformUniform,
    pub meshlet_offset: u32,
    pub material: SlabIndex,
    _pad0: [u32; 2],
}

#[derive(Copy, Clone, Debug)]
pub struct MaterialState {
    pub index: SlabIndex,
    pub alpha_mode: AlphaMode,
}

#[derive(Clone, Debug)]
pub struct TerrainMaterialState {
    pub materials: Vec<SlabIndex>,
    pub textures: Vec<TextureSlabIndex>,
    pub layer_index: u64,
}
