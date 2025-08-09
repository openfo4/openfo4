use std::io::Read;
use std::ops::Deref;

use openfo4_hkx::HkxFile;

use crate::assets::FileLoadContext;
use crate::loader::Reader;

use super::{Asset, AssetServer, Error};

#[derive(Clone, Debug)]
pub struct FileHkxHandle(pub HkxFile);

impl Deref for FileHkxHandle {
    type Target = HkxFile;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Asset for HkxFile {
    async fn load(_server: &AssetServer, reader: &mut Reader) -> Result<Self, Error> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Self::decode(&buf).map_err(|e| e.into())
    }
}

impl openfo4_asset::Asset for FileHkxHandle {
    type Key = String;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        _loader: &openfo4_asset::AssetLoader,
        path: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let file: crate::assets::Handle<HkxFile> = ctx
            .server
            .load(&path)
            .await
            .map_err(|err| Error(format!("failed to read file: {}: {}", path, err)))?;

        Ok(Self((*file).clone()))
    }
}
