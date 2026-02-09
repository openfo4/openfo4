use hashbrown::HashMap;
use openfo4_core::record::RecordId;
use openfo4_script::runtime::ObjectAddr;
use slab::Slab;

use crate::systems::scripting::ScriptInstance;
use crate::systems::scripting::loader::ScriptObjectId;

#[derive(Clone, Debug, Default)]
pub struct ScriptObjects {
    // FIXME: Remove this hack
    pub(super) instances: Slab<ObjectInfo>,
    addrs: HashMap<ObjectAddr, (ScriptObjectId, String)>,
    records: HashMap<RecordId, ScriptObjectId>,
}

impl ScriptObjects {
    pub fn insert(
        &mut self,
        scripts: HashMap<String, ScriptInstance>,
        record: Option<RecordId>,
    ) -> ScriptObjectId {
        let id = ScriptObjectId(self.instances.vacant_key());

        for (name, script) in &scripts {
            self.addrs.insert(script.object.addr(), (id, name.clone()));
        }

        if let Some(record) = record {
            debug_assert!(!self.records.contains_key(&record));
            self.records.insert(record, id);
        }

        let id2 = self.instances.insert(ObjectInfo { scripts, record });
        debug_assert_eq!(id.0, id2);
        id
    }

    pub fn get_by_id(&self, id: ScriptObjectId) -> Option<ScriptsRef<'_>> {
        let info = self.instances.get(id.0)?;
        Some(ScriptsRef { info, id })
    }

    pub fn get_by_addr(&self, addr: ObjectAddr) -> Option<ScriptRef<'_>> {
        let (id, name) = self.addrs.get(&addr)?;
        let info = self.instances.get(id.0).unwrap();
        let instance = info.scripts.get(name).unwrap();

        Some(ScriptRef {
            instance,
            record: info.record,
            name,
            id: *id,
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
        let (name, instance) = self.info.scripts.get_key_value(name)?;
        Some(ScriptRef {
            instance: instance,
            record: self.info.record,
            name,
            id: self.id,
        })
    }

    pub fn len(&self) -> usize {
        self.info.scripts.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ScriptRef<'a>> + '_ {
        self.info.scripts.iter().map(|(k, v)| ScriptRef {
            instance: v,
            record: self.info.record,
            name: &k,
            id: self.id,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ScriptRef<'a> {
    instance: &'a ScriptInstance,
    name: &'a str,
    record: Option<RecordId>,
    id: ScriptObjectId,
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
pub(super) struct ObjectInfo {
    pub(super) scripts: HashMap<String, ScriptInstance>,
    record: Option<RecordId>,
}
