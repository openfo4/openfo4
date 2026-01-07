use std::path::Path;

use async_lock::RwLock;
use game_common::collections::lru::LruCache;
use hashbrown::HashMap;
use openfo4_esp::{Error, EspFile, FormId, Record, RecordChildren, RecordKind};
use openfo4_io::fs::File;

use crate::state::hardcoded::load_hardcoded_forms;

#[derive(Debug)]
pub struct EspLoader {
    files: Vec<EspFile<File>>,
    cache: RwLock<LruCache<FormId, Record>>,
    hardcoded: HashMap<FormId, Record>,
}

impl EspLoader {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            cache: RwLock::new(LruCache::new(65_536)),
            hardcoded: load_hardcoded_forms(),
        }
    }

    pub fn push<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let file = futures_lite::future::block_on(EspFile::new(file)).unwrap();
        self.files.push(file);
    }

    #[tracing::instrument(name = "EspLoader::lookup")]
    pub async fn lookup(&self, kind: Option<RecordKind>, id: FormId) -> Result<Record, Error> {
        if let Some(record) = self.hardcoded.get(&id) {
            return Ok(record.clone());
        }

        {
            if let Some(record) = self.cache.write().await.get(&id) {
                return Ok(record.clone());
            }
        }

        for file in &self.files {
            let record = file.lookup(kind, id).await?;
            self.cache.write().await.insert(id, record.clone());
            return Ok(record);
        }

        todo!()
    }

    pub async fn lookup_all(&self, kind: Option<RecordKind>) -> Result<Vec<Record>, Error> {
        for file in &self.files {
            let records = file.lookup_all(kind).collect::<Result<Vec<_>, _>>()?;
            // self.cache.write().await.insert(record.clone());
            return Ok(records);
        }

        // TODO: We should also yield hardcoded forms here.

        todo!()
    }

    #[tracing::instrument(name = "EspLoader::children")]
    pub async fn children(&self, kind: RecordChildren, id: FormId) -> Result<Vec<Record>, Error> {
        for file in &self.files {
            return file.children(kind, id).await;
        }

        todo!();
    }
}
