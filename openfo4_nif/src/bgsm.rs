use std::io::Cursor;

use glam::Vec2;

use crate::{Decode, Endianess, Error, Reader, Rgb};

const MAGIC: u32 = 0x4d534742;

#[derive(Clone, Debug)]
pub struct Bgsm {
    pub tile_u: bool,
    pub tile_v: bool,
    pub uv_offset: Vec2,
    pub uv_scale: Vec2,

    pub diffuse_texture: String,
    pub normal_texture: String,
    pub smooth_spec_texture: String,

    pub alpha: f32,

    pub specular_color: Rgb,
    pub specular_strength: f32,

    pub smoothness: f32,

    pub grayscale_palette: Option<GreyscalePalette>,

    pub alpha_test: bool,
    pub alpha_test_threshold: u8,
}

impl Bgsm {
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader {
            buf: Cursor::new(buf),
            endian: Endianess::Little,
            strings: Vec::new(),
        };

        let magic = reader.get_u32()?;
        if magic != MAGIC {
            return Err(Error::BadMagic);
        }

        let version = reader.get_u32()?;

        let tile_flags = reader.get_u32()?;
        let tile_u = (tile_flags & 2) != 0;
        let tile_v = (tile_flags & 1) != 0;
        let uv_offset = Vec2::decode(&mut reader)?;
        let uv_scale = Vec2::decode(&mut reader)?;

        let alpha = reader.get_f32()?;
        let alpha_blend_mode0 = reader.get_u8()?;
        let alpha_blend_mode1 = reader.get_u32()?;
        let alpha_blend_mode2 = reader.get_u32()?;
        let alpha_test_threshold = reader.get_u8()?;
        let alpha_test = reader.get_bool()?;

        let z_buffer_write = reader.get_bool()?;
        let z_buffer_test = reader.get_bool()?;
        let screen_space_reflections = reader.get_bool()?;
        let wetness_control_screen_space_reflections = reader.get_bool()?;
        let decal = reader.get_bool()?;
        let two_sided = reader.get_bool()?;
        let decal_no_fade = reader.get_bool()?;
        let non_occluder = reader.get_bool()?;

        let refraction = reader.get_bool()?;
        let refraction_falloff = reader.get_bool()?;
        let refraction_power = reader.get_f32()?;

        if version < 10 {
            let environment_mapping = reader.get_bool()?;
            let environment_mapping_mask_scale = reader.get_f32()?;
        } else {
            let depth_bida = reader.get_bool()?;
        }

        let grayscale_to_palette_color = reader.get_bool()?;

        if version >= 6 {
            let mask_wrtes = reader.get_u8()?;
        }

        let diffuse_texture = read_string(&mut reader)?;
        let normal_texture = read_string(&mut reader)?;
        let smooth_spec_texture = read_string(&mut reader)?;
        let grayscale_texture = read_string(&mut reader)?;

        if version > 2 {
            let emissive_texture = read_string(&mut reader)?;
            let wrinkles_texture = read_string(&mut reader)?;
            let specular_texture = read_string(&mut reader)?;
            let lighting_texture = read_string(&mut reader)?;
            let flow_texture = read_string(&mut reader)?;

            if version >= 17 {
                let distnace_field_alpha_texture = read_string(&mut reader)?;
            }
        } else {
            let env_map_texture = read_string(&mut reader)?;
            let emissive_texture = read_string(&mut reader)?;
            let inner_layer_texture = read_string(&mut reader)?;
            let wrinkles_texture = read_string(&mut reader)?;
            let displacement_texture = read_string(&mut reader)?;
        }

        let enable_editor_alpha_ref = reader.get_bool()?;

        let mut translucency = None;
        if version >= 8 {
            if reader.get_bool()? {
                translucency = Some(Translucency::decode(&mut reader)?)
            }
        } else {
            let rim_lighting = reader.get_bool()?;
            let rim_power = reader.get_f32()?;
            let back_light_power = reader.get_f32()?;

            let subsurface_lighting = reader.get_bool()?;
            let subsurface_lighting_rolloff = reader.get_f32()?;
        }

        let specular_enabled = reader.get_bool()?;
        let specular_color = Rgb::decode(&mut reader)?;
        // Set specular_strength to 0 if specular_enabled is false.
        let specular_strength = f32::from(u8::from(specular_enabled)) * reader.get_f32()?;
        let smoothness = reader.get_f32()?;

        let fresnel_power = reader.get_f32()?;
        let wetness_control_spec_scale = reader.get_f32()?;
        let wetness_control_spec_power_scale = reader.get_f32()?;
        let wetness_control_spec_minvar = reader.get_f32()?;

        if version < 10 {
            let wetness_control_env_map_scale = reader.get_f32()?;
        }

        let wetness_control_fresnel_power = reader.get_f32()?;
        let wetness_control_metalness = reader.get_f32()?;

        if version > 2 {
            let pbr = reader.get_bool()?;

            if version >= 9 {
                let custom_porosity = reader.get_bool()?;
                let porosity_value = reader.get_f32()?;
            }
        }

        let root_material_path = read_string(&mut reader)?;

        let anisotropic_lighting = reader.get_bool()?;
        let emit_enabled = reader.get_bool()?;

        if emit_enabled {
            let emittance_color = Rgb::decode(&mut reader)?;
        }

        let emittance_mult = reader.get_f32()?;
        let model_space_normals = reader.get_bool()?;
        let external_emittance = reader.get_bool()?;

        if version >= 12 {
            let lum_emittance = reader.get_f32()?;
        }

        if version >= 13 {
            let use_adaptive_emissive = reader.get_bool()?;
            let adaptive_emissive_exposure_offset = reader.get_f32()?;
            let adaptive_emissive_final_exposure_min = reader.get_f32()?;
            let adaptive_emissive_final_expsoure_max = reader.get_f32()?;
        }

        if version < 8 {
            let back_lighting = reader.get_bool()?;
        }

        let receive_shadows = reader.get_bool()?;
        let hide_screet = reader.get_bool()?;
        let cast_shadows = reader.get_bool()?;
        let dissolve_fade = reader.get_bool()?;
        let assume_shadow_mask = reader.get_bool()?;

        let glowmap = reader.get_bool()?;

        if version < 7 {
            let environment_mapping_window = reader.get_bool()?;
            let environment_mapping_eye = reader.get_bool()?;
        }

        let hair = reader.get_bool()?;
        let hair_tint_color = Rgb::decode(&mut reader)?;

        let tree = reader.get_bool()?;
        let facegen = reader.get_bool()?;
        let skin_tint = reader.get_bool()?;
        let tessellate = reader.get_bool()?;

        if version < 3 {
            let displacement_texture_bias = reader.get_f32()?;
            let displacement_texture_scale = reader.get_f32()?;
            let tessellation_pn_scale = reader.get_f32()?;
            let tessellation_base_factor = reader.get_f32()?;
            let tessellation_fade_distance = reader.get_f32()?;
        }

        let grayscale_to_palette_scale = reader.get_f32()?;

        let grayscale_palette = grayscale_to_palette_color.then_some(GreyscalePalette {
            grayscale_texture,
            scale: grayscale_to_palette_scale,
        });

        Ok(Self {
            tile_u,
            tile_v,
            uv_offset,
            uv_scale,
            diffuse_texture,
            normal_texture,
            smooth_spec_texture,
            specular_color,
            specular_strength,
            smoothness,
            alpha,
            grayscale_palette,
            alpha_test,
            alpha_test_threshold,
        })
    }
}

fn read_string(reader: &mut Reader<'_>) -> Result<String, Error> {
    let len = reader.get_u32()?;
    let mut bytes = Vec::new();
    for _ in 0..len {
        bytes.push(reader.get_u8()?);
    }

    let mut s = String::from_utf8(bytes).unwrap();
    if s.ends_with('\0') {
        s.truncate(s.len() - 1);
    }

    Ok(s)
}

#[derive(Copy, Clone, Debug)]
pub struct Translucency {
    pub thick_object: bool,
    pub mix_albedo_with_subsurface_color: bool,
    pub subsurface_color: Rgb,
    pub transmissive_scale: f32,
    pub turbulance: f32,
}

impl Decode for Translucency {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let thick_object = reader.get_bool()?;
        let mix_albedo_with_subsurface_color = reader.get_bool()?;
        let subsurface_color = Rgb::decode(reader)?;
        let transmissive_scale = reader.get_f32()?;
        let turbulance = reader.get_f32()?;

        Ok(Self {
            thick_object,
            mix_albedo_with_subsurface_color,
            subsurface_color,
            transmissive_scale,
            turbulance,
        })
    }
}

#[derive(Clone, Debug)]
pub struct GreyscalePalette {
    pub grayscale_texture: String,
    pub scale: f32,
}
