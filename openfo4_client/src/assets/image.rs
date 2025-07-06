use std::io::Read;

use bytes::Bytes;
use openfo4_dds::Image;

use crate::loader::Reader;

use super::{Asset, AssetServer, Error};

impl Asset for Image {
    async fn load(_server: &AssetServer, reader: &mut Reader) -> Result<Self, Error> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let buf = Bytes::from(buf);

        Image::parse(&buf).map_err(|e| e.into())
    }
}
