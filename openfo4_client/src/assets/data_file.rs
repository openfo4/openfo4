use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use image::{ImageBuffer, Rgba};
use openfo4_asset::{Asset, AssetLoader};
use openfo4_core::path::DataPathBuf;
use openfo4_dds::Image;
use openfo4_hkx::HkxFile;
use openfo4_nif::Nif;
use openfo4_nif::bgsm::Bgsm;
use tracing::instrument;

use super::Error;
use crate::loader::FileLoader;

#[derive(Clone, Debug)]
pub struct DataFileContext {
    pub loader: Arc<FileLoader>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataFile<T>(pub T);

impl<T> Deref for DataFile<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for DataFile<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Asset for DataFile<T>
where
    T: DecodeAsset + Send + Sync + 'static,
{
    type Key = DataPathBuf;
    type Context = DataFileContext;
    type Error = Error;

    #[instrument(name = "DataFile::load", skip_all)]
    async fn load(
        _loader: &AssetLoader,
        path: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let buf = ctx.loader.read(path).await.map_err(|err| match err {
            crate::loader::Error::NotFound(_) => Error::NotFound,
            err => Error::from(err),
        })?;

        let asset = T::decode(&buf)?;
        Ok(Self(asset))
    }
}

trait DecodeAsset: Sized {
    type Error: std::error::Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error>;
}

impl DecodeAsset for Bgsm {
    type Error = openfo4_nif::Error;

    #[instrument(name = "Bgsm::decode", skip_all)]
    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        Bgsm::decode(buf)
    }
}

impl DecodeAsset for Nif {
    type Error = openfo4_nif::Error;

    #[instrument(name = "Nif::decode", skip_all)]
    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        Nif::decode(buf)
    }
}

impl DecodeAsset for HkxFile {
    type Error = openfo4_hkx::Error;

    #[instrument(name = "HkxFile::decode", skip_all)]
    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        HkxFile::decode(buf)
    }
}

impl DecodeAsset for Image {
    type Error = openfo4_dds::Error;

    #[instrument(name = "Image::decode", skip_all)]
    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        Image::parse(buf)
    }
}

impl DecodeAsset for ImageBuffer<Rgba<u8>, Vec<u8>> {
    type Error = image::ImageError;

    #[instrument(name = "ImageBuffer::decode", skip_all)]
    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        Ok(image::load_from_memory(buf)?.to_rgba8())
    }
}
