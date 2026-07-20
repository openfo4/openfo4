use std::fmt::Display;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;
use toml_edit::{Document, Item, Key, Table, Value};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    TomlSerde(#[from] toml_edit::de::Error),
    #[error(transparent)]
    Toml(#[from] toml_edit::TomlError),
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub scripting: Scripting,
    pub streaming: Streaming,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self, ConfigError>
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
            Err(err) => panic!("{:?}", err),
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        let (config, new_buf) = Self::parse_and_update(&buf)?;

        if let Some(new_buf) = new_buf
            && writable
        {
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(new_buf.as_bytes())?;
        }

        Ok(config)
    }

    fn parse_and_update(buf: &str) -> Result<(Self, Option<String>), ConfigError> {
        let config = toml_edit::de::from_str(buf)?;

        let mut document = Document::parse(buf)?.into_mut();

        let root = document.as_table_mut();

        if let Item::Table(table) = root
            .entry("scripting")
            .or_insert_with(|| Item::Table(Table::new()))
        {
            Scripting::write(table);
        }

        if let Item::Table(table) = root
            .entry("streaming")
            .or_insert_with(|| Item::Table(Table::new()))
        {
            Streaming::write(table);
        }

        Ok((config, Some(document.to_string())))
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Scripting {
    pub validation: bool,
    pub debug: bool,
}

impl Scripting {
    fn write(table: &mut Table) {
        let default = Self::default();

        insert_if_missing(
            table,
            "validation",
            default.validation,
            r#"Enable extra validations in the script compiler and runtime.
            Note that this option does not affect the validation pass for papyrus script,
            that pass cannot be disabled.
            Enabling this option comes at a significant performance penalty, for both
            script compilation time and script execution time.
            "#,
        );
        insert_if_missing(
            table,
            "debug",
            default.debug,
            r#"Enable generation of debug info for compiled scripts.
            Enabling this option comes at a significant performance penality for script
            compilation.
            "#,
        );
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Streaming {
    pub active: u32,
    pub visible: u32,
}

impl Streaming {
    fn write(table: &mut Table) {
        let default = Self::default();

        insert_if_missing(
            table,
            "active",
            i64::from(default.active),
            r#"The radius around the player's current cell which are considered active
            for gameplay.
            "#,
        );
        insert_if_missing(
            table,
            "visible",
            i64::from(default.visible),
            r#"The radius around the player's current cell which are rendered.
            "#,
        );
    }
}

impl Default for Streaming {
    fn default() -> Self {
        Self {
            active: 2,
            visible: 3,
        }
    }
}

fn insert_if_missing<T>(table: &mut Table, key: &str, default_value: T, comment: &str)
where
    T: Into<Value> + Display,
{
    if table.contains_key(key) {
        return;
    }

    let lines = comment.lines().map(|line| line.trim()).collect::<Vec<_>>();
    let prefix = lines.join("\n# ");
    let prefix = format!("# {}\n# Default Value: {}\n", prefix, default_value);

    let mut key = Key::new(key);
    key.leaf_decor_mut().set_prefix(prefix);

    table.insert_formatted(&key, Item::Value(default_value.into()));
}
