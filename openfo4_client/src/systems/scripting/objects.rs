use hashbrown::HashMap;
use openfo4_core::record::RecordId;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::ObjectAddr;
use slab::Slab;

use crate::systems::scripting::ScriptInstance;
use crate::systems::scripting::loader::ScriptObjectId;

#[derive(Clone, Debug, Default)]
pub struct ScriptObjects {
    instances: Slab<ObjectInfo>,
    addrs: HashMap<ObjectAddr, (ScriptObjectId, usize, String)>,
    records: HashMap<RecordId, ScriptObjectId>,
}

impl ScriptObjects {
    pub fn insert(
        &mut self,
        engine: &Engine,
        scripts: Vec<ScriptInstance>,
        record: Option<RecordId>,
    ) -> ScriptObjectId {
        let id = ScriptObjectId(self.instances.vacant_key());

        if let Some(record) = record {
            debug_assert!(!self.records.contains_key(&record));
            self.records.insert(record, id);
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

            self.addrs.insert(
                script.object.addr(),
                (id, index, info.name.unwrap_or_default().into_inner()),
            );
        }

        let id2 = self.instances.insert(ObjectInfo {
            scripts,
            class_names,
            record,
        });
        debug_assert_eq!(id.0, id2);
        id
    }

    pub fn get_by_id(&self, id: ScriptObjectId) -> Option<ScriptsRef<'_>> {
        let info = self.instances.get(id.0)?;
        Some(ScriptsRef { info, id })
    }

    pub fn get_by_addr(&self, addr: ObjectAddr) -> Option<ScriptRef<'_>> {
        let (id, index, name) = self.addrs.get(&addr)?;
        let info = self.instances.get(id.0).unwrap();
        let instance = &info.scripts[*index];

        Some(ScriptRef {
            instance,
            record: info.record,
            id: *id,
            name,
        })
    }

    pub fn get_by_record(&self, id: RecordId) -> Option<ScriptsRef<'_>> {
        let id = *self.records.get(&id)?;
        let info = self.instances.get(id.0).unwrap();
        Some(ScriptsRef { info, id })
    }

    pub fn iter(&self) -> impl Iterator<Item = ScriptsRef<'_>> + '_ {
        self.instances.iter().map(|(id, info)| ScriptsRef {
            info,
            id: ScriptObjectId(id),
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ScriptsRef<'a> {
    info: &'a ObjectInfo,
    id: ScriptObjectId,
}

impl<'a> ScriptsRef<'a> {
    pub fn id(&self) -> ScriptObjectId {
        self.id
    }

    pub fn get(&self, name: &str) -> Option<ScriptRef<'a>> {
        let (name, index) = self.info.class_names.get_key_value(&name.to_lowercase())?;
        let instance = &self.info.scripts[*index];

        Some(ScriptRef {
            instance: instance,
            record: self.info.record,
            id: self.id,
            name,
        })
    }

    pub fn len(&self) -> usize {
        self.info.class_names.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ScriptRef<'a>> + '_ {
        self.info.class_names.iter().map(|(name, index)| {
            let instance = &self.info.scripts[*index];

            ScriptRef {
                instance,
                record: self.info.record,
                id: self.id,
                name,
            }
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ScriptRef<'a> {
    instance: &'a ScriptInstance,
    record: Option<RecordId>,
    id: ScriptObjectId,
    name: &'a str,
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
        self.name
    }

    pub fn get(&self) -> &ScriptInstance {
        self.instance
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
}
