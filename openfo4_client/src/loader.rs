use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(io::Error),
}

#[derive(Debug)]
pub struct FileLoader {
    locations: Vec<PathBuf>,
}

impl FileLoader {
    pub fn new<T, I>(locations: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<Path>,
    {
        let locations = locations
            .into_iter()
            .map(|path| path.as_ref().to_owned())
            .collect();

        Self { locations }
    }

    pub fn open(&self, path: &str) -> Result<Reader, Error> {
        let path = format!("../../archive/raw/Data/{}", fix_path(path));
        tracing::info!("loading {}", path);
        let file = File::open(path).map_err(Error::Io)?;

        Ok(Reader::File(file))
    }
}

#[derive(Debug)]
pub enum Reader {
    File(File),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(f) => f.read(buf),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        match self {
            Self::File(f) => f.read_to_end(buf),
        }
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            Self::File(f) => f.seek(pos),
        }
    }
}

/// Normalizes the asset path.
pub fn fix_path(path: &str) -> String {
    let mut path = path.trim().replace("\\", "/");
    path.make_ascii_lowercase();

    let mut start = 0;
    let mut acc = 0;
    for seg in path.split('/') {
        if seg != "data" {
            acc += seg.len() + 1;
            continue;
        }

        start = acc + seg.len() + 1;
    }
    let path = &path[start..];

    // The file paths are all over the place.
    // Some start with data\, some with \data, some are even absolute
    // and start at c:\ and some have no prefix at all.
    let path = path.strip_prefix("/").unwrap_or(&path);
    let path = path.strip_prefix("data/").unwrap_or(&path);
    let path = path.strip_prefix("data").unwrap_or(&path);

    path.to_owned()
}

#[cfg(test)]
mod tests {
    use crate::loader::fix_path;

    #[test]
    fn test_fix_path() {
        assert_eq!(fix_path("\\NO_UPPERCASE\\ALLOWED"), "no_uppercase/allowed");
        assert_eq!(fix_path("meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(fix_path("\\meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(fix_path("data\\meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(fix_path("\\data\\meshes\\my_mesh"), "meshes/my_mesh");
        assert_eq!(
            fix_path("c:\\a\\b\\data\\meshes\\my_mesh"),
            "meshes/my_mesh"
        );
        assert_eq!(
            fix_path("c:\\a\\b\\data\\a\\b\\data\\meshes\\my_mesh"),
            "meshes/my_mesh"
        );
    }
}
