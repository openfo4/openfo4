use std::fmt::Display;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use thiserror::Error;
use toml_edit::{Document, Item, Key, Table, Value};

pub use toml_edit;

/// Derive [`ConfigSection`] for a struct.
///
/// # Examples
///
/// ```
/// # use serde::Deserialize;
/// # use openfo4_config::ConfigSection;
/// #[derive(Default, Deserialize, ConfigSection)]
/// struct Test {
///     x: bool,
///     /// This comment is included in the config file!
///     y: u32,
/// }
/// ```
pub use openfo4_config_macros::ConfigSection;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml_edit::de::Error),
    #[error(transparent)]
    Toml(#[from] toml_edit::TomlError),
}

/// A section of a configuration.
pub trait ConfigSection: Default + DeserializeOwned {
    fn write(table: &mut Table) -> bool;

    /// Loads this config from the file at the given [`Path`].
    ///
    /// If the file does not exist it will be created.
    /// If the file is writable it will be updated when the format of this config changed.
    /// If the file is not writable it will be read, but not updated.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file exists, but cannot be opened.
    /// - The file does not exist and cannot be created.
    /// - The file contains invalid data.
    /// - The changes cannot be written back if the file was opened writable.
    fn from_file<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let mut opts = OpenOptions::new();
        opts.read(true);
        opts.write(true);

        let (mut file, writable) = match opts.open(path.as_ref()) {
            Ok(file) => (file, true),
            // File does not exist, try to create it.
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let file = File::create_new(path.as_ref())?;
                (file, true)
            }
            Err(err) if err.kind() == ErrorKind::PermissionDenied => {
                let file = File::open(path.as_ref())?;
                (file, false)
            }
            Err(err) => return Err(err.into()),
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        let config = toml_edit::de::from_str(&buf)?;
        let mut document = Document::parse(buf)?.into_mut();

        let root = document.as_table_mut();
        let updated = Self::write(root);
        if updated && writable {
            let new_buf = document.to_string();

            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(new_buf.as_bytes())?;
        }

        Ok(config)
    }
}

/// A value that can be used inside a [`ConfigSection`] definition.
pub trait ConfigValue {
    fn insert(&self, table: &mut Table, key: &str, prefix: &str) -> bool;
}

macro_rules! config_value_int {
    ($($t:ty),*) => {
        $(
            impl ConfigValue for $t {
                fn insert(&self, table: &mut Table, key: &str, prefix: &str) -> bool {
                    i64::from(*self).insert(table, key, prefix)
                }
            }
        )*
    };
}

config_value_int!(u8, u16, u32, i8, i16, i32);

impl ConfigValue for i64 {
    fn insert(&self, table: &mut Table, key: &str, prefix: &str) -> bool {
        insert_if_missing(table, key, *self, prefix)
    }
}

impl ConfigValue for bool {
    fn insert(&self, table: &mut Table, key: &str, prefix: &str) -> bool {
        insert_if_missing(table, key, *self, prefix)
    }
}

impl<T> ConfigValue for T
where
    T: ConfigSection,
{
    fn insert(&self, table: &mut Table, key: &str, _prefix: &str) -> bool {
        let mut updated = false;

        if let Item::Table(nested) = table
            .entry(key)
            .or_insert_with(|| Item::Table(Table::new()))
        {
            updated |= Self::write(nested);
        }

        updated
    }
}

fn insert_if_missing<T>(table: &mut Table, key: &str, default_value: T, prefix: &str) -> bool
where
    T: Into<Value> + Display,
{
    if table.contains_key(key) {
        return false;
    }

    let prefix = format!("{}{}\n", prefix, default_value);

    let mut key = Key::new(key);
    key.leaf_decor_mut().set_prefix(prefix);

    table.insert_formatted(&key, Item::Value(default_value.into()));
    true
}
