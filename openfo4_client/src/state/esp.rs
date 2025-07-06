use std::fs::File;
use std::io::BufReader;

use async_lock::{Mutex, RwLock};
use game_common::collections::lru::LruCache;
use game_tracing::trace_span;
use openfo4_esp::{AsRecordKind, Error, EspFile, FormId, Record, RecordChildren, RecordKind};

#[derive(Debug)]
pub struct EspLoader {
    files: Mutex<Vec<EspFile<BufReader<File>>>>,
    cache: RwLock<LruCache<FormId, Record>>,
}

impl EspLoader {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(Vec::new()),
            cache: RwLock::new(LruCache::new(65_536)),
        }
    }

    pub fn push(&mut self, file: EspFile<BufReader<File>>) {
        self.files.lock_blocking().push(file);
    }

    #[tracing::instrument(name = "EspLoader::lookup")]
    pub async fn lookup(&self, kind: Option<RecordKind>, id: FormId) -> Result<Record, Error> {
        // let _span = trace_span!("EspLoader::lookup").entered();

        {
            if let Some(record) = self.cache.write().await.get(&id) {
                return Ok(record.clone());
            }
        }

        for file in &mut *self.files.lock().await {
            let record = file.lookup(kind, id)?;
            self.cache.write().await.insert(id, record.clone());
            return Ok(record);
        }

        todo!()
    }

    pub async fn lookup_all(&self, kind: Option<RecordKind>) -> Result<Vec<Record>, Error> {
        for file in &mut *self.files.lock().await {
            let records = file.lookup_all(kind).collect::<Result<Vec<_>, _>>()?;
            // self.cache.write().await.insert(record.clone());
            return Ok(records);
        }

        todo!()
    }

    #[tracing::instrument(name = "EspLoader::children")]
    pub async fn children(&self, kind: RecordChildren, id: FormId) -> Result<Vec<Record>, Error> {
        // let _span = trace_span!("EspLoader::children").entered();

        for file in &mut *self.files.lock().await {
            return file.children(kind, id);
        }

        todo!();
    }
}
