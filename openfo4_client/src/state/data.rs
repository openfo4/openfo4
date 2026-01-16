use std::io;
use std::path::{Path, PathBuf};

use game_tracing::trace_span;

const PLUGIN_EXT: &[&str] = &["esp", "esl", "esm"];
const ARCHIVE_EXT: &[&str] = &["ba2"];

#[derive(Clone, Debug)]
pub struct DataDirectoryContents {
    pub plugins: Vec<PathBuf>,
    pub archives: Vec<PathBuf>,
    /// Filesystem directory paths from which to load files from.
    pub directories: Vec<PathBuf>,
}

pub fn scan_data_directory<P>(path: P) -> Result<DataDirectoryContents, io::Error>
where
    P: AsRef<Path>,
{
    let _span = trace_span!("scan_data_directory").entered();

    let mut plugins = Vec::new();
    let mut archives = Vec::new();
    let mut directories = Vec::new();

    let metadata = path.as_ref().metadata()?;
    if metadata.file_type().is_dir() {
        directories.push(path.as_ref().to_owned());

        let mut dirs = vec![path.as_ref().to_owned()];
        while let Some(path) = dirs.pop() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;

                let file_type = entry.file_type()?;
                let file_path = entry.path();

                // We don't handle symlinks here.
                // Using symlinks for mod contents itself seems of limited use itself
                // and we avoid cycles all together if we just ignore symlinks here.
                if file_type.is_symlink() {
                    continue;
                }

                if file_type.is_dir() {
                    dirs.push(file_path);
                    continue;
                }

                if file_path.is_file() {
                    if is_plugin_file(&file_path) {
                        plugins.push(file_path);
                    } else if is_archive_file(&file_path) {
                        archives.push(file_path);
                    }
                }
            }
        }
    } else if metadata.file_type().is_file() {
        if is_plugin_file(&path) {
            plugins.push(path.as_ref().to_owned());
        } else if is_archive_file(&path) {
            archives.push(path.as_ref().to_owned());
        }
    }

    Ok(DataDirectoryContents {
        plugins,
        archives,
        directories,
    })
}

fn is_plugin_file<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    path.as_ref()
        .extension()
        .is_some_and(|ext| PLUGIN_EXT.iter().any(|v| ext.eq_ignore_ascii_case(v)))
}

fn is_archive_file<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    path.as_ref()
        .extension()
        .is_some_and(|ext| ARCHIVE_EXT.iter().any(|v| ext.eq_ignore_ascii_case(v)))
}
