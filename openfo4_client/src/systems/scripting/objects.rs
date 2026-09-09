use std::marker::PhantomData;
use std::sync::Arc;

use hashbrown::HashMap;
use openfo4_core::record::RecordId;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::ObjectAddr;
use parking_lot::Mutex;
use slab::Slab;

use crate::systems::scripting::ScriptInstance;
use crate::systems::scripting::loader::ScriptObjectId;
use crate::systems::scripting::state::ObjectData;

#[derive(Debug, Default)]
pub struct ScriptObjects {
    instances: Mutex<Slab<Arc<ObjectInfo>>>,
    addrs: Mutex<HashMap<ObjectAddr, (ScriptObjectId, usize, String)>>,
    records: Mutex<HashMap<RecordId, ScriptObjectId>>,
}

impl ScriptObjects {
    pub fn insert(
        &self,
        engine: &Engine,
        scripts: Vec<ScriptInstance>,
        record: Option<RecordId>,
        data: ObjectData,
    ) -> ScriptObjectId {
        let mut instances = self.instances.lock();
        let mut addrs = self.addrs.lock();
        let mut records = self.records.lock();

        let entry = instances.vacant_entry();

        let id = ScriptObjectId(entry.key());

        if let Some(record) = record {
            debug_assert!(!records.contains_key(&record));
            records.insert(record, id);
        }

        let mut class_names = HashMap::new();
        for (index, script) in scripts.iter().enumerate() {
            let info = engine.type_info(script.layout);

            for class in &info.classes {
                let info = engine.type_info(*class);
                if let Some(name) = info.name {
                    class_names.insert(name.into_inner(), index);
                }
            }

            addrs.insert(
                script.object.addr(),
                (id, index, info.name.unwrap_or_default().into_inner()),
            );
        }

        entry.insert(Arc::new(ObjectInfo {
            scripts,
            class_names,
            record,
            data,
        }));

        id
    }

    pub fn get_by_id(&self, id: ScriptObjectId) -> Option<ScriptsRef<'_>> {
        let instances = self.instances.lock();

        let info = instances.get(id.0)?;

        Some(ScriptsRef {
            info: Arc::clone(info),
            id,
            _owner: PhantomData,
        })
    }

    pub fn get_by_addr(&self, addr: ObjectAddr) -> Option<ScriptRef<'_>> {
        let instances = self.instances.lock();
        let addrs = self.addrs.lock();

        let (id, index, name) = addrs.get(&addr)?;
        let info = instances.get(id.0).unwrap();

        Some(ScriptRef {
            info: Arc::clone(info),
            index: *index,
            record: info.record,
            id: *id,
            name: name.clone(),
            _owner: PhantomData,
        })
    }

    pub fn get_by_record(&self, id: RecordId) -> Option<ScriptsRef<'_>> {
        let instances = self.instances.lock();
        let records = self.records.lock();

        let id = *records.get(&id)?;
        let info = instances.get(id.0).unwrap();

        Some(ScriptsRef {
            info: Arc::clone(info),
            id,
            _owner: PhantomData,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ScriptsRef<'a> {
    info: Arc<ObjectInfo>,
    id: ScriptObjectId,
    _owner: PhantomData<&'a ()>,
}

impl<'a> ScriptsRef<'a> {
    pub fn id(&self) -> ScriptObjectId {
        self.id
    }

    pub fn get(&self, name: &str) -> Option<ScriptRef<'a>> {
        let (name, index) = self.info.class_names.get_key_value(&name.to_lowercase())?;

        Some(ScriptRef {
            info: Arc::clone(&self.info),
            index: *index,
            record: self.info.record,
            id: self.id,
            name: name.clone(),
            _owner: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        self.info.class_names.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ScriptRef<'a>> + '_ {
        self.info.class_names.iter().map(|(name, index)| ScriptRef {
            info: Arc::clone(&self.info),
            index: *index,
            record: self.info.record,
            id: self.id,
            name: name.clone(),
            _owner: PhantomData,
        })
    }

    /// Returns the extra data attached to this instance.
    pub fn data(&self) -> &ObjectData {
        &self.info.data
    }
}

#[derive(Clone, Debug)]
pub struct ScriptRef<'a> {
    info: Arc<ObjectInfo>,
    index: usize,
    record: Option<RecordId>,
    id: ScriptObjectId,
    name: String,
    _owner: PhantomData<&'a ()>,
}

impl<'a> ScriptRef<'a> {
    /// Returns the [`RecordId`] associated with this script instance.
    ///
    /// May be `None` if the script refers to a runtime-created object.
    pub fn record(&self) -> Option<RecordId> {
        self.record
    }

    pub fn id(&self) -> ScriptObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get(&self) -> &ScriptInstance {
        &self.info.scripts[self.index]
    }
}

#[derive(Clone, Debug)]
struct ObjectInfo {
    scripts: Vec<ScriptInstance>,
    /// Maps class names to script instances.
    ///
    /// Note that it is possible for multiple scripts with the same class hierarchy
    /// to exist, but in that case which one we return is not defined.
    class_names: HashMap<String, usize>,
    record: Option<RecordId>,
    data: ObjectData,
}
