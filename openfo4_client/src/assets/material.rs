use game_common::components::Color;
use game_core_pipeline::entities::MaterialHandle;
use game_core_pipeline::material::{AlphaMode, StandardMaterial};
use openfo4_asset::{Asset, AssetLoader, Handle};
use openfo4_core::path::DataPathBuf;
use openfo4_nif::bgsm::Bgsm;

use crate::assets::data_file::DataFile;
use crate::assets::nif::DECAL_DEPTH_BIAS;
use crate::assets::texture::FileImageHandle;
use crate::assets::{Error, FileLoadContext};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialKey {
    pub path: DataPathBuf,
    pub alpha_mode: Option<AlphaMode>,
}

#[derive(Clone, Debug)]
pub struct BgsmMaterial {
    pub material: MaterialHandle,
    pub textures: Vec<Handle<FileImageHandle>>,
}

impl Asset for BgsmMaterial {
    type Key = MaterialKey;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let standard = loader.load::<BgsmStandardMaterial, _>(key).await?;
        let material = ctx.entities.create_material(standard.material.clone());

        Ok(Self {
            material,
            textures: standard.textures.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct BgsmStandardMaterial {
    pub material: StandardMaterial,
    pub textures: Vec<Handle<FileImageHandle>>,
}

impl Asset for BgsmStandardMaterial {
    type Key = MaterialKey;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        _ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let bgsm = loader
            .load::<DataFile<Bgsm>, _>(&key.path)
            .await
            .map_err(|err| Error::Other(format!("failed to read file {}: {}", key.path, err)))?;

        let alpha_mode = key.alpha_mode.unwrap_or_else(|| {
            if bgsm.blending {
                AlphaMode::Blend
            } else if bgsm.alpha_test {
                AlphaMode::Mask(bgsm.alpha_test_threshold as f32 / 255.0)
            } else {
                AlphaMode::Opaque
            }
        });

        let mut material = StandardMaterial {
            alpha_mode,
            base_color: Color::from_rgba([1.0, 1.0, 1.0, bgsm.alpha]),
            flip_normal_y: true,
            two_component_normal_encoding: true,
            roughness: 1.0 - bgsm.smoothness,
            specular_color: Color::from_rgb(bgsm.specular_color.to_array()),
            specular_strength: bgsm.specular_strength,
            depth_bias: if bgsm.decal { DECAL_DEPTH_BIAS } else { 0.0 },
            ..Default::default()
        };

        let mut textures = Vec::new();

        if !bgsm.diffuse_texture.is_empty() {
            let handle = loader
                .load::<FileImageHandle, _>(&DataPathBuf::new(&*bgsm.diffuse_texture))
                .await?;

            material.base_color_texture = Some(handle.0.clone());
            textures.push(handle);
        }

        if !bgsm.normal_texture.is_empty() {
            let handle = loader
                .load::<FileImageHandle, _>(&DataPathBuf::new(&*bgsm.normal_texture))
                .await?;

            material.normal_texture = Some(handle.0.clone());
            textures.push(handle);
        }

        if !bgsm.smooth_spec_texture.is_empty() {
            let handle = loader
                .load::<FileImageHandle, _>(&DataPathBuf::new(&*bgsm.smooth_spec_texture))
                .await?;

            material.specular_glossiness_texture = Some(handle.0.clone());
            textures.push(handle);
        }

        if let Some(grayscale_palette) = &bgsm.grayscale_palette {
            let handle = loader
                .load::<FileImageHandle, _>(&DataPathBuf::new(
                    &*grayscale_palette.grayscale_texture,
                ))
                .await?;

            material.grayscale_palette_scale = grayscale_palette.scale;
            material.grayscale_palette = Some(handle.0.clone());
            textures.push(handle);
        }

        Ok(Self { material, textures })
    }
}
