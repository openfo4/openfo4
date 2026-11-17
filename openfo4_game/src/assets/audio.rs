use std::io::Cursor;

use bytes::Bytes;
use openfo4_asset::{Asset, AssetLoader};
use openfo4_audio::source::AudioSource;
use openfo4_core::path::DataPathBuf;
use tracing::instrument;

use crate::assets::Error;
use crate::assets::data_file::DataFileContext;

#[derive(Clone, Debug)]
pub struct FileAudioSource {
    bytes: Bytes,
}

impl FileAudioSource {
    /// Creates a new audio stream from the source.
    ///
    /// The stream will borrow the data from the source.
    pub fn stream(&self) -> AudioSource {
        let source = Cursor::new(self.bytes.clone());
        AudioSource::from_reader(source).unwrap()
    }
}

impl Asset for FileAudioSource {
    type Key = DataPathBuf;
    type Error = Error;
    type Context = DataFileContext;

    #[instrument(name = "FileAudioSource::load", skip_all)]
    async fn load(
        _loader: &AssetLoader,
        key: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let buf = ctx.loader.read(key).await.map_err(|err| match err {
            crate::loader::Error::NotFound(_) => Error::NotFound,
            err => Error::from(err),
        })?;

        let bytes = Bytes::from(buf);

        Ok(Self { bytes })
    }
}
