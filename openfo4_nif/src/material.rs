use std::fmt::DebugList;

use glam::Vec2;

use crate::{Block, BlockRef, Decode, Error, Reader, Rgb, Rgba, TextureClampMode};

#[derive(Clone, Debug)]
pub struct BSLightingShaderProperty {
    pub shader_type: u32,
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub shader_flags1: u32,
    pub shader_flags2: u32,
    pub uv_offset: Vec2,
    pub uv_scale: Vec2,
    pub texture_set: Option<BlockRef>,
    pub emissive_color: Rgb,
    pub emissive_mult: f32,
    pub root_material: String,
    pub texture_clamp_mode: TextureClampMode,
    pub alpha: f32,
    pub refraction_strength: f32,
    pub smoothness: f32,
    pub specular_color: Rgb,
    pub specular_strength: f32,
    pub subsurface_rolloff: f32,
    pub rimlight_power: f32,
    pub backlight_power: f32,
    pub grayscale_to_palette_scale: f32,
    pub fresnel_power: f32,
    pub wetness_params: WetnessParams,
    pub environment_map_scale: f32,
    pub use_screen_space_reflections: bool,
    pub wetness_control_use_ssr: bool,
}

impl Decode for BSLightingShaderProperty {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let shader_type = reader.get_u32()?;

        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let extra_data_len = reader.get_u32()?;
        let extra_data = (0..extra_data_len)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        let controller = <Option<BlockRef>>::decode(reader)?;

        let shader_flags1 = reader.get_u32()?;
        let shader_flags2 = reader.get_u32()?;

        let uv_offset = Vec2::decode(reader)?;
        let uv_scale = Vec2::decode(reader)?;

        let texture_set = <Option<BlockRef>>::decode(reader)?;

        let emissive_color = Rgb::decode(reader)?;
        let emissive_mult = reader.get_f32()?;

        let root_material = reader.get_u32()?;
        let root_material = reader.string(root_material)?.to_owned();

        let texture_clamp_mode = TextureClampMode::from_bits_truncate(reader.get_u32()?);
        let alpha = reader.get_f32()?;
        let refraction_strength = reader.get_f32()?;
        let smoothness = reader.get_f32()?;
        let specular_color = Rgb::decode(reader)?;
        let specular_strength = reader.get_f32()?;
        let subsurface_rolloff = reader.get_f32()?;
        let rimlight_power = reader.get_f32()?;
        let backlight_power = reader.get_f32()?;
        let grayscale_to_palette_scale = reader.get_f32()?;
        let fresnel_power = reader.get_f32()?;

        let wetness_params = WetnessParams::decode(reader)?;

        let environment_map_scale = reader.get_f32()?;
        let use_screen_space_reflections = reader.get_bool()?;
        let wetness_control_use_ssr = reader.get_bool()?;

        Ok(Self {
            shader_type,
            name,
            extra_data,
            controller,
            shader_flags1,
            shader_flags2,
            uv_offset,
            uv_scale,
            texture_set,
            emissive_color,
            emissive_mult,
            root_material,
            texture_clamp_mode,
            alpha,
            refraction_strength,
            smoothness,
            specular_color,
            specular_strength,
            subsurface_rolloff,
            rimlight_power,
            backlight_power,
            grayscale_to_palette_scale,
            fresnel_power,
            wetness_params,
            environment_map_scale,
            use_screen_space_reflections,
            wetness_control_use_ssr,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WetnessParams {
    pub spec_scale: f32,
    pub spec_power: f32,
    pub min_var: f32,
    pub env_map_scale: f32,
    pub fresnel_power: f32,
    pub metalness: f32,
}

impl Decode for WetnessParams {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let spec_scale = reader.get_f32()?;
        let spec_power = reader.get_f32()?;
        let min_var = reader.get_f32()?;
        let env_map_scale = reader.get_f32()?;
        let fresnel_power = reader.get_f32()?;
        let metalness = reader.get_f32()?;

        Ok(Self {
            spec_scale,
            spec_power,
            min_var,
            env_map_scale,
            fresnel_power,
            metalness,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BSShaderTextureSet {
    pub textures: Vec<String>,
}

impl Decode for BSShaderTextureSet {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let textures_len = reader.get_u32()?;
        let textures = (0..textures_len)
            .map(|_| String::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self { textures })
    }
}

#[derive(Clone, Debug)]
pub struct BSEffectShaderProperty {
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub shader_flags1: u32,
    pub shader_flags2: u32,
    pub uv_offset: Vec2,
    pub uv_scale: Vec2,
    pub source_texture: String,
    pub texture_clamp_mode: u8,
    pub lighting_influence: u8,
    pub env_map_min_lod: u8,
    pub falloff_start_angle: f32,
    pub falloff_stop_angle: f32,
    pub falloff_start_opacity: f32,
    pub falloff_stop_opacity: f32,
    pub base_color: Rgba<f32>,
    pub base_color_scale: f32,
    pub soft_falloff_depth: f32,
    pub grayscale_texture: String,
    pub env_map_texture: String,
    pub normal_texture: String,
    pub env_mask_texture: String,
    pub env_map_scale: f32,
}

impl Decode for BSEffectShaderProperty {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let extra_data_len = reader.get_u32()?;
        let extra_data = (0..extra_data_len)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        let controller = <Option<BlockRef>>::decode(reader)?;
        let shader_flags1 = reader.get_u32()?;
        let shader_flags2 = reader.get_u32()?;
        let uv_offset = Vec2::decode(reader)?;
        let uv_scale = Vec2::decode(reader)?;
        let source_texture = String::decode(reader)?;
        let texture_clamp_mode = reader.get_u8()?;
        let lighting_influence = reader.get_u8()?;
        let env_map_min_lod = reader.get_u8()?;
        reader.get_u8()?;
        let falloff_start_angle = reader.get_f32()?;
        let falloff_stop_angle = reader.get_f32()?;
        let falloff_start_opacity = reader.get_f32()?;
        let falloff_stop_opacity = reader.get_f32()?;
        let base_color = Rgba::<f32>::decode(reader)?;
        let base_color_scale = reader.get_f32()?;
        let soft_falloff_depth = reader.get_f32()?;
        let grayscale_texture = String::decode(reader)?;
        let env_map_texture = String::decode(reader)?;
        let normal_texture = String::decode(reader)?;
        let env_mask_texture = String::decode(reader)?;
        let env_map_scale = reader.get_f32()?;

        Ok(Self {
            name,
            extra_data,
            controller,
            shader_flags1,
            shader_flags2,
            uv_offset,
            uv_scale,
            source_texture,
            texture_clamp_mode,
            lighting_influence,
            env_map_min_lod,
            falloff_start_angle,
            falloff_stop_angle,
            falloff_start_opacity,
            falloff_stop_opacity,
            base_color,
            base_color_scale,
            soft_falloff_depth,
            grayscale_texture,
            env_map_texture,
            normal_texture,
            env_mask_texture,
            env_map_scale,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BSSkyShaderProperty {
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub shader_flags1: u32,
    pub shader_flags2: u32,
    pub uv_offset: Vec2,
    pub uv_scale: Vec2,
    pub source_texture: String,
    pub sky_object_type: (), // What type?
}

impl Decode for BSSkyShaderProperty {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let extra_data_len = reader.get_u32()?;
        let extra_data = (0..extra_data_len)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        let controller = <Option<BlockRef>>::decode(reader)?;
        let shader_flags1 = reader.get_u32()?;
        let shader_flags2 = reader.get_u32()?;
        let uv_offset = Vec2::decode(reader)?;
        let uv_scale = Vec2::decode(reader)?;
        let source_texture = String::decode(reader)?;

        Ok(Self {
            name,
            extra_data,
            controller,
            shader_flags1,
            shader_flags2,
            uv_offset,
            uv_scale,
            source_texture,
            sky_object_type: (),
        })
    }
}
