use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::num::NonZeroU32;

use async_lock::RwLock;
use hashbrown::HashMap;
use openfo4_common::collections::lru::LruCache;
use openfo4_common::collections::slab::Slab;
use openfo4_common::string::Lowercase;
use openfo4_core::record::{ModuleId, RecordId};
use openfo4_esp::bstr::{BStr, BString, ByteSlice};
use openfo4_esp::records::Tes4;
use openfo4_esp::{ErrorKind, EspFile, FormId, Record, RecordChildren, RecordData, RecordKind};
use openfo4_io::fs::{AsyncReadAt, File};
use thiserror::Error;

#[derive(Debug)]
pub struct EspLoaderBuilder<T = EspFile<File>> {
    files: Slab<usize, FileEntry<T>>,
    hardcoded: HashMap<RecordId, Record>,
    hardcoded_parents: HashMap<RecordId, RecordId>,
}

impl<T> EspLoaderBuilder<T> {
    pub fn new() -> Self {
        Self {
            files: Slab::new(),
            hardcoded: HashMap::new(),
            hardcoded_parents: HashMap::new(),
        }
    }

    /// Adds a new [`EspSource`] to the loader.
    pub fn push<S>(&mut self, name: S, source: T)
    where
        S: AsRef<BStr>,
    {
        self.files.insert(FileEntry {
            name: Lowercase::new(name.as_ref()),
            source,
        });
    }

    pub fn set_hardcoded(&mut self, hardcoded: HashMap<RecordId, Record>) {
        self.hardcoded = hardcoded;
    }

    pub fn set_hardcoded_parents(&mut self, parents: HashMap<RecordId, RecordId>) {
        self.hardcoded_parents = parents;
    }

    pub async fn finalize(mut self) -> EspLoader<T>
    where
        T: EspSource,
    {
        let mut dependents = HashMap::<_, Vec<_>>::with_capacity(self.files.len());
        let mut num_dependencies = HashMap::with_capacity(self.files.len());
        let mut queue = BinaryHeap::new();

        let mut plugin_names = HashMap::new();
        for (index, file) in &self.files {
            let header = file.source.header().await.unwrap();

            if plugin_names.contains_key(&file.name) {
                tracing::warn!(
                    "duplicate plugin name {}, second plugin will be skipped",
                    file.name
                );
                continue;
            }

            plugin_names.insert(&file.name, index);

            if header.master_files.is_empty() {
                queue.push(Reverse(index));
                continue;
            }

            for master in &*header.master_files {
                let master = Lowercase::new(master.file_name.as_bstr());
                dependents.entry(master).or_default().push(index);
            }

            num_dependencies.insert(index, header.master_files.len());
        }

        drop(plugin_names);

        let mut next_module_id = NonZeroU32::MIN;
        let mut plugins = Vec::<PluginFile<T>>::new();
        let mut filenames = HashMap::new();

        while let Some(index) = queue.pop() {
            let file = self.files.remove(index.0).unwrap();
            // FIXME: We should cache the header from earlier.
            let header = file.source.header().await.unwrap();

            let id = ModuleId(next_module_id);
            next_module_id = next_module_id.checked_add(1).unwrap();

            let mut dependencies = Vec::with_capacity(header.master_files.len() + 1);
            for master in &*header.master_files {
                let id = filenames[&Lowercase::new(master.file_name.as_bstr())];
                dependencies.push(id);
            }

            dependencies.push(id);
            debug_assert_eq!(dependencies.len(), header.master_files.len() + 1);

            tracing::debug!("plugin {} has ID {:?}", file.name, id);

            // Mark the forms that we are overriding in our dependencies.
            for form in header.overriden_forms.unwrap_or_default() {
                let Some(parent) = dependencies.get(usize::from(form.plugin())) else {
                    continue;
                };

                let parent = &mut plugins[(parent.0.get() - 1) as usize];
                parent.overriden.insert(form.form(), id);
            }

            filenames.insert(file.name.clone(), id);
            plugins.push(PluginFile {
                id,
                source: file.source,
                filename: file.name.clone(),
                dependencies: dependencies.into_boxed_slice(),
                overriden: HashMap::new(),
            });

            if let Some(dependents) = dependents.remove(&file.name) {
                for index in dependents {
                    let num_deps = num_dependencies.get_mut(&index).unwrap();
                    *num_deps -= 1;
                    if *num_deps == 0 {
                        queue.push(Reverse(index));
                    }
                }
            }
        }

        // TODO: Some files could not be loaded.
        assert!(dependents.is_empty());

        EspLoader {
            plugins,
            filenames,
            cache: RwLock::new(LruCache::new(65_536)),
            hardcoded: self.hardcoded,
            hardcoded_parents: self.hardcoded_parents,
        }
    }
}

#[derive(Debug)]
struct FileEntry<T> {
    name: Lowercase<BString>,
    source: T,
}

#[derive(Debug)]
pub struct EspLoader<T = EspFile<File>> {
    /// The plugin files in topological order, i.e. master plugins are found before their users.
    plugins: Vec<PluginFile<T>>,
    filenames: HashMap<Lowercase<BString>, ModuleId>,
    cache: RwLock<LruCache<RecordId, GlobalRecord>>,
    hardcoded: HashMap<RecordId, Record>,
    hardcoded_parents: HashMap<RecordId, RecordId>,
}

impl<T> EspLoader<T>
where
    T: EspSource,
{
    /// Looks up the record with the given [`RecordId`].
    ///
    /// If not record with the given [`RecordId`] exists, an [`Error`] is returned.
    #[tracing::instrument(name = "EspLoader::lookup", skip_all)]
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

        let index = id.module.0.get() - 1;
        let Some(mut plugin) = self.plugins.get(index as usize) else {
            return Err(Error::NotFound(id))?;
        };

        // The plugin index refering to the file itself is the index
        // after all dependencies.
        debug_assert_eq!(*plugin.dependencies.last().unwrap(), id.module);

        if let Some(parent) = plugin.overriden.get(&id.form.form()) {
            // Guaranteed to be in bounds.
            let index = parent.0.get() - 1;
            plugin = &self.plugins[index as usize];
        }

        self.lookup_in_plugin(plugin, kind, id).await
    }

    /// Looks up the given [`RecordId`] within in the given plugin.
    async fn lookup_in_plugin(
        &self,
        plugin: &PluginFile<T>,
        kind: Option<RecordKind>,
        id: RecordId,
    ) -> Result<GlobalRecord, Error> {
        let Some(index) = plugin.dependencies.iter().position(|v| *v == id.module) else {
            return Err(Error::NotFound(id));
        };

        let form = FormId::new(index as u8, id.form.form());

        let record = match plugin.source.lookup(kind, form).await {
            Ok(record) => record,
            Err(err) if err.kind() == ErrorKind::NotFound => return Err(Error::NotFound(id)),
            Err(err) => return Err(Error::Esp(err.into())),
        };

        let record = GlobalRecord {
            id: RecordId {
                module: plugin.id,
                form: record.id,
            },
            local: record,
        };

        self.cache.write().await.insert(id, record.clone());

        return Ok(record);
    }

    /// Returns all records matching the given query.
    ///
    /// Note that this function must iterate over all records of all plugins and is therefore an
    /// expensive operation.
    pub async fn lookup_all(&self, kind: Option<RecordKind>) -> Result<Vec<GlobalRecord>, Error> {
        let mut records = Vec::new();

        for (id, record) in &self.hardcoded {
            if kind.is_none_or(|kind| kind == record.data.kind()) {
                records.push(GlobalRecord {
                    id: *id,
                    local: record.clone(),
                });
            }
        }

        for plugin in &self.plugins {
            let local_records = plugin.source.lookup_all(kind).await?;

            records.reserve(local_records.len());
            for local in local_records {
                // Only visit records that are actually defined in this source.
                // Records that are only overwritten or referenced are returned
                // when we visit the other sources.
                if usize::from(local.id.plugin()) != plugin.dependencies.len() - 1 {
                    continue;
                }

                let id = RecordId {
                    module: plugin.id,
                    form: local.id,
                };

                let Some(parent) = plugin.overriden.get(&id.form.form()) else {
                    records.push(GlobalRecord { id, local });
                    continue;
                };

                // Guaranteed to be in bounds.
                let index = parent.0.get() - 1;
                let plugin = &self.plugins[index as usize];

                let record = self.lookup_in_plugin(plugin, kind, id).await?;
                records.push(record);
            }
        }

        Ok(records)
    }

    #[tracing::instrument(name = "EspLoader::children", skip_all)]
    pub async fn children(
        &self,
        kind: RecordChildren,
        id: RecordId,
    ) -> Result<Vec<GlobalRecord>, Error> {
        let mut records = Vec::new();

        for plugin in &self.plugins {
            // Resolve the parent to local plugin index.
            let Some(index) = plugin.dependencies.iter().position(|v| *v == id.module) else {
                continue;
            };

            let form = FormId::new(index as u8, id.form.form());
            let local_records = match plugin.source.children(kind, form).await {
                Ok(v) => v,
                Err(err) if err.kind() == ErrorKind::NotFound => continue,
                Err(err) => return Err(err.into()),
            };

            records.reserve(local_records.len());

            for local in local_records {
                // Only visit records that are actually defined in this source.
                // Records that are only overwritten or referenced are returned
                // when we visit the other sources.
                if usize::from(local.id.plugin()) != plugin.dependencies.len() - 1 {
                    continue;
                }

                let id = RecordId {
                    module: plugin.id,
                    form: local.id,
                };

                let Some(parent) = plugin.overriden.get(&id.form.form()) else {
                    records.push(GlobalRecord { id, local });
                    continue;
                };

                // Guaranteed to be in bounds.
                let index = parent.0.get() - 1;
                let plugin = &self.plugins[index as usize];

                let record = self.lookup_in_plugin(plugin, None, id).await?;
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Resolves a plugin-local [`FormId`] into a global [`RecordId`].
    ///
    /// The `origin` is the [`RecordId`] where the [`FormId`] was defined.
    pub fn resolve_form(&self, form: FormId, origin: RecordId) -> Option<RecordId> {
        // Hardcoded forms can only reference other hardcoded forms from the
        // same plugin.
        if self.hardcoded.contains_key(&origin) && form.plugin() == 0 {
            return Some(RecordId::new(origin.module, form));
        }

        self.resolve_form_from_module(form, origin.module)
    }

    fn resolve_form_from_module(&self, form: FormId, module: ModuleId) -> Option<RecordId> {
        let index = module.0.get() - 1;
        let plugin = self.plugins.get(index as usize)?;

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
    pub fn resolve_file<S>(&self, file: S) -> Option<ModuleId>
    where
        S: AsRef<Lowercase<BStr>>,
    {
        self.filenames.get(file.as_ref()).copied()
    }

    /// Returns the [`RecordId`] that is the immediate parent of the given `id`.
    ///
    /// If the given `id` has no parent `None` is returned.
    pub fn parent(&self, id: RecordId) -> Option<RecordId> {
        if let Some(parent) = self.hardcoded_parents.get(&id) {
            return Some(*parent);
        }

        let index = id.module.0.get() - 1;
        let parent = self.plugins.get(index as usize)?.source.parent(id.form)?;
        self.resolve_form(parent, id)
    }

    /// Returns the filename of a [`ModuleId`].
    pub fn file(&self, id: ModuleId) -> Option<&Lowercase<BStr>> {
        let index = id.0.get() - 1;
        self.plugins
            .get(index as usize)
            .map(|plugin| plugin.filename.as_bstr())
    }
}

#[derive(Debug)]
struct PluginFile<T> {
    source: T,
    id: ModuleId,
    dependencies: Box<[ModuleId]>,
    filename: Lowercase<BString>,
    overriden: HashMap<u32, ModuleId>,
}

#[derive(Clone, Debug)]
pub struct GlobalRecord {
    pub id: RecordId,
    pub local: Record,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("record {0} does not exist")]
    NotFound(RecordId),
    #[error(transparent)]
    Esp(#[from] openfo4_esp::Error),
}

pub trait EspSource: Send + Sync + 'static {
    fn header(&self) -> impl Future<Output = Result<Tes4, openfo4_esp::Error>> {
        async move {
            let record = self.lookup(Some(RecordKind::Tes4), FormId::NULL).await?;
            match record.data {
                RecordData::Tes4(data) => Ok(data),
                _ => unreachable!(),
            }
        }
    }

    fn lookup(
        &self,
        kind: Option<RecordKind>,
        id: FormId,
    ) -> impl Future<Output = Result<Record, openfo4_esp::Error>>;

    fn lookup_all(
        &self,
        kind: Option<RecordKind>,
    ) -> impl Future<Output = Result<Vec<Record>, openfo4_esp::Error>>;

    fn children(
        &self,
        kind: RecordChildren,
        id: FormId,
    ) -> impl Future<Output = Result<Vec<Record>, openfo4_esp::Error>>;

    fn parent(&self, id: FormId) -> Option<FormId>;
}

impl<R> EspSource for EspFile<R>
where
    R: AsyncReadAt + 'static,
{
    async fn lookup(
        &self,
        kind: Option<RecordKind>,
        id: FormId,
    ) -> Result<Record, openfo4_esp::Error> {
        self.lookup(kind, id).await
    }

    async fn lookup_all(
        &self,
        kind: Option<RecordKind>,
    ) -> Result<Vec<Record>, openfo4_esp::Error> {
        self.lookup_all(kind).collect()
    }

    async fn children(
        &self,
        kind: RecordChildren,
        id: FormId,
    ) -> Result<Vec<Record>, openfo4_esp::Error> {
        self.children(kind, id).await
    }

    fn parent(&self, id: FormId) -> Option<FormId> {
        self.parent(id)
    }
}

#[cfg(test)]
mod tests {
    use futures_lite::future::block_on;
    use hashbrown::HashMap;
    use openfo4_core::record::RecordId;
    use openfo4_esp::bstr::BString;
    use openfo4_esp::records::misc_object::MiscObject;
    use openfo4_esp::records::{Cell, MasterFile, PlacedObject, Tes4};
    use openfo4_esp::{
        Error, FormId, Record, RecordChildren, RecordData, RecordHeaderFlags, RecordKind,
    };

    use crate::state::esp::{EspLoaderBuilder, EspSource};

    #[derive(Clone, Debug)]
    struct StaticSource {
        header: Tes4,
        records: HashMap<FormId, Record>,
        children: HashMap<FormId, Vec<FormId>>,
        next_id: u32,
    }

    impl StaticSource {
        fn add_record<T>(&mut self, data: T) -> FormId
        where
            T: Into<RecordData>,
        {
            let num_deps = self.header.master_files.len() as u8;
            let id = FormId::new(num_deps, self.next_id);
            self.next_id += 1;

            self.records.insert(
                id,
                Record {
                    id,
                    flags: RecordHeaderFlags::default(),
                    data: data.into(),
                },
            );

            id
        }

        fn get_parent_record(&self, parent: &str, id: FormId) -> FormId {
            let index = self
                .header
                .master_files
                .iter()
                .position(|master| master.file_name == parent)
                .unwrap();

            FormId::new(index as u8, id.0)
        }

        fn add_parent_reference<T>(&mut self, parent: &str, id: FormId, data: T) -> FormId
        where
            T: Into<RecordData>,
        {
            let index = self
                .header
                .master_files
                .iter()
                .position(|master| master.file_name == parent)
                .unwrap();

            let id = FormId::new(index as u8, id.form());
            self.next_id += 1;

            self.records.insert(
                id,
                Record {
                    id,
                    flags: RecordHeaderFlags::default(),
                    data: data.into(),
                },
            );

            id
        }

        fn add_parent_overwrite<T>(&mut self, parent: &str, id: FormId, data: T) -> FormId
        where
            T: Into<RecordData>,
        {
            let index = self
                .header
                .master_files
                .iter()
                .position(|master| master.file_name == parent)
                .unwrap();

            let id = FormId::new(index as u8, id.form());

            self.records.insert(
                id,
                Record {
                    id,
                    flags: RecordHeaderFlags::default(),
                    data: data.into(),
                },
            );
            self.header.overriden_forms.get_or_insert_default().push(id);

            id
        }
    }

    impl EspSource for StaticSource {
        async fn header(&self) -> Result<Tes4, Error> {
            Ok(self.header.clone())
        }

        async fn lookup(&self, _kind: Option<RecordKind>, id: FormId) -> Result<Record, Error> {
            self.records
                .get(&id)
                .cloned()
                .ok_or_else(|| Error::new_not_found(id))
        }

        async fn lookup_all(&self, kind: Option<RecordKind>) -> Result<Vec<Record>, Error> {
            Ok(self
                .records
                .values()
                .filter(|record| kind.is_none_or(|kind| record.data.kind() == kind))
                .cloned()
                .collect())
        }

        async fn children(&self, _kind: RecordChildren, id: FormId) -> Result<Vec<Record>, Error> {
            let mut children = Vec::new();
            if let Some(ids) = self.children.get(&id) {
                for id in ids {
                    children.push(self.records[id].clone());
                }
            }

            Ok(children)
        }

        fn parent(&self, _id: FormId) -> Option<FormId> {
            unreachable!()
        }
    }

    fn make_source(masters: &[&str]) -> StaticSource {
        let mut header = Tes4::default();
        for master in masters {
            let mut m = MasterFile::default();
            m.file_name = BString::new(master.as_bytes().to_vec());
            header.master_files.push(m);
        }

        StaticSource {
            header,
            records: HashMap::new(),
            next_id: 0,
            children: HashMap::new(),
        }
    }

    #[test]
    fn esp_builder_preserve_order() {
        let mut builder = EspLoaderBuilder::new();
        builder.push("a", make_source(&[]));
        builder.push("b", make_source(&["a"]));
        builder.push("c", make_source(&["a"]));
        builder.push("d", make_source(&[]));

        let loader = block_on(builder.finalize());
        assert_eq!(loader.plugins[0].filename.as_inner(), "a");
        assert_eq!(loader.plugins[1].filename.as_inner(), "b");
        assert_eq!(loader.plugins[2].filename.as_inner(), "c");
        assert_eq!(loader.plugins[3].filename.as_inner(), "d");
    }

    #[test]
    fn esp_builder_dependencies() {
        let mut builder = EspLoaderBuilder::new();
        builder.push("a", make_source(&["b", "c"]));
        builder.push("d", make_source(&["c"]));
        builder.push("b", make_source(&[]));
        builder.push("c", make_source(&["b"]));

        let loader = block_on(builder.finalize());
        assert_eq!(loader.plugins[0].filename.as_inner(), "b");
        assert_eq!(loader.plugins[1].filename.as_inner(), "c");
        assert_eq!(loader.plugins[2].filename.as_inner(), "a");
        assert_eq!(loader.plugins[3].filename.as_inner(), "d");
    }

    #[test]
    fn esp_loader_lookup() {
        let mut builder = EspLoaderBuilder::new();

        let mut item = MiscObject::default();
        item.id = Some("Hello".into());

        let mut source = make_source(&[]);
        let id = source.add_record(item);
        builder.push("a", source);

        let loader = block_on(builder.finalize());

        let id = RecordId::new(loader.plugins[0].id, id);
        let record = block_on(loader.lookup(None, id)).unwrap();
        let data: MiscObject = record.local.data.try_into().unwrap();
        assert_eq!(data.id.unwrap(), "Hello");
    }

    #[test]
    fn esp_loader_lookup_from_dependency() {
        let mut builder = EspLoaderBuilder::new();

        let mut item = MiscObject::default();
        item.id = Some("Hello".into());

        let mut source = make_source(&[]);
        let id = source.add_record(item);
        builder.push("a", source);

        let source = make_source(&["a"]);
        let id = source.get_parent_record("a", id);
        builder.push("b", source);

        let loader = block_on(builder.finalize());

        let id = loader
            .resolve_form_from_module(id, loader.plugins[1].id)
            .unwrap();
        let record = block_on(loader.lookup(None, id)).unwrap();
        let data: MiscObject = record.local.data.try_into().unwrap();
        assert_eq!(data.id.unwrap(), "Hello");
    }

    #[test]
    fn esp_loader_lookup_overwritten() {
        let mut builder = EspLoaderBuilder::new();

        let mut item = MiscObject::default();
        item.id = Some("Hello".into());

        let mut source = make_source(&[]);
        let id = source.add_record(item);
        builder.push("a", source);

        let mut item = MiscObject::default();
        item.id = Some("World".into());

        let mut source = make_source(&["a"]);
        source.add_parent_overwrite("a", id, item);
        builder.push("b", source);

        let loader = block_on(builder.finalize());

        let id = RecordId::new(loader.plugins[0].id, id);
        let record = block_on(loader.lookup(None, id)).unwrap();
        let data: MiscObject = record.local.data.try_into().unwrap();
        assert_eq!(data.id.unwrap(), "World");
    }

    #[test]
    fn esp_loader_lookup_all_referenced() {
        let mut builder = EspLoaderBuilder::new();

        let mut item = MiscObject::default();
        item.id = Some("Hello".into());

        let mut source = make_source(&[]);
        let id = source.add_record(item);
        builder.push("a", source);

        let mut source = make_source(&["a"]);
        source.add_parent_reference("a", id, MiscObject::default());
        builder.push("b", source);

        let loader = block_on(builder.finalize());

        let mut records = block_on(loader.lookup_all(None)).unwrap();
        assert_eq!(records.len(), 1);
        let record = records.remove(0);
        assert_eq!(record.id, RecordId::new(loader.plugins[0].id, id));
        let data: MiscObject = record.local.data.try_into().unwrap();
        assert_eq!(data.id.unwrap(), "Hello");
    }

    #[test]
    fn esp_loader_lookup_all_overwritten() {
        let mut builder = EspLoaderBuilder::new();

        let mut item = MiscObject::default();
        item.id = Some("Hello".into());

        let mut source = make_source(&[]);
        let id = source.add_record(item);
        builder.push("a", source);

        let mut item = MiscObject::default();
        item.id = Some("World".into());

        let mut source = make_source(&["a"]);
        source.add_parent_overwrite("a", id, item);
        builder.push("b", source);

        let loader = block_on(builder.finalize());

        let mut records = block_on(loader.lookup_all(None)).unwrap();
        assert_eq!(records.len(), 1);
        let record = records.remove(0);
        assert_eq!(record.id, RecordId::new(loader.plugins[1].id, id));
        let data: MiscObject = record.local.data.try_into().unwrap();
        assert_eq!(data.id.unwrap(), "World");
    }

    #[test]
    fn esp_loader_children_referenced_parent() {
        let mut builder = EspLoaderBuilder::new();

        let cell = Cell::default();
        let refr1 = PlacedObject::default();

        let mut source = make_source(&[]);
        let cell = source.add_record(cell);
        let refr1 = source.add_record(refr1);
        source.children.insert(cell, vec![refr1]);
        builder.push("a", source);

        let refr2 = PlacedObject::default();

        let mut source = make_source(&["a"]);
        let cell_ref = source.add_parent_reference("a", cell, Cell::default());
        let refr2 = source.add_record(refr2);
        source.children.insert(cell_ref, vec![refr2]);
        builder.push("b", source);

        let loader = block_on(builder.finalize());

        let cell = loader
            .resolve_form_from_module(cell, loader.plugins[0].id)
            .unwrap();
        let records = block_on(loader.children(RecordChildren::Cells, cell)).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, RecordId::new(loader.plugins[0].id, refr1));
        assert_eq!(records[1].id, RecordId::new(loader.plugins[1].id, refr2));
    }

    #[test]
    fn esp_loader_children_overwritten() {
        let mut builder = EspLoaderBuilder::new();

        let cell = Cell::default();
        let refr1 = PlacedObject::default();

        let mut source = make_source(&[]);
        let cell = source.add_record(cell);
        let refr1 = source.add_record(refr1);
        source.children.insert(cell, vec![refr1]);
        builder.push("a", source);

        let mut source = make_source(&["a"]);
        let refr2 = source.add_parent_overwrite("a", refr1, PlacedObject::default());
        builder.push("b", source);

        let loader = block_on(builder.finalize());

        let cell = loader
            .resolve_form_from_module(cell, loader.plugins[0].id)
            .unwrap();
        let records = block_on(loader.children(RecordChildren::Cells, cell)).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, RecordId::new(loader.plugins[1].id, refr2));
    }
}
