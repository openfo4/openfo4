pub mod animation;
pub mod data_file;
pub mod material;
pub mod nif_v2;
pub mod texture;

use std::fmt::{self, Debug, Display, Formatter};

use game_core_pipeline::entities::Entities;

#[derive(Debug)]
pub struct FileLoadContext {
    pub entities: Entities,
}

#[derive(Debug)]
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
