use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use game_common::color::LinearRgb;
use game_common::components::{Color, Transform};
use game_render::buffer::GpuBuffer;
use glam::Vec3;

use crate::entities::SceneHandle;

#[derive(Clone, Debug)]
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
    Ambient(AmbientLight),
}

#[derive(Clone, Debug)]
pub struct DirectionalLight {
    pub transform: Transform,
    pub scene: SceneHandle,
    pub color: LinearRgb,
    /// The illuminance of the light in lux.
    pub illuminance: f32,
}

#[derive(Clone, Debug)]
pub struct PointLight {
    pub transform: Transform,
    pub scene: SceneHandle,
    pub color: Color,
    /// The luminious power of the light in lumen.
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct SpotLight {
    pub transform: Transform,
    pub scene: SceneHandle,
    pub color: Color,
    /// The luminious power of the light in lumen.
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

#[derive(Clone, Debug)]
pub struct AmbientLight {
    pub scene: SceneHandle,
    pub top: LinearRgb,
    pub bottom: LinearRgb,
    pub left: LinearRgb,
    pub right: LinearRgb,
    pub front: LinearRgb,
    pub back: LinearRgb,
    pub brightness: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct DirectionalLightUniform {
    pub direction: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub illuminance: f32,
}

impl DirectionalLightUniform {
    pub(crate) fn new(entity: &DirectionalLight) -> Self {
        Self {
            direction: (entity.transform.rotation * -Vec3::Z).to_array(),
            color: entity.color.as_rgb(),
            illuminance: entity.illuminance,
            _pad0: 0,
        }
    }
}

impl GpuBuffer for DirectionalLightUniform {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct PointLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub _pad1: [u32; 3],
}

impl PointLightUniform {
    pub(crate) fn new(entity: &PointLight) -> Self {
        Self {
            position: entity.transform.translation.to_array(),
            color: entity.color.as_rgb(),
            intensity: lumen_to_candela(entity.intensity),
            radius: entity.radius,
            _pad0: 0,
            _pad1: [0; 3],
        }
    }
}

impl GpuBuffer for PointLightUniform {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct SpotLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub direction: [f32; 3],
    pub _pad1: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
    pub _pad2: [u32; 1],
}

impl SpotLightUniform {
    pub(crate) fn new(entity: &SpotLight) -> Self {
        Self {
            position: entity.transform.translation.to_array(),
            direction: (entity.transform.rotation * -Vec3::Z).to_array(),
            color: entity.color.as_rgb(),
            intensity: lumen_to_candela(entity.intensity),
            radius: entity.radius,
            inner_cutoff: entity.inner_cutoff,
            outer_cutoff: entity.outer_cutoff,
            _pad0: 0,
            _pad1: 0,
            _pad2: [0; 1],
        }
    }
}

impl GpuBuffer for SpotLightUniform {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

fn lumen_to_candela(lm: f32) -> f32 {
    // https://google.github.io/filament/main/filament.html#mjx-eqn-pointLightLuminousPower
    lm / (4.0 * PI)
}
