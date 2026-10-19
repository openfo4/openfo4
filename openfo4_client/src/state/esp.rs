use std::num::NonZeroU32;
use std::path::Path;

use async_lock::RwLock;
use hashbrown::{HashMap, HashSet};
use openfo4_common::collections::lru::LruCache;
use openfo4_common::string::Lowercase;
use openfo4_core::record::{ModuleId, RecordId};
use openfo4_esp::{EspFile, FormId, Record, RecordChildren, RecordData, RecordKind};
use openfo4_io::fs::File;

#[derive(Debug)]
pub struct EspLoaderBuilder {
    files: Vec<EspFile<File>>,
    file_names: Vec<String>,
    hardcoded: HashMap<RecordId, Record>,
}

impl EspLoaderBuilder {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            file_names: Vec::new(),
            hardcoded: HashMap::new(),
        }
    }

    pub fn push<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        self.file_names.push(
            path.as_ref()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        );

        let file = File::open(path).unwrap();
        let file = futures_lite::future::block_on(EspFile::new(file)).unwrap();

        self.files.push(file);
    }

    pub fn set_hardcoded(&mut self, hardcoded: HashMap<RecordId, Record>) {
        self.hardcoded = hardcoded;
    }

    pub fn finalize(self) -> EspLoader {
        let mut next_module_id = NonZeroU32::MIN;

        #[derive(Debug)]
        struct Entry {
            file: EspFile<File>,
            dependencies: Vec<String>,
            name: String,
        }

        let mut name_to_id = HashMap::new();
        let mut entries = HashMap::new();

        let mut dependencies = HashMap::<String, HashSet<String>>::new();
        let mut dependents = HashMap::<String, HashSet<String>>::new();
        let mut queue = Vec::new();

        for (file, name) in self.files.into_iter().zip(self.file_names.into_iter()) {
            let name = name.to_lowercase();

            let RecordData::Tes4(header) =
                futures_lite::future::block_on(file.lookup(Some(RecordKind::Tes4), FormId::NULL))
                    .unwrap()
                    .data
            else {
                unreachable!();
            };

            let id = ModuleId(next_module_id);
            next_module_id = next_module_id
                .checked_add(1)
                .expect("plugin file limit reached");

            let deps = header
                .master_files
                .iter()
                .map(|v| v.file_name.as_str().unwrap().to_lowercase())
                .collect::<Vec<_>>();

            entries.insert(
                id,
                Entry {
                    file,
                    dependencies: deps.clone(),
                    name: name.clone(),
                },
            );
            name_to_id.insert(name.clone(), id);

            if deps.is_empty() {
                queue.push(id);
            } else {
                debug_assert!(!dependencies.contains_key(&name));
                dependencies.insert(name.clone(), deps.iter().cloned().collect());

                for dep in deps {
                    dependents.entry(dep).or_default().insert(name.clone());
                }
            }
        }

        let mut files = Vec::with_capacity(entries.len());
        let mut plugin_file_names = HashMap::new();

        // +1 because ModuleIds start at 1.
        let mut plugins = Vec::with_capacity(entries.len() + 1);
        plugins.resize(entries.len() + 1, 0);

        while let Some(id) = queue.pop() {
            let entry = entries.remove(&id).unwrap();

            let mut deps = Vec::with_capacity(entry.dependencies.len() + 1);

            for dep in &entry.dependencies {
                let id = *name_to_id.get(dep).unwrap();
                deps.push(id);
            }

            // The first index after dependency plugins refers to the plugin itself.
            deps.push(id);

            let index = files.len();
            files.push(PluginFile {
                file: entry.file,
                id,
                dependencies: deps.into_boxed_slice(),
            });
            plugins[id.0.get() as usize] = index as u32;

            plugin_file_names.insert(Lowercase::new(&*entry.name), id);

            if let Some(dependents) = dependents.remove(&entry.name) {
                for dep in dependents {
                    let deps = dependencies.get_mut(&dep).unwrap();
                    assert!(deps.remove(&entry.name));
                    if deps.is_empty() {
                        let id = *name_to_id.get(&dep).unwrap();
                        queue.push(id);
                    }
                }
            }
        }

        assert!(dependents.is_empty());

        EspLoader {
            files,
            plugins: plugins.into_boxed_slice(),
            cache: RwLock::new(LruCache::new(65_536)),
            plugin_file_names,
            hardcoded: self.hardcoded,
        }
    }
}

#[derive(Debug)]
pub struct EspLoader {
    files: Vec<PluginFile>,
    plugins: Box<[u32]>,
    cache: RwLock<LruCache<RecordId, GlobalRecord>>,
    hardcoded: HashMap<RecordId, Record>,
    plugin_file_names: HashMap<Lowercase<String>, ModuleId>,
}

impl EspLoader {
    #[tracing::instrument(name = "EspLoader::lookup")]
    pub async fn lookup(
        &self,
        kind: Option<RecordKind>,
        id: RecordId,
    ) -> Result<GlobalRecord, Error> {
        if let Some(record) = self.hardcoded.get(&id) {
            return Ok(GlobalRecord {
                id,
                local: record.clone(),
            });
        }

        {
            if let Some(record) = self.cache.write().await.get(&id) {
                return Ok(record.clone());
            }
        }

        let index = self.plugins[id.module.0.get() as usize];
        let file = &self.files[index as usize];
        let Some(index) = file.dependencies.iter().position(|v| *v == id.module) else {
            return Err(Error::EndOfFile);
        };

        // The plugin index refering to the file itself is the index
        // after all dependencies.
        let form = FormId::new(index as u8, id.form.form());

        let record = file.file.lookup(kind, form).await?;

        let record = GlobalRecord { id, local: record };

        self.cache.write().await.insert(id, record.clone());

        Ok(record)
    }

    pub async fn lookup_all(&self, kind: Option<RecordKind>) -> Result<Vec<GlobalRecord>, Error> {
        let mut records = Vec::new();

        // TODO: We should also yield hardcoded forms here.

        for file in &self.files {
            let local_records = file.file.lookup_all(kind).collect::<Result<Vec<_>, _>>()?;
            records.reserve(local_records.len());
            for local in local_records {
                let record = GlobalRecord {
                    id: RecordId {
                        module: file.id,
                        form: local.id,
                    },
                    local,
                };

                records.push(record);
            }
        }

        Ok(records)
    }

    #[tracing::instrument(name = "EspLoader::children")]
    pub async fn children(
        &self,
        kind: RecordChildren,
        id: RecordId,
    ) -> Result<Vec<GlobalRecord>, Error> {
        let mut records = Vec::new();

        for file in &self.files {
            // Resolve the parent to local plugin index.
            let Some(index) = file.dependencies.iter().position(|v| *v == id.module) else {
                continue;
            };

            let form = FormId::new(index as u8, id.form.form());
            let local_records = match file.file.children(kind, form).await {
                Ok(v) => v,
                // EOF means the file does not contain the record to query
                // children of.
                Err(Error::EndOfFile) => continue,
                Err(err) => return Err(err),
            };

            records.reserve(local_records.len());

            for local in local_records {
                let record = GlobalRecord {
                    id: RecordId {
                        module: file.id,
                        form: local.id,
                    },
                    local,
                };

                records.push(record);
            }
        }

        Ok(records)
    }

    /// Resolves a plugin-local [`FormId`] into a global [`RecordId`].
    ///
    /// The `origin` is the [`RecordId`] where the [`FormId`] was defined.
    pub fn resolve_form(&self, form: FormId, origin: RecordId) -> Option<RecordId> {
        self.resolve_form_from_module(form, origin.module)
    }

    fn resolve_form_from_module(&self, form: FormId, module: ModuleId) -> Option<RecordId> {
        let index = *self.plugins.get(module.0.get() as usize)?;
        let plugin = &self.files[index as usize];

        // It seems that out of bounds plugin indices reference the plugin
        // itself, i.e. clamped at the number of dependencies+1.
        let plugin_index = usize::min(
            usize::from(form.plugin()),
            plugin.dependencies.len().saturating_sub(1),
        );

        let module = *plugin.dependencies.get(plugin_index)?;
        Some(RecordId { form, module })
    }

    /// Resolves a filename to a [`ModuleId`]. Returns `None` if no file with the given name is
    /// loaded.
    pub fn resolve_file(&self, file: &Lowercase<str>) -> Option<ModuleId> {
        self.plugin_file_names.get(file).copied()
    }

    /// Returns the [`RecordId`] that is the immediate parent of the given `id`.
    ///
    /// If the given `id` has no parent `None` is returned.
    pub fn parent(&self, id: RecordId) -> Option<RecordId> {
        let index = self.plugins[id.module.0.get() as usize] as usize;
        let parent = self.files[index].file.parent(id.form)?;
        self.resolve_form(parent, id)
    }
}

#[derive(Debug)]
struct PluginFile {
    file: EspFile<File>,
    id: ModuleId,
    dependencies: Box<[ModuleId]>,
}

#[derive(Clone, Debug)]
pub struct GlobalRecord {
    pub id: RecordId,
    pub local: Record,
}

pub type Error = openfo4_esp::Error;
