pub mod animation;
pub mod data_file;
pub mod gltf;
pub mod material;
pub mod nif;
pub mod texture;

use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

use openfo4_asset::AssetLoader;
use openfo4_core_pipeline::entities::Entities;
use openfo4_hkx::HkxFile;
use openfo4_nif::Nif;
use openfo4_nif::bgsm::Bgsm;

use crate::assets::animation::FileAnimationClip;
use crate::assets::data_file::{DataFile, DataFileContext};
use crate::assets::gltf::GltfScene;
use crate::assets::material::{BgsmMaterial, BgsmStandardMaterial};
use crate::assets::nif::{NifLoaderContext, NifScene};
use crate::assets::texture::{FileImage, FileImageHandle};
use crate::loader::FileLoader;

#[derive(Debug)]
pub struct FileLoadContext {
    pub entities: Entities,
}

#[derive(Clone, Debug)]
pub enum Error {
    NotFound,
    Other(String),
}

impl<T> From<T> for Error
where
    T: std::error::Error,
{
    fn from(value: T) -> Self {
        Self::Other(value.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::Other(err) => write!(f, "{}", err),
        }
    }
}

pub fn init_asset_loader(file_loader: &Arc<FileLoader>, entities: &Entities) -> AssetLoader {
    let loader = AssetLoader::new();
    loader.register::<NifScene>(NifLoaderContext::new(entities.clone()));
    loader.register::<GltfScene>(NifLoaderContext::new(entities.clone()));
    loader.register::<FileImage>(());
    loader.register::<FileImageHandle>(FileLoadContext {
        entities: entities.clone(),
    });
    loader.register::<BgsmStandardMaterial>(FileLoadContext {
        entities: entities.clone(),
    });
    loader.register::<BgsmMaterial>(FileLoadContext {
        entities: entities.clone(),
    });
    loader.register::<DataFile<Nif>>(DataFileContext {
        loader: file_loader.clone(),
    });
    loader.register::<DataFile<HkxFile>>(DataFileContext {
        loader: file_loader.clone(),
    });
    loader.register::<DataFile<Bgsm>>(DataFileContext {
        loader: file_loader.clone(),
    });
    loader.register::<DataFile<openfo4_dds::Image>>(DataFileContext {
        loader: file_loader.clone(),
    });
    loader.register::<FileAnimationClip>(());

    loader
}
