use std::io::Read;

use openfo4_hkx::HkxFile;

use crate::loader::Reader;

use super::{Asset, AssetServer, Error};

impl Asset for HkxFile {
    async fn load(_server: &AssetServer, reader: &mut Reader) -> Result<Self, Error> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Self::decode(&buf).map_err(|e| e.into())
    }
}
