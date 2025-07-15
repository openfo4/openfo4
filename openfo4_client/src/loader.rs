use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};

use openfo4_ba2::Archive;
use parking_lot::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(io::Error),
    #[error("error reading ba2 archive: {0}")]
    Ba2(openfo4_ba2::Error),
    #[error("file {0} not found")]
    NotFound(String),
}

#[derive(Debug)]
pub struct FileLoader {
    sources: Mutex<Vec<Source>>,
}

impl FileLoader {
    pub fn new<T, I>(locations: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<Path>,
    {
        let mut sources = Vec::new();

        for location in locations {
            let meta = match std::fs::metadata(&location) {
                Ok(meta) => meta,
                Err(err) => {
                    tracing::error!(
                        "cannot load from {}: {}",
                        location.as_ref().to_string_lossy(),
                        err
                    );

                    continue;
                }
            };

            tracing::info!("using data source {}", location.as_ref().to_string_lossy());

            let source = if meta.is_dir() {
                Source::Directory(location.as_ref().to_owned())
            } else {
                let file = File::open(location).unwrap();
                let archive = Archive::new(BufReader::new(file)).unwrap();
                Source::Archive(archive)
            };

            sources.push(source);
        }

        Self {
            sources: Mutex::new(sources),
        }
    }

    pub fn open(&self, path: &str) -> Result<Reader, Error> {
        let path = fix_path(path);

        tracing::debug!("loading {}", path);

        let mut sources = self.sources.lock();

        for source in sources.iter_mut().rev() {
            match source {
                Source::Directory(root) => {
                    let path = root.join(path);
                    let file = File::open(path).map_err(Error::Io)?;
                    return Ok(Reader::File(file));
                }
                Source::Archive(archive) => {
                    if !archive.contains(&path) {
                        continue;
                    }

                    let buf = archive.read(&path).map_err(Error::Ba2)?;
                    return Ok(Reader::Archive(Cursor::new(buf)));
                }
            }
        }

        Err(Error::NotFound(path))
    }
}

#[derive(Debug)]
enum Source {
    /// A filesystem root directory.
    Directory(PathBuf),
    Archive(Archive<BufReader<File>>),
}

#[derive(Debug)]
pub enum Reader {
    File(File),
    Archive(Cursor<Vec<u8>>),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(f) => f.read(buf),
            Self::Archive(v) => v.read(buf),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        match self {
            Self::File(f) => f.read_to_end(buf),
            Self::Archive(v) => v.read_to_end(buf),
        }
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            Self::File(f) => f.seek(pos),
            Self::Archive(v) => v.seek(pos),
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
