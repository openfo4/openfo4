use game_render::api::{Buffer, BufferSlice, Sampler, Texture, TextureView};
use game_render::backend::{ClearValue, LoadOp};

use crate::camera::CameraUniform;
use crate::passes::mesh::MeshState;

pub mod drawgen;
pub mod opaque;
pub mod transparent;

pub struct ForwardParams<'a> {
    pub mesh: &'a mut MeshState,
    pub visbuffer: &'a TextureView,
    pub depth: &'a Texture,
    pub materials: &'a Buffer,
    pub camera: CameraUniform,
    pub textures: &'a [&'a TextureView],
    pub sampler: &'a Sampler,
    pub visbuffer_load_op: LoadOp<ClearValue>,
    pub depth_load_op: LoadOp<f32>,
    pub instances_out: BufferSlice<'a>,
    pub base_out_instance: u32,
}
