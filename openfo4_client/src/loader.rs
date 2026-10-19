use std::io;
use std::path::{Path, PathBuf};

use hashbrown::HashMap;
use openfo4_ba2::Archive;
use openfo4_core::path::{DataPath, DataPathBuf};
use openfo4_io::fs::{AsyncReadAt, File};
use openfo4_tracing::trace_span;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(io::Error),
    #[error("error reading ba2 archive: {0}")]
    Ba2(openfo4_ba2::Error),
    #[error("file {0} not found")]
    NotFound(DataPathBuf),
}

#[derive(Debug)]
pub struct FileLoader {
    sources: Vec<Source>,
    files: HashMap<DataPathBuf, usize>,
}

impl FileLoader {
    pub fn new<T, I>(locations: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<Path>,
    {
        let _span = trace_span!("FileLoader::new").entered();

        let mut sources = Vec::new();
        let mut files = HashMap::new();

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

            let source_index = sources.len();

            let source = if meta.is_dir() {
                let source = DirectorySource::new(&location).unwrap();

                for file in source.files.keys() {
                    files.insert(file.clone(), source_index);
                }

                Source::Directory(source)
            } else {
                let file = File::open(location).unwrap();
                let archive = futures_lite::future::block_on(Archive::new(file)).unwrap();

                for file in archive.file_names() {
                    files.insert(file.to_owned(), source_index);
                }

                Source::Archive(archive)
            };

            sources.push(source);
        }

        // Drop excess capacity, these will stay unchanged for the
        // entire process lifetime.
        sources.shrink_to_fit();
        files.shrink_to_fit();

        Self { sources, files }
    }

    pub async fn read(&self, path: &DataPath) -> Result<Vec<u8>, Error> {
        tracing::debug!("loading {}", path);

        let Some(index) = self.files.get(path) else {
            return Err(Error::NotFound(path.to_owned()));
        };

        match &self.sources[*index] {
            Source::Directory(source) => {
                let entry = source.files.get(path).unwrap();
                let file = File::open(&entry.path).map_err(Error::Io)?;

                let mut buf = vec![0; entry.len as usize];
                file.read_exact_at(&mut buf, 0).await.map_err(Error::Io)?;

                Ok(buf)
            }
            Source::Archive(archive) => {
                let buf = archive.read(&path).await.map_err(Error::Ba2)?;
                Ok(buf)
            }
        }
    }

    pub fn files(&self) -> impl Iterator<Item = &DataPath> {
        self.files.keys().map(|v| v.as_ref())
    }
}

#[derive(Debug)]
enum Source {
    /// A filesystem root directory.
    Directory(DirectorySource),
    Archive(Archive<File>),
}

#[derive(Debug)]
struct DirectorySource {
    files: HashMap<DataPathBuf, FileEntry>,
}

impl DirectorySource {
    fn new<P>(root_dir: P) -> Result<Self, io::Error>
    where
        P: AsRef<Path>,
    {
        let mut files = HashMap::new();

        let mut directories = vec![root_dir.as_ref().to_owned()];
        while let Some(path) = directories.pop() {
            let dir = std::fs::read_dir(&path)?;
            for entry in dir {
                let entry = entry?;
                let meta = entry.metadata()?;
                let file_path = entry.path();

                if meta.is_symlink() {
                    continue;
                }

                if meta.is_dir() {
                    directories.push(file_path);
                    continue;
                }

                if meta.is_file() {
                    // We recursively visit all subentries of the root directory,
                    // so all paths should start with the root directory.
                    let name = file_path.strip_prefix(&root_dir).unwrap();
                    let name = DataPathBuf::new(name.to_str().unwrap());

                    files.insert(
                        name,
                        FileEntry {
                            path: file_path,
                            len: meta.len(),
                        },
                    );
                }
            }
        }

        Ok(Self { files })
    }
}

#[derive(Clone, Debug)]
struct FileEntry {
    path: PathBuf,
    len: u64,
}
