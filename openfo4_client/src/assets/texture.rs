use game_core_pipeline::entities::ImageHandle;
use game_render::backend::TextureFormat;
use game_render::texture::Image;
use game_render::texture::image::MipImage;
use openfo4_asset::{Asset, AssetLoader};
use openfo4_dds::Format;

use crate::assets::{Error, FileLoadContext};
use crate::loader::fix_path;

/// An [`ImageHandle`] that was loaded by a file.
#[derive(Clone, Debug)]
pub struct FileImageHandle(pub ImageHandle);

impl Asset for FileImageHandle {
    type Key = String;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        _loader: &AssetLoader,
        path: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let mut path = fix_path(&path);
        if !path.starts_with("textures") {
            path = format!("textures/{}", path);
        }

        if path.to_lowercase().ends_with(".dds") {
            let image = ctx
                .server
                .load::<openfo4_dds::Image>(&path)
                .await
                .map_err(|err| Error(format!("failed to read file {}: {}", path, err)))?;

            let format = match image.format {
                Format::Dxt1 => TextureFormat::Bc1RgbaUnorm,
                Format::Dxt2 => TextureFormat::Bc2RgbaUnorm,
                Format::Dxt3 => TextureFormat::Bc2RgbaUnorm,
                Format::Dxt4 => TextureFormat::Bc3RgbaUnorm,
                Format::Dxt5 => TextureFormat::Bc3RgbaUnorm,
                Format::Bc5U => TextureFormat::Bc5RgUnorm,
                Format::Rgb8 => TextureFormat::Rgb8Unorm,
                Format::Rgba8 => TextureFormat::Rgba8Unorm,
            };

            let mips = image
                .mips
                .iter()
                .map(|mip| Image::new(mip.size, format, mip.bytes.clone()))
                .collect();

            let handle = ctx.entities.create_image(MipImage::new(mips));

            Ok(Self(handle))
        } else {
            todo!("unknown file extension");
        }
    }
}
