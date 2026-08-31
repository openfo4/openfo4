use game_core_pipeline::entities::ImageHandle;
use game_render::backend::TextureFormat;
use game_render::texture::Image;
use game_render::texture::image::MipImage;
use openfo4_asset::{Asset, AssetLoader};
use openfo4_core::path::{DataPath, DataPathBuf};
use openfo4_dds::Format;

use crate::assets::data_file::DataFile;
use crate::assets::{Error, FileLoadContext};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileImageKey {
    pub path: DataPathBuf,
}

/// An [`ImageHandle`] that was loaded by a file.
#[derive(Clone, Debug)]
pub struct FileImageHandle(pub ImageHandle);

impl Asset for FileImageHandle {
    type Key = FileImageKey;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let image = loader.load::<FileImage, _>(key).await?;
        let handle = ctx.entities.create_image(image.0.clone());
        Ok(Self(handle))
    }
}

#[derive(Clone, Debug)]
pub struct FileImage(pub MipImage);

impl Asset for FileImage {
    type Key = FileImageKey;
    type Error = Error;
    type Context = ();

    async fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        _ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let mut path = key.path.clone();
        if !path.starts_with(DataPath::from_str("textures")) {
            path.push_front("textures");
        }

        if path.extension().is_some_and(|v| v == "dds") {
            let image = loader
                .load::<DataFile<openfo4_dds::Image>, _>(&path)
                .await
                .map_err(|err| Error::Other(format!("failed to read file {}: {}", path, err)))?;

            let format = convert_dds_format(image.format).ok_or_else(|| {
                Error::Other(format!(
                    "unsupported DXGI format in {}: {:?}",
                    path, image.format
                ))
            })?;

            let mips = image
                .mips
                .iter()
                .map(|mip| Image::new(mip.size, format, mip.bytes.clone()))
                .collect();

            Ok(Self(MipImage::new(mips)))
        } else {
            todo!("unknown file extension");
        }
    }
}

fn convert_dds_format(format: Format) -> Option<TextureFormat> {
    match format {
        Format::Rgba8Unorm => Some(TextureFormat::Rgba8Unorm),
        Format::Rgba8UnormSrgb => Some(TextureFormat::Rgba8UnormSrgb),
        Format::Bgra8Unorm => Some(TextureFormat::Bgra8Unorm),
        Format::Bgra8UnormSrgb => Some(TextureFormat::Bgra8UnormSrgb),
        Format::Bc1Unorm => Some(TextureFormat::Bc1RgbaUnorm),
        Format::Bc1UnormSrgb => Some(TextureFormat::Bc1RgbaUnormSrgb),
        Format::Bc2Unorm => Some(TextureFormat::Bc2RgbaUnorm),
        Format::Bc2UnormSrgb => Some(TextureFormat::Bc2RgbaUnormSrgb),
        Format::Bc3Unorm => Some(TextureFormat::Bc3RgbaUnorm),
        Format::Bc3UnormSrgb => Some(TextureFormat::Bc3RgbaUnormSrgb),
        Format::Bc4Unorm => Some(TextureFormat::Bc4RUnorm),
        Format::Bc4Snorm => Some(TextureFormat::Bc4RSnorm),
        Format::Bc5Unorm => Some(TextureFormat::Bc5RgUnorm),
        Format::Bc5Snorm => Some(TextureFormat::Bc5RgSnorm),
        Format::Bc7Unorm => Some(TextureFormat::Bc7RgbaUnorm),
        Format::Bc7UnormSrgb => Some(TextureFormat::Bc7RgbaUnormSrgb),
        _ => None,
    }
}
