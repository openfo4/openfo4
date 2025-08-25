use game_common::components::Color;
use game_core_pipeline::entities::MaterialHandle;
use game_core_pipeline::material::{AlphaMode, StandardMaterial};
use openfo4_asset::{Asset, AssetLoader, Handle};
use openfo4_nif::bgsm::Bgsm;

use crate::assets::texture::FileImageHandle;
use crate::assets::{Error, FileLoadContext};

#[derive(Clone, Debug)]
pub struct BgsmMaterial {
    pub material: MaterialHandle,
    pub textures: Vec<Handle<FileImageHandle>>,
}

impl Asset for BgsmMaterial {
    type Key = String;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        loader: &AssetLoader,
        path: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let bgsm = ctx
            .server
            .load::<Bgsm>(&path)
            .await
            .map_err(|err| Error(format!("failed to read file {}: {}", path, err)))?;

        let mut material = StandardMaterial {
            base_color: Color::from_rgba([1.0, 1.0, 1.0, bgsm.alpha]),
            flip_normal_y: true,
            two_component_normal_encoding: true,
            roughness: 1.0 - bgsm.smoothness,
            specular_color: Color::from_rgb(bgsm.specular_color.to_array()),
            specular_strength: bgsm.specular_strength,
            // alpha_mode: game_core_pipeline::material::AlphaMode::Mask,
            ..Default::default()
        };

        let mut textures = Vec::new();

        if !bgsm.diffuse_texture.is_empty() {
            let handle = loader
                .load::<FileImageHandle, _>(&bgsm.diffuse_texture)
                .await?;

            material.base_color_texture = Some(handle.0.clone());
            textures.push(handle);
        }

        if !bgsm.normal_texture.is_empty() {
            let handle = loader
                .load::<FileImageHandle, _>(&bgsm.normal_texture)
                .await?;

            material.normal_texture = Some(handle.0.clone());
            textures.push(handle);
        }

        if !bgsm.smooth_spec_texture.is_empty() {
            let handle = loader
                .load::<FileImageHandle, _>(&bgsm.smooth_spec_texture)
                .await?;

            material.specular_glossiness_texture = Some(handle.0.clone());
            textures.push(handle);
        }

        if let Some(grayscale_palette) = &bgsm.grayscale_palette {
            let handle = loader
                .load::<FileImageHandle, _>(&grayscale_palette.grayscale_texture)
                .await?;

            // material.grayscale = Some(handle.0.clone());
            textures.push(handle);
        }

        if bgsm.alpha_test {
            material.alpha_mode = AlphaMode::Mask;
        }

        let material = ctx.entities.create_material(material);

        Ok(Self { material, textures })
    }
}
