use bitflags::bitflags;
use glam::Vec2;

use crate::{BlockRef, Decode, Error, Reader, Rgb, Rgba, TextureClampMode};

#[derive(Clone, Debug)]
pub struct BSLightingShaderProperty {
    pub shader_type: u32,
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub shader_flags1: ShaderFlags1,
    pub shader_flags2: ShaderFlags2,
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

        let shader_flags1 = ShaderFlags1::from_bits_truncate(reader.get_u32()?);
        let shader_flags2 = ShaderFlags2::from_bits_truncate(reader.get_u32()?);

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

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ShaderFlags1: u32 {
        const SPECULAR = 1 << 0;
        const SKINNED = 1 << 1;
        const TEMP_REFRACTION = 1 << 2;
        const VERTEX_ALPHA = 1 << 3;
        const GRAYSCALE_TO_PALETTE_COLOR = 1 << 4;
        const GRAYSCALE_TO_PALETTE_SCALE = 1 << 5;
        const USE_FALLOFF = 1 << 6;
        const ENVIRONMENT_MAPPING = 1 << 7;
        const RGB_FALLOFF = 1 << 8;
        const CAST_SHADOWS = 1 << 9;
        const FACE = 1 << 10;
        const UI_MASK_RECTS = 1 << 11;
        const MODEL_SPACE_NORMALS = 1 << 12;
        const NON_PROJECTIVE_SHADOWS = 1 << 13;
        const LANDSCAPE = 1 << 14;
        const REFRACTION = 1 << 15;
        const FIRE_REFRACTION = 1 << 16;
        const EYE_ENVIRONMENT_MAPPING = 1 << 17;
        const HAIR = 1 << 18;
        const SCREENDOOR_ALPHA_FADE = 1 << 19;
        const LOCALMAP_HIDE_SECRET = 1 << 20;
        const SKIN_TINT = 1 << 21;
        const OWN_EMIT = 1 << 22;
        const PROJECTED_UV = 1 << 23;
        const MULTIPLE_TEXTURES = 1 << 24;
        const TESSELATE = 1 << 25;
        const DECAL = 1 << 26;
        const DYNAMIC_DECAL = 1 << 27;
        const CHARACTER_LIGHTING = 1 << 28;
        const EXTERNAL_EMITTANCE = 1 << 29;
        const SOFT_EFFECT = 1 << 30;
        const ZBUFFER_TEST = 1 << 31;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ShaderFlags2: u32 {
        const ZBUFFER_WRITE = 1 << 0;
        const LOD_LANDSCAPE = 1 << 1;
        const LOD_OBJECTS = 1 << 2;
        const NO_FADE = 1 << 3;
        const DOUBLE_SIDED = 1 << 4;
        const VERTEX_COLOR = 1 << 5;
        const GLOW_MAP = 1 << 6;
        const TRANSFORM_CHANGED = 1 << 7;
        const DISMEMBERMENT_MEATCUFF = 1 << 8;
        const TINT = 1 << 9;
        const GRASS_VERTEX_LIGHTING = 1 << 10;
        const GRASS_UNIFORM_SCALE = 1 << 11;
        const GRASS_FIT_SLOPE = 1 << 12;
        const GRASS_BILLBOARD = 1 << 13;
        const NO_LOD_LAND_BLEND = 1 << 14;
        const DISMEMBERMENT = 1 << 15;
        const WIREFRAME = 1 << 16;
        const WEAPON_BLOOD = 1 << 17;
        const HIDE_ON_LOCAL_MAP = 1 << 18;
        const PREMULT_ALPHA = 1 << 19;
        const VATS_TARGET = 1 << 20;
        const ANISOTROPIC_LIGHTING = 1 << 21;
        const SKEW_SPECULAR_ALPHA = 1 << 22;
        const MENU_SCREEN = 1 << 23;
        const MULTI_LAYER_PARALLAX = 1 << 24;
        const ALPHA_TEST = 1 << 25;
        const GRADIENT_REMAP = 1 << 26;
        const VATS_TARGET_DRAW_ALL = 1 << 27;
        const PIPBOY_SCREEN = 1 << 28;
        const TREE_ANIM = 1 << 29;
        const EFFECT_LIGHTING = 1 << 30;
        const REFRACTION_WRITES_DEPTH = 1 << 31;
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

#[derive(Clone, Debug)]
pub struct NiAlphaProperty {
    pub name: String,
    pub extra_data: Vec<BlockRef>,
    pub controller: Option<BlockRef>,
    pub alpha_blend: Option<AlphaBlendState>,
    pub alpha_test: Option<AlphaTestState>,
}

impl Decode for NiAlphaProperty {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let extra_data_len = reader.get_u32()?;
        let extra_data = (0..extra_data_len)
            .map(|_| BlockRef::decode(reader))
            .collect::<Result<_, _>>()?;

        let controller = <Option<BlockRef>>::decode(reader)?;
        let flags = u16::decode(reader)?;
        let threshold = u8::decode(reader)?;

        let enable_blending = (flags & 1) != 0;
        let alpha_blend = if enable_blending {
            let src_mode = BlendMode::from_bits(((flags >> 1) & 0b1111) as u8)?;
            let dst_mode = BlendMode::from_bits(((flags >> 5) & 0b1111) as u8)?;
            Some(AlphaBlendState { src_mode, dst_mode })
        } else {
            None
        };

        let enable_alpha_test = (flags & (1 << 9)) != 0;
        let alpha_test = if enable_alpha_test {
            let function = AlphaTestFunction::from_bits(((flags >> 10) & 0b111) as u8)?;
            Some(AlphaTestState {
                function,
                threshold,
            })
        } else {
            None
        };

        Ok(Self {
            name,
            extra_data,
            controller,
            alpha_blend,
            alpha_test,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AlphaBlendState {
    pub src_mode: BlendMode,
    pub dst_mode: BlendMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    One,
    Zero,
    SrcColor,
    InvSrcColor,
    DstColor,
    InvDstColor,
    SrcAlpha,
    InvSrcAlpha,
    DstAlpha,
    InvDstAlpha,
    SrcAlphaSaturate,
}

impl BlendMode {
    fn from_bits(bits: u8) -> Result<Self, Error> {
        match bits {
            0 => Ok(Self::One),
            1 => Ok(Self::Zero),
            2 => Ok(Self::SrcColor),
            3 => Ok(Self::InvSrcColor),
            4 => Ok(Self::DstColor),
            5 => Ok(Self::InvDstColor),
            6 => Ok(Self::SrcAlpha),
            7 => Ok(Self::InvSrcAlpha),
            8 => Ok(Self::DstAlpha),
            9 => Ok(Self::InvDstAlpha),
            10 => Ok(Self::SrcAlphaSaturate),
            _ => Err(Error::InvalidBlendMode(bits)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AlphaTestState {
    pub function: AlphaTestFunction,
    pub threshold: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AlphaTestFunction {
    Always,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Never,
}

impl AlphaTestFunction {
    fn from_bits(bits: u8) -> Result<Self, Error> {
        match bits {
            0 => Ok(Self::Always),
            1 => Ok(Self::Less),
            2 => Ok(Self::Equal),
            3 => Ok(Self::LessEqual),
            4 => Ok(Self::Greater),
            5 => Ok(Self::NotEqual),
            6 => Ok(Self::GreaterEqual),
            7 => Ok(Self::Never),
            _ => Err(Error::InvalidBlendMode(bits)),
        }
    }
}
