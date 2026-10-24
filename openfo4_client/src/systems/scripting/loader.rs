use std::sync::Arc;

use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use openfo4_common::string::Lowercase;
use openfo4_core::record::RecordId;
use openfo4_esp::vmad::{Script, Value};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_script::compiler::resources::TypeInfo;
use openfo4_script::engine::{Engine, Instance};
use openfo4_script::runtime::ObjectAddr;
use openfo4_script::runtime::object::ObjectHandle;
use parking_lot::Mutex;

use crate::state::esp::EspLoader;
use crate::systems::scripting::object_builder::build_script_object;
use crate::systems::scripting::object_data::{ObjectData, load_record_object_data};
use crate::systems::scripting::objects::ScriptObjects;
use crate::systems::scripting::registration::{Event, EventKind, EventSender, RegistrationSet};
use crate::systems::scripting::{COMPILER_OPTIONS, ScriptInstance};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScriptObjectId(pub(super) usize);

/// The script object associated with an instanced record.
///
/// Note that a single `ScriptObject` may represent more than one script if more than one script
/// was attached to a record.
#[derive(Clone, Debug)]
pub struct ScriptObject(pub(super) Arc<ScriptObjectInner>);

impl ScriptObject {
    pub fn id(&self) -> ScriptObjectId {
        self.0.id
    }
}

#[derive(Debug)]
pub(super) struct ScriptObjectInner {
    base: Option<RecordId>,
    pub(super) id: ScriptObjectId,
    // TODO: Cleanup when this is dropped.
}

#[derive(Debug)]
pub struct ScriptLoader<T = Arc<EspLoader>> {
    pub(super) engine: Arc<Engine>,
    pub(super) esp_loader: T,
    load_events: Mutex<HashMap<RecordId, LoadEvent>>,
    cache: CompilationCache,
    pub(super) objects: Arc<ScriptObjects>,
    pub(super) registration: Arc<RegistrationSet>,
}

impl<T> ScriptLoader<T> {
    pub(super) fn new(
        engine: Arc<Engine>,
        esp_loader: T,
        objects: Arc<ScriptObjects>,
        registration: Arc<RegistrationSet>,
    ) -> Self {
        Self {
            engine,
            esp_loader,
            load_events: Default::default(),
            cache: Default::default(),
            objects,
            registration,
        }
    }
}

impl<T> ScriptLoader<T>
where
    T: RecordLoader,
{
    /// Loads the script object attached to a record.
    ///
    /// Returns `None` if no script is attached to the record.
    #[tracing::instrument(name = "ScriptLoader::load", skip_all)]
    pub async fn load(&self, record: RecordId) -> Option<LoadResult> {
        tracing::debug!("loading script for {:?}", record);

        let res = self
            .load_in(record, RecordInstantiationKind::BaseObject)
            .await;

        tracing::trace!("script for {:?} is {:?}", record, res.object.id());

        Some(res)
    }

    /// Creates a new instance of the given base record.
    #[tracing::instrument(name = "ScriptLoader::load_instance", skip_all)]
    pub async fn load_instance(&self, record: RecordId) -> Option<ScriptObject> {
        tracing::debug!("loading script for {:?}", record);

        let res = self
            .load_in(record, RecordInstantiationKind::Instance)
            .await;

        tracing::trace!("script for {:?} is {:?}", record, res.object.id());

        Some(res.object)
    }

    async fn load_in(&self, root: RecordId, kind: RecordInstantiationKind) -> LoadResult {
        if let Some(scripts) = self.objects.get_by_record(root)
        // FIXME: We should still cache default instances here
        // and clone the object for each call.
        && kind.is_base_object()
        {
            return LoadResult {
                object: ScriptObject(Arc::new(ScriptObjectInner {
                    base: Some(root),
                    id: scripts.id(),
                })),
                // FIXME: What should we do here?
                aliases: HashMap::new(),
            };
        }

        let mut root_id = None;

        let mut records = HashMap::new();
        let mut record_aliases = HashMap::new();

        let mut listeners = HashMap::new();

        let mut queue = vec![root];
        while let Some(record) = queue.pop() {
            // We want our own instance, don't sync with other tasks loading
            // the same instance.
            let is_instance_root = kind.is_instance() && record == root;

            if self.objects.get_by_record(record).is_some()
                || records.contains_key(&record)
                || listeners.contains_key(&record)
            {
                if !is_instance_root {
                    continue;
                }
            }

            if !is_instance_root {
                // Try to synchronize with another task that is already loading this record.
                // If another task is already loading the record, we skip it locally and
                // let the other task provide it to us later, otherwise we setup an event
                // for another task to synchronize to.
                let mut events = self.load_events.lock();

                // The other thread may have finished loading between our last check and locking.
                // In this cast the entry will be vacant, but we MUST not create another object.
                // We must do this check while holding the lock to prevent the other thread from
                // removing the event that we synchronize with otherwise.
                if self.objects.get_by_record(record).is_some() {
                    continue;
                }

                match events.entry(record) {
                    Entry::Occupied(event) => {
                        debug_assert!(!records.contains_key(&record));
                        debug_assert!(!listeners.contains_key(&record));

                        listeners.insert(record, event.get().listen());
                        continue;
                    }
                    Entry::Vacant(event) => {
                        event.insert(LoadEvent::default());
                    }
                }
            }

            tracing::debug!("loading attached scripts for record {:?}", record);

            let kind = match kind {
                RecordInstantiationKind::BaseObject => RecordInstantiationKind::BaseObject,
                RecordInstantiationKind::Instance if record == root => {
                    RecordInstantiationKind::Instance
                }
                RecordInstantiationKind::Instance => RecordInstantiationKind::BaseObject,
            };

            let Some(mut loaded_rec) = self
                .esp_loader
                .load_record(record, &self.engine, kind, &self.cache)
                .await
            else {
                if !is_instance_root {
                    let mut events = self.load_events.lock();
                    let event = events.remove(&record).unwrap();
                    event.loaded.notify(event.loaded.total_listeners());
                    event.built.notify(event.built.total_listeners());
                    event.ready.notify(event.ready.total_listeners());
                }

                continue;
            };

            debug_assert!(!listeners.contains_key(&record));

            // Avoid requeuing the same record again, doing so may result
            // in the loop never terminating.
            loaded_rec.depends.remove(&record);

            queue.extend(loaded_rec.depends);
            records.insert(record, (loaded_rec.scripts, loaded_rec.data));
            record_aliases.extend(loaded_rec.aliases);
        }

        // - We load the the record ourselves OR
        // - We sync with another thread that is loading the record OR
        // - The record is already loaded.
        debug_assert!(
            records.contains_key(&root)
                || listeners.contains_key(&root)
                || self.objects.get_by_record(root).is_some()
        );

        let local_records: Vec<_> = records.keys().copied().collect();

        let mut partial_object_addrs = HashMap::new();
        for (id, (scripts, _)) in &records {
            let mut addrs = HashMap::new();
            for (_, script) in scripts {
                let addr = script.builder.addr();

                // We can also use any supertype of our type.
                for id in &script.info.classes {
                    let info = self.engine.get_type_by_id(*id).unwrap();
                    let info = info.info();
                    if let Some(name) = info.name.clone() {
                        addrs.insert(name, addr);
                    }
                }
            }
            partial_object_addrs.insert(*id, addrs);
        }

        let mut partial_alias_addrs = HashMap::new();
        for (key, alias) in &record_aliases {
            let mut addrs = HashMap::new();
            for (_, script) in alias {
                let addr = script.builder.addr();

                for id in &script.info.classes {
                    let info = self.engine.get_type_by_id(*id).unwrap();
                    let info = info.info();
                    if let Some(name) = info.name.clone() {
                        addrs.insert(name, addr);
                    }
                }
            }
            partial_alias_addrs.insert(*key, addrs);
        }

        // At this point we are ready to build our objects, but we must wait for
        // all objects that we synchronized with to become ready.
        // Before this happens we MUST make our local addresses available,
        // otherwise we may deadlock, since the other task may also wait for our
        // local addresses.
        {
            let mut events = self.load_events.lock();

            for (record, addrs) in &partial_object_addrs {
                // We don't sync instance scripts.
                let is_instance_root = kind.is_instance() && *record == root;
                if is_instance_root {
                    continue;
                }

                debug_assert!(records.contains_key(record));
                debug_assert!(!listeners.contains_key(record));

                // This key must still exist, only we are allowed to remove it.
                let event = events.get_mut(record).unwrap();
                event.addrs = Some(addrs.clone());

                // NOTE: The number of listeners cannot change, because we hold
                // an exclusive lock.
                event.loaded.notify(event.loaded.total_listeners());
            }
        }

        for (_, listener) in &mut listeners {
            (&mut listener.loaded).await;
        }

        {
            let mut events = self.load_events.lock();

            for (record, _) in &listeners {
                // We don't sync instance scripts.
                let is_instance_root = kind.is_instance() && *record == root;
                if is_instance_root {
                    continue;
                }

                // If the event is already removed, this means that the loading
                // has finished and the object is already fully loaded and available
                // in the object pool.
                let Some(event) = events.get_mut(record) else {
                    continue;
                };

                let addrs = event.addrs.as_ref().unwrap();
                partial_object_addrs.insert(*record, addrs.clone());
            }
        }

        let mut built_objects = Vec::new();

        for (record, (scripts, data)) in records {
            let mut final_instances = Vec::new();

            for (name, mut object) in scripts {
                register_static_events(&object, &self.registration);

                if let Some(script) = &object.script {
                    tracing::debug!("instantiating script {:?} (record {:?})", name, record);

                    let (_, warnings) = build_script_object(
                        |form| self.esp_loader.resolve_form(form, record).unwrap(),
                        &self.engine,
                        script,
                        &self.objects,
                        &partial_object_addrs,
                        &partial_alias_addrs,
                        &mut object.builder,
                        record,
                    );

                    for warning in warnings {
                        tracing::warn!(
                            "warning while building script {} for object {:?}: {}",
                            &script.name,
                            record,
                            warning,
                        );
                    }
                }

                let layout = self.engine.get_type(&name).unwrap();
                final_instances.push(ScriptInstance {
                    instance: object.instance,
                    layout: layout.id(),
                    object: object.builder,
                });
            }

            let rid = match kind {
                RecordInstantiationKind::BaseObject => Some(record),
                // Always register dependencies as base objects.
                RecordInstantiationKind::Instance if record != root => Some(record),
                RecordInstantiationKind::Instance => None,
            };

            if rid.is_some() {
                debug_assert!(self.objects.get_by_record(record).is_none());
            }

            let is_root = record == root;

            built_objects.push((final_instances, rid, None, is_root, data));
        }

        let mut compiled_aliases = HashMap::new();
        for (key, scripts) in record_aliases {
            let mut final_instances = Vec::new();

            for (name, mut object) in scripts {
                register_static_events(&object, &self.registration);

                if let Some(script) = &object.script {
                    tracing::debug!("instantiating alias script {:?} for {:?}", name, key);

                    let (_, warnings) = build_script_object(
                        |form| self.esp_loader.resolve_form(form, key.accessor).unwrap(),
                        &self.engine,
                        script,
                        &self.objects,
                        &partial_object_addrs,
                        &partial_alias_addrs,
                        &mut object.builder,
                        key.accessor,
                    );

                    for warning in warnings {
                        tracing::warn!(
                            "warning while building script {} for alias {:?}: {}",
                            &script.name,
                            key,
                            warning,
                        );
                    }
                }

                let layout = self.engine.get_type(&name).unwrap();
                final_instances.push(ScriptInstance {
                    instance: object.instance,
                    layout: layout.id(),
                    object: object.builder,
                });
            }

            built_objects.push((final_instances, None, Some(key), false, ObjectData::None));
        }

        {
            let mut events = self.load_events.lock();

            for record in &local_records {
                // We don't sync instance scripts.
                let is_instance_root = kind.is_instance() && *record == root;
                if is_instance_root {
                    continue;
                }

                debug_assert!(!listeners.contains_key(record));

                let event = events.get_mut(record).unwrap();
                event.loaded.notify(event.loaded.total_listeners());
                event.built.notify(event.built.total_listeners());
            }
        }

        for (_, listener) in &mut listeners {
            (&mut listener.built).await;
        }

        // Once we insert the new objects into the object pool, they are allowed
        // to immediately be used by another thread, so must wait until the objects
        // are fully built.
        for (instances, record, alias_key, is_root, data) in built_objects {
            let id = self.objects.insert(&self.engine, instances, record, data);

            if is_root {
                root_id = Some(id);
            }

            if let Some(key) = alias_key {
                compiled_aliases.insert(
                    key,
                    ScriptObject(Arc::new(ScriptObjectInner { base: None, id })),
                );
            }
        }

        {
            let mut events = self.load_events.lock();

            // Signal all other tasks that have started waiting for our objects after
            // we already signaled for the first time. Not doing so would cause them
            // to deadlock.
            for record in local_records {
                // We don't sync instance scripts.
                let is_instance_root = kind.is_instance() && record == root;
                if is_instance_root {
                    continue;
                }

                // We are the owner of the event, so only we are allowed to remove it.
                let event = events.remove(&record).unwrap();

                // The listener count cannot change, because we hold an exclusive lock.
                event.loaded.notify(event.loaded.total_listeners());
                event.built.notify(event.built.total_listeners());
                event.ready.notify(event.ready.total_listeners());
            }
        }

        let root_id = match root_id {
            Some(id) => id,
            // We did not build the root object ourselves.
            // This can happen when two threads load the same object at the
            // same time and the other thread "wins". At any time there must
            // only exist one object for each record, so must reuse the other
            // threads object.
            None => {
                debug_assert!(kind.is_base_object());

                match self.objects.get_by_record(root) {
                    Some(scripts) => scripts.id(),
                    None => {
                        // We need to wait until the thread that has our object has
                        // inserted the object into the pool and assigned it an ID.
                        let listen = listeners.get_mut(&root).unwrap();
                        (&mut listen.ready).await;

                        // FIXME: Once we implement object cleanup, we need to restart
                        // the loading process on failure here.
                        self.objects.get_by_record(root).unwrap().id()
                    }
                }
            }
        };

        LoadResult {
            object: ScriptObject(Arc::new(ScriptObjectInner {
                base: Some(root),
                id: root_id,
            })),
            aliases: compiled_aliases,
        }
    }
}

/// The result of a loading the script for a record.
#[derive(Clone, Debug)]
pub struct LoadResult {
    /// The script object of the requested record.
    pub object: ScriptObject,
    /// Alias objects that we build together with the requested objects.
    ///
    /// Note that these objects are only compiled and don't point to any entities yet.
    /// You must manually assign them to a real entity using the [`AliasSet`].
    ///
    /// [`AliasSet`]: super::AliasSet
    pub aliases: HashMap<AliasKey, ScriptObject>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordInstantiationKind {
    /// The base object itself.
    BaseObject,
    /// A runtime instance of the base object.
    Instance,
}

impl RecordInstantiationKind {
    const fn is_base_object(&self) -> bool {
        matches!(self, Self::BaseObject)
    }

    const fn is_instance(&self) -> bool {
        matches!(self, Self::Instance)
    }
}

#[derive(Debug, Default)]
struct LoadEvent {
    addrs: Option<HashMap<Lowercase<String>, ObjectAddr>>,
    /// Event triggered when the object has received an [`ObjectAddr`].
    loaded: event_listener::Event,
    /// Event triggered when the object has fully completed building.
    ///
    /// Accessing the [`ObjectAddr`] of the object before this event was received is invalid.
    built: event_listener::Event,
    // /// Number of references keeping this event alive, including the publisher.
    // ref_count: NonZeroUsize,
    ready: event_listener::Event,
}

impl LoadEvent {
    fn listen(&self) -> LoadEventListener {
        LoadEventListener {
            loaded: self.loaded.listen(),
            built: self.built.listen(),
            ready: self.ready.listen(),
        }
    }
}

#[derive(Debug)]
struct LoadEventListener {
    loaded: event_listener::EventListener,
    built: event_listener::EventListener,
    ready: event_listener::EventListener,
}

#[derive(Debug, Default)]
pub struct CompilationCache {
    classes: Mutex<HashMap<String, Instance>>,
}

impl CompilationCache {
    async fn get_or_compile(&self, engine: &Engine, class: &str) -> Instance {
        {
            if let Some(instance) = self.classes.lock().get(class) {
                return instance.clone();
            }
        }

        let instance = engine.compile(class, &COMPILER_OPTIONS).await.unwrap();

        let mut instances = self.classes.lock();

        // If two threads compile the same class twice and overwrite
        // each other that is okay.
        // Engine::compile returns the same instance for both
        // compilations.
        instances.insert(class.to_owned(), instance.clone());

        instance
    }
}

async fn load_alias_type(esp_loader: &EspLoader, id: RecordId, alias_id: u32) -> Option<AliasType> {
    let RecordData::Quest(data) = esp_loader
        .lookup(Some(RecordKind::Quest), id)
        .await
        .unwrap()
        .local
        .data
    else {
        todo!();
    };

    for alias in &data.aliases.references {
        if alias.id == alias_id {
            return Some(AliasType::Reference);
        }
    }

    for alias in &data.aliases.locations {
        if alias.id == alias_id {
            return Some(AliasType::Location);
        }
    }

    for alias in &data.aliases.ref_collections {
        if alias.id == alias_id {
            return Some(AliasType::RefCollection);
        }
    }

    None
}

async fn load_record(
    esp_loader: &EspLoader,
    record: RecordId,
    engine: &Engine,
    kind: RecordInstantiationKind,
    compiled_classes: &CompilationCache,
) -> Option<LoadedRecord> {
    let Ok(record) = esp_loader.lookup(None, record).await.inspect_err(|err| {
        tracing::warn!("failed to load {:?}: {}", record, err);
    }) else {
        return None;
    };

    let mut aliases = HashMap::new();

    let record_scripts = match record.local.data.clone() {
        RecordData::Activator(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Armor(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Book(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Container(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Door(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::FLOR(data) => data.vmad.map(|v| v.scripts),
        RecordData::Furniture(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Ingredient(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Key(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Light(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::MagicEffect(data) => data.virtual_machine_adpater.map(|v| v.scripts),
        RecordData::MiscObject(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::MSTT(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Npc(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Note(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Package(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Perk(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Quest(data) => {
            for alias in &data.aliases.references {
                aliases.insert(
                    (record.id, alias.id),
                    AliasInfo {
                        ty: AliasType::Reference,
                        scripts: Vec::new(),
                    },
                );
            }

            for alias in &data.aliases.locations {
                aliases.insert(
                    (record.id, alias.id),
                    AliasInfo {
                        ty: AliasType::Location,
                        scripts: Vec::new(),
                    },
                );
            }

            for alias in &data.aliases.ref_collections {
                aliases.insert(
                    (record.id, alias.id),
                    AliasInfo {
                        ty: AliasType::RefCollection,
                        scripts: Vec::new(),
                    },
                );
            }

            if let Some(vmad) = &data.virtual_machine_adapter {
                if let Some(frags) = &vmad.fragments {
                    for alias in &frags.aliases {
                        let alias_id = alias.object.alias.unwrap();

                        let id = esp_loader
                            .resolve_form(alias.object.form, record.id)
                            .unwrap();

                        match aliases.get_mut(&(id, alias_id as u32)) {
                            Some(a) => {
                                a.scripts.extend(alias.scripts.clone());
                            }
                            None => {
                                let Some(ty) =
                                    load_alias_type(esp_loader, id, alias_id as u32).await
                                else {
                                    tracing::warn!(
                                        "cannot found alias {} in quest {}, skipping",
                                        alias_id,
                                        id,
                                    );
                                    continue;
                                };

                                aliases.insert(
                                    (id, alias_id as u32),
                                    AliasInfo {
                                        ty,
                                        scripts: alias.scripts.clone(),
                                    },
                                );
                            }
                        }
                    }
                }
            }

            if let Some(vmad) = data.virtual_machine_adapter {
                let mut scripts = vmad.scripts;

                if let Some(frags) = vmad.fragments
                    && let Some(script) = frags.script
                {
                    scripts.push(script);
                }

                Some(scripts)
            } else {
                None
            }
        }
        RecordData::STAT(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::TACT(data) => data.vmad.map(|v| v.scripts),
        RecordData::Terminal(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Tree(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::Weapon(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::PlacedObject(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        RecordData::PlacedNpc(data) => data.virtual_machine_adapter.map(|v| v.scripts),
        _ => None,
    };

    // Find all other forms that the scripts reference.
    // Note that scripts may reference the same form multiple times, which
    // is why we use a HashSet instead of a Vec or directly pushing to queue.
    let mut depends = HashSet::new();

    // FIXME: Avoid cloning here.
    let aliases2 = aliases.clone();
    let scripts_to_iter: Vec<&Script> = record_scripts
        .iter()
        .flat_map(|v| v)
        .chain(aliases2.values().map(|a| a.scripts.iter()).flatten())
        .collect();
    for script in scripts_to_iter {
        let mut values_queue: Vec<_> = script.properties.iter().map(|v| &v.value).collect();

        while let Some(value) = values_queue.pop() {
            match value {
                Value::Object(value) => {
                    if value.form.is_null() {
                        continue;
                    }

                    let id = esp_loader.resolve_form(value.form, record.id).unwrap();

                    // Even if an alias with another owner record is referenced here,
                    // we still create a new object here.
                    // The "local" alias (defined in this record) may still attach additional
                    // scripts that are not attached to the alias in the owner record.
                    // FIXME: We could in theory skip this, if no scripts are attached
                    // locally, right?
                    if let Some(alias) = &value.alias {
                        if !aliases.contains_key(&(id, *alias as u32)) {
                            let Some(ty) = load_alias_type(esp_loader, id, *alias as u32).await
                            else {
                                tracing::warn!(
                                    "cannot found alias {} in quest {}, skipping",
                                    alias,
                                    id,
                                );
                                continue;
                            };

                            aliases.insert(
                                (id, *alias as u32),
                                AliasInfo {
                                    ty,
                                    scripts: Vec::new(),
                                },
                            );
                        }
                    } else {
                        depends.insert(id);
                    }
                }
                Value::Struct(fields, _) => {
                    for field in fields {
                        values_queue.push(&field.value);
                    }
                }
                Value::Array(values, _) => {
                    values_queue.extend(values);
                }
                _ => {}
            }
        }
    }

    let mut scripts = HashMap::new();

    // We only want to create a new instance of the root object.
    // The dependencies must remain regular base objects.
    let base_class_name = if kind.is_instance() {
        record_instance_script_name(&record.local.data).unwrap()
    } else {
        record_script_name(&record.local.data).unwrap()
    };

    let instance = compiled_classes
        .get_or_compile(engine, base_class_name)
        .await;
    let base_class = engine.get_type(base_class_name).unwrap();
    let builder = engine.object_pool().insert(base_class.id());

    for script in record_scripts.into_iter().flat_map(|v| v) {
        let instance = compiled_classes.get_or_compile(engine, &script.name).await;
        let layout = engine.get_type(&script.name).unwrap();
        let builder = engine.object_pool().insert(layout.id());

        let info = layout.info().clone();
        if !info.classes.contains(&base_class.id()) {
            tracing::debug!(
                "attached script {} on record {} does not inherit from base class {}, skipping",
                script.name,
                record.id,
                base_class_name,
            );
            continue;
        }

        scripts.insert(
            script.name.clone(),
            PartialObject {
                instance: instance.clone(),
                builder,
                script: Some(script),
                info,
            },
        );
    }

    // If we have at least one script loaded we don't need to explicitly attach
    // the base script, the base script is already included in the loaded script.
    // All attached scripts must inherit from the base class.
    if scripts.is_empty() {
        scripts.insert(
            base_class_name.to_owned(),
            PartialObject {
                instance: instance.clone(),
                builder,
                script: None,
                info: base_class.info().clone(),
            },
        );
    }

    let mut record_aliases = HashMap::new();

    // Build alias objects
    for ((alias_owner, alias_id), alias) in aliases {
        let mut scripts = HashMap::new();

        let base_class_name = alias.ty.script_name();
        let instance = compiled_classes
            .get_or_compile(engine, base_class_name)
            .await;
        let layout = engine.get_type(base_class_name).unwrap();
        let builder = engine.object_pool().insert(layout.id());

        scripts.insert(
            base_class_name.to_owned(),
            PartialObject {
                instance: instance.clone(),
                builder,
                script: None,
                info: layout.info().clone(),
            },
        );

        for script in alias.scripts.into_iter() {
            let instance = compiled_classes.get_or_compile(engine, &script.name).await;
            let layout = engine.get_type(&script.name).unwrap();
            let builder = engine.object_pool().insert(layout.id());

            scripts.insert(
                script.name.clone(),
                PartialObject {
                    instance: instance.clone(),
                    builder,
                    script: Some(script),
                    info: layout.info().clone(),
                },
            );
        }

        record_aliases.insert(
            AliasKey {
                accessor: record.id,
                owner: alias_owner,
                id: alias_id,
            },
            scripts,
        );
    }

    Some(LoadedRecord {
        depends,
        scripts,
        aliases: record_aliases,
        data: load_record_object_data(esp_loader, &record),
    })
}

#[derive(Debug)]
pub struct LoadedRecord {
    /// Records that this record depends upon.
    depends: HashSet<RecordId>,
    scripts: HashMap<String, PartialObject>,
    aliases: HashMap<AliasKey, HashMap<String, PartialObject>>,
    data: ObjectData,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AliasKey {
    /// The object that accesses the alias.
    pub accessor: RecordId,
    /// The owner quest of the alias.
    pub owner: RecordId,
    /// The ID of the alias in the owner quest.
    pub id: u32,
}

#[derive(Clone, Debug)]
struct AliasInfo {
    ty: AliasType,
    scripts: Vec<Script>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum AliasType {
    Reference,
    Location,
    RefCollection,
}

impl AliasType {
    const fn script_name(&self) -> &'static str {
        match self {
            Self::Reference => "ReferenceAlias",
            Self::Location => "LocationAlias",
            Self::RefCollection => "RefCollectionAlias",
        }
    }
}

fn register_static_events(object: &PartialObject, registration: &RegistrationSet) {
    // FIXME: We should validate that the base object type is correct
    // and the function signature matches what we expect before we
    // register the event.
    // We don't want to send OnTriggerEnter for Quests for example.
    for kind in [
        // Quest
        EventKind::QuestInit,
        EventKind::QuestShutdown,
        EventKind::QuestStageSet,
        // Alias
        EventKind::AliasInit,
        EventKind::AliasReset,
        EventKind::AliasShutdown,
        // Trigger
        EventKind::TriggerEnter,
        EventKind::TriggerLeave,
        // Inventory
        EventKind::ItemEquipped,
        EventKind::ItemUnequipped,
        EventKind::Equipped,
        EventKind::Unequipped,
        // Actor "Health"
        EventKind::Cripple,
        EventKind::Death,
        EventKind::Dying,
        EventKind::EnterBleedout,
        // Cell
        EventKind::CellAttach,
        EventKind::CellDetach,
        EventKind::CellLoad,
        // Animations
        EventKind::Open,
        EventKind::Close,
    ] {
        let name = kind.func_name();

        if object.instance.get_func(name).is_none() {
            continue;
        }

        registration.register(
            object.builder.addr(),
            Event {
                kind,
                sender: EventSender::Object(object.builder.addr()),
                limit: None,
            },
        );
    }
}

#[derive(Debug)]
struct PartialObject {
    instance: Instance,
    builder: ObjectHandle,
    script: Option<Script>,
    info: TypeInfo,
}

fn record_script_name(data: &RecordData) -> Option<&'static str> {
    match data {
        RecordData::Action(_) => Some("Action"),
        RecordData::Activator(_) => Some("Activator"),
        RecordData::ActorValueInfo(_) => Some("ActorValue"),
        RecordData::Ammo(_) => Some("Ammo"),
        RecordData::Armor(_) => Some("Armor"),
        RecordData::AssocationType(_) => Some("AssociationType"),
        RecordData::AudioCategorySnapshot(_) => Some("SoundCategorySnapshot"),
        RecordData::Book(_) => Some("Book"),
        RecordData::CameraShot(_) => Some("CameraShot"),
        RecordData::Cell(_) => Some("Cell"),
        RecordData::Class(_) => Some("Class"),
        RecordData::CombatStyle(_) => Some("CombatStyle"),
        RecordData::Component(_) => Some("Component"),
        RecordData::ConstructibleObject(_) => Some("ConstructibleObject"),
        RecordData::Container(_) => Some("Container"),
        RecordData::Door(_) => Some("Door"),
        RecordData::EffectShader(_) => Some("EffectShader"),
        RecordData::EncounterZone(_) => Some("EncounterZone"),
        RecordData::Explosion(_) => Some("Explosion"),
        RecordData::Faction(_) => Some("Faction"),
        RecordData::FLOR(_) => Some("Flora"),
        RecordData::FormList(_) => Some("FormList"),
        RecordData::Furniture(_) => Some("Furniture"),
        RecordData::GLOB(_) => Some("GlobalVariable"),
        RecordData::Hazard(_) => Some("Hazard"),
        RecordData::HeadPart(_) => Some("HeadPart"),
        RecordData::Animation(_) => Some("Idle"),
        RecordData::IdleMarker(_) => Some("IdleMarker"),
        RecordData::ImageSpaceAdapter(_) => Some("ImageSpaceModifier"),
        RecordData::ImpactDataSet(_) => Some("ImpactDataSet"),
        RecordData::Ingestible(_) => Some("Potion"),
        RecordData::InstanceNamingRules(_) => Some("InstanceNamingRules"),
        RecordData::Key(_) => Some("Key"),
        RecordData::Keyword(_) => Some("Keyword"),
        RecordData::LeveledItem(_) => Some("LeveledItem"),
        RecordData::LeveledActor(_) => Some("LeveledActor"),
        RecordData::Light(_) => Some("Light"),
        RecordData::Location(_) => Some("Location"),
        RecordData::LocationReferenceType(_) => Some("LocationRefType"),
        RecordData::MagicEffect(_) => Some("MagicEffect"),
        RecordData::Message(_) => Some("Message"),
        RecordData::MiscObject(_) => Some("MiscObject"),
        RecordData::MSTT(_) => Some("MovableStatic"),
        RecordData::MusicType(_) => Some("MusicType"),
        RecordData::NavMesh(_) => Some("Form"),
        RecordData::Note(_) => Some("Holotape"),
        RecordData::Npc(_) => Some("ActorBase"),
        RecordData::ObjectModification(_) => Some("ObjectMod"),
        RecordData::Outfit(_) => Some("Outfit"),
        RecordData::Package(_) => Some("Package"),
        RecordData::Perk(_) => Some("Perk"),
        RecordData::Projectile(_) => Some("Projectile"),
        RecordData::Quest(_) => Some("Quest"),
        RecordData::Race(_) => Some("Race"),
        RecordData::ShaderParticleGeometry(_) => Some("ShaderParticleGeometry"),
        RecordData::SoundDescriptor(_) => Some("Sound"),
        RecordData::SoundCategory(_) => Some("SoundCategory"),
        RecordData::SoundOutputModel(_) => Some("OutputModel"),
        RecordData::SPEL(_) => Some("Spell"),
        RecordData::STAT(_) => Some("Static"),
        RecordData::TACT(_) => Some("TalkingActivator"),
        RecordData::Terminal(_) => Some("Terminal"),
        RecordData::TextureSet(_) => Some("TextureSet"),
        RecordData::VisualEffect(_) => Some("VisualEffect"),
        RecordData::VoiceType(_) => Some("VoiceType"),
        RecordData::Weapon(_) => Some("Weapon"),
        RecordData::Weather(_) => Some("Weather"),
        RecordData::Worldspace(_) => Some("WorldSpace"),
        RecordData::Land(_) => Some("Form"),
        RecordData::PlacedObject(_) => Some("ObjectReference"),
        RecordData::PlacedNpc(_) => Some("Actor"),
        RecordData::PlacedHazard(_) => Some("ObjectReference"),
        RecordData::PlacedMissile(_) => Some("ObjectReference"),
        RecordData::PlacedGrenade(_) => Some("ObjectReference"),
        RecordData::DialogueTopic(_) => Some("Topic"),
        RecordData::Scene(_) => Some("Scene"),
        RecordData::ReferenceGroup(_) => Some("Form"),
        x => todo!("{:?}", x),
    }
}

fn record_instance_script_name(data: &RecordData) -> Option<&'static str> {
    match data {
        RecordData::Ammo(_) => Some("ObjectReference"),
        RecordData::Armor(_) => Some("ObjectReference"),
        RecordData::Book(_) => Some("ObjectReference"),
        RecordData::Component(_) => Some("ObjectReference"),
        RecordData::Container(_) => Some("ObjectReference"),
        RecordData::Door(_) => Some("ObjectReference"),
        RecordData::FLOR(_) => Some("ObjectReference"),
        RecordData::Furniture(_) => Some("ObjectReference"),
        RecordData::Ingestible(_) => Some("ObjectReference"),
        RecordData::Key(_) => Some("ObjectReference"),
        RecordData::MiscObject(_) => Some("ObjectReference"),
        RecordData::MSTT(_) => Some("ObjectReference"),
        RecordData::Note(_) => Some("ObjectReference"),
        RecordData::Npc(_) => Some("Actor"),
        RecordData::Weapon(_) => Some("ObjectReference"),
        _ => todo!(),
    }
}

pub trait RecordLoader {
    async fn load_record(
        &self,
        record: RecordId,
        engine: &Engine,
        kind: RecordInstantiationKind,
        compiled_classes: &CompilationCache,
    ) -> Option<LoadedRecord>;

    fn resolve_form(&self, form: FormId, origin: RecordId) -> Option<RecordId>;
}

impl RecordLoader for Arc<EspLoader> {
    fn load_record(
        &self,
        record: RecordId,
        engine: &Engine,
        kind: RecordInstantiationKind,
        compiled_classes: &CompilationCache,
    ) -> impl Future<Output = Option<LoadedRecord>> {
        load_record(self, record, engine, kind, compiled_classes)
    }

    fn resolve_form(&self, form: FormId, origin: RecordId) -> Option<RecordId> {
        self.as_ref().resolve_form(form, origin)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_lite::future::block_on;
    use hashbrown::{HashMap, HashSet};
    use openfo4_core::record::RecordId;
    use openfo4_esp::FormId;
    use openfo4_pex::{Class, Module, State};
    use openfo4_script::compiler::CompilerOptions;
    use openfo4_script::engine::Engine;

    use crate::state::hardcoded::HARDCODED_MODULE_ID;
    use crate::systems::scripting::loader::{
        CompilationCache, LoadedRecord, PartialObject, RecordInstantiationKind, RecordLoader,
        ScriptLoader,
    };
    use crate::systems::scripting::object_data::ObjectData;
    use crate::systems::scripting::objects::ScriptObjects;
    use crate::systems::scripting::registration::RegistrationSet;

    #[derive(Clone, Debug)]
    struct Loader {
        deps: HashMap<RecordId, HashSet<RecordId>>,
    }

    impl RecordLoader for Loader {
        async fn load_record(
            &self,
            record: RecordId,
            engine: &Engine,
            _kind: RecordInstantiationKind,
            compiled_classes: &CompilationCache,
        ) -> Option<LoadedRecord> {
            let instance = compiled_classes.get_or_compile(engine, "MyClass").await;
            let ty = engine.get_type("MyClass").unwrap();

            let builder = engine.object_pool().insert(ty.id());
            let info = ty.info().clone();

            Some(LoadedRecord {
                depends: self.deps.get(&record).unwrap().clone(),
                scripts: [(
                    "MyClass".to_owned(),
                    PartialObject {
                        instance,
                        builder,
                        script: None,
                        info,
                    },
                )]
                .into_iter()
                .collect(),
                aliases: HashMap::new(),
                data: ObjectData::None,
            })
        }

        fn resolve_form(&self, _form: FormId, _origin: RecordId) -> Option<RecordId> {
            unreachable!()
        }
    }

    fn build_loader<T>(t: T) -> ScriptLoader<T> {
        let mut engine = Engine::new(CompilerOptions::default());

        engine.register(&Module {
            classes: vec![Class {
                name: "MyClass".to_owned(),
                parent: "".to_owned(),
                states: vec![State {
                    name: "".to_owned(),
                    functions: vec![],
                }],
                structs: Vec::new(),
                default_state: "".to_owned(),
                properties: Vec::new(),
                variables: Vec::new(),
            }],
        });

        let engine = Arc::new(engine);
        let objects = Arc::new(ScriptObjects::default());
        let registration = Arc::new(RegistrationSet::default());

        ScriptLoader::new(engine, t, objects, registration)
    }

    fn make_rid(id: u32) -> RecordId {
        RecordId::new(HARDCODED_MODULE_ID, FormId(id))
    }

    #[test]
    fn script_loader_parallel_load() {
        let loader = Arc::new(build_loader(Loader {
            deps: [(make_rid(1), HashSet::new())].into_iter().collect(),
        }));

        let loadera = loader.clone();
        let a = std::thread::spawn(move || block_on(loadera.load(make_rid(1))));
        let loaderb = loader.clone();
        let b = std::thread::spawn(move || block_on(loaderb.load(make_rid(1))));

        let a = a.join().unwrap().unwrap().object;
        let b = b.join().unwrap().unwrap().object;
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn script_loader_cyclic_dependency() {
        let loader = Arc::new(build_loader(Loader {
            deps: [
                (make_rid(1), [make_rid(2)].into_iter().collect()),
                (make_rid(2), [make_rid(1)].into_iter().collect()),
            ]
            .into_iter()
            .collect(),
        }));

        let loadera = loader.clone();
        let a = std::thread::spawn(move || block_on(loadera.load(make_rid(1))));
        let loaderb = loader.clone();
        let b = std::thread::spawn(move || block_on(loaderb.load(make_rid(2))));

        let a = a.join().unwrap().unwrap().object;
        let b = b.join().unwrap().unwrap().object;
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn script_loader_self_cycle_instance() {
        let loader = Arc::new(build_loader(Loader {
            deps: [(make_rid(1), [make_rid(1)].into_iter().collect())]
                .into_iter()
                .collect(),
        }));

        let loadera = loader.clone();
        block_on(loadera.load_instance(make_rid(1)));
    }
}
