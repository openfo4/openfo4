use std::sync::Arc;

use async_lock::Mutex;
use game_common::string::Lowercase;
use hashbrown::{HashMap, HashSet};
use openfo4_core::record::RecordId;
use openfo4_esp::vmad::{Property, Script, Value};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_script::compiler::resources::{PropertyType, TypeId};
use openfo4_script::engine::{Engine, Instance};
use openfo4_script::runtime::object::{IntoValue, ObjectArrayBuilder, ObjectBuilder};
use openfo4_script::runtime::{ObjectAddr, ObjectRef};

use crate::state::esp::EspLoader;
use crate::systems::scripting::classes::State;
use crate::systems::scripting::objects::ScriptObjects;
use crate::systems::scripting::registration::{Event, EventKind, EventSender};
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
pub struct ScriptLoader {
    pub(super) engine: Arc<Mutex<Engine>>,
    /// Cache of compiled classes.
    pub(super) classes: Mutex<HashMap<String, Instance>>,
    pub(super) esp_loader: Arc<EspLoader>,
    pub(super) state: Mutex<Option<State>>,
}

impl ScriptLoader {
    /// Loads the script object attached to a record.
    ///
    /// Returns `None` if no script is attached to the record.
    #[tracing::instrument(name = "ScriptLoader::load", skip_all)]
    pub async fn load(&self, record: RecordId) -> Option<LoadResult> {
        tracing::debug!("loading script for {:?}", record);

        let mut engine = self.engine.lock().await;
        let mut classes = self.classes.lock().await;
        let mut state = self.state.lock().await;

        let res = instantiate_form(
            &mut engine,
            &self.esp_loader,
            record,
            &mut classes,
            state.as_mut().unwrap(),
            RecordInstantiationKind::BaseObject,
        )
        .await?;

        tracing::trace!("script for {:?} is {:?}", record, res.object.id());

        Some(res)
    }

    /// Creates a new instance of the given base record.
    #[tracing::instrument(name = "ScriptLoader::load_instance", skip_all)]
    pub async fn load_instance(&self, record: RecordId) -> Option<ScriptObject> {
        tracing::debug!("loading script for {:?}", record);

        let mut engine = self.engine.lock().await;
        let mut classes = self.classes.lock().await;
        let mut state = self.state.lock().await;

        let res = instantiate_form(
            &mut engine,
            &self.esp_loader,
            record,
            &mut classes,
            state.as_mut().unwrap(),
            RecordInstantiationKind::Instance,
        )
        .await?;

        tracing::trace!("script for {:?} is {:?}", record, res.object.id());

        Some(res.object)
    }

    /// Returns a deep clone of the [`ScriptObject`].
    ///
    /// Only the top level properties are cloned.
    pub async fn clone_object(&self, object: &ScriptObject) -> ScriptObject {
        let engine = self.engine.lock().await;
        let mut state = self.state.lock().await;
        let state = state.as_mut().unwrap();

        let scripts = state.objects.get_by_id(object.id()).unwrap();

        let scripts = scripts
            .iter()
            .map(|script| ScriptInstance {
                instance: script.get().instance.clone(),
                layout: script.get().layout.clone(),
                stacks: script.get().stacks.clone(),
                object: unsafe {
                    engine
                        .object_pool()
                        .clone_object(&script.get().object)
                        .finalize()
                },
            })
            .collect();

        let id = state.objects.insert(&engine, scripts, None);

        ScriptObject(Arc::new(ScriptObjectInner {
            base: object.0.base,
            id,
        }))
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
enum RecordInstantiationKind {
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

async fn instantiate_form(
    engine: &mut Engine,
    esp_loader: &EspLoader,
    root: RecordId,
    compiled_classes: &mut HashMap<String, Instance>,
    state: &mut State,
    kind: RecordInstantiationKind,
) -> Option<LoadResult> {
    if let Some(scripts) = state.objects.get_by_record(root)
        // FIXME: We should still cache default instances here
        // and clone the object for each call.
        && kind.is_base_object()
    {
        return Some(LoadResult {
            object: ScriptObject(Arc::new(ScriptObjectInner {
                base: Some(root),
                id: scripts.id(),
            })),
            // FIXME: What should we do here?
            aliases: HashMap::new(),
        });
    }

    let mut root_id = None;

    let mut records = HashMap::new();
    let mut record_aliases = HashMap::new();

    let mut queue = vec![root];
    while let Some(record) = queue.pop() {
        if state.objects.get_by_record(record).is_some() {
            continue;
        }

        let record = esp_loader.lookup(None, record).await.unwrap();

        let mut aliases = HashMap::new();

        let record_scripts = match record.local.data.clone() {
            RecordData::Activator(data) => data.virtual_machine_adapter.map(|v| v.scripts),
            RecordData::Armor(data) => data.virtual_machine_adapter.map(|v| v.scripts),
            RecordData::Book(data) => data.virtual_machine_adapter.map(|v| v.scripts),
            RecordData::Container(data) => data.virtual_machine_adapter.map(|v| v.scripts),
            RecordData::Door(data) => data.virtual_machine_adapter.map(|v| v.scripts),
            RecordData::FLOR(data) => data.vmad.map(|v| v.scripts),
            RecordData::Furniture(data) => data.virtual_machine_adapter.map(|v| v.scripts),
            RecordData::INGR(data) => data.vmad.map(|v| v.scripts),
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
                                    let ty = load_alias_type(esp_loader, id, alias_id as u32).await;

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
                        let id = esp_loader.resolve_form(value.form, record.id).unwrap();
                        let is_loaded =
                            state.objects.get_by_record(id).is_some() || records.contains_key(&id);

                        // Even if an alias with another owner record is referenced here,
                        // we still create a new object here.
                        // The "local" alias (defined in this record) may still attach additional
                        // scripts that are not attached to the alias in the owner record.
                        // FIXME: We could in theory skip this, if no scripts are attached
                        // locally, right?
                        if let Some(alias) = &value.alias {
                            if !aliases.contains_key(&(id, *alias as u32)) {
                                let ty = load_alias_type(esp_loader, id, *alias as u32).await;
                                aliases.insert(
                                    (id, *alias as u32),
                                    AliasInfo {
                                        ty,
                                        scripts: Vec::new(),
                                    },
                                );
                            }
                        } else {
                            if is_loaded {
                                continue;
                            }

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

        for id in depends {
            debug_assert!(state.objects.get_by_record(id).is_none());
            debug_assert!(!records.contains_key(&id));
            queue.push(id);
        }

        let mut scripts = HashMap::new();

        // We only want to create a new instance of the root object.
        // The dependencies must remain regular base objects.
        let base_class_name = if kind.is_instance() && record.id == root {
            record_instance_script_name(&record.local.data).unwrap()
        } else {
            record_script_name(&record.local.data).unwrap()
        };

        let instance = compiled_classes
            .entry(base_class_name.to_owned())
            .or_insert_with(|| engine.compile(base_class_name, &COMPILER_OPTIONS));
        let layout = engine.get_type(base_class_name).unwrap();
        let builder = engine.object_pool().insert(layout.clone());

        scripts.insert(
            base_class_name.to_owned(),
            PartialObject {
                instance: instance.clone(),
                builder,
                script: None,
            },
        );

        for script in record_scripts.into_iter().flat_map(|v| v) {
            let instance = compiled_classes
                .entry(script.name.clone())
                .or_insert_with(|| engine.compile(&script.name, &COMPILER_OPTIONS));
            let layout = engine.get_type(&script.name).unwrap();
            let builder = engine.object_pool().insert(layout.clone());

            scripts.insert(
                script.name.clone(),
                PartialObject {
                    instance: instance.clone(),
                    builder,
                    script: Some(script),
                },
            );
        }

        records.insert(record.id, scripts);

        // Build alias objects
        for ((alias_owner, alias_id), alias) in aliases {
            let mut scripts = HashMap::new();

            let base_class_name = alias.ty.script_name();
            let instance = compiled_classes
                .entry(base_class_name.to_owned())
                .or_insert_with(|| engine.compile(base_class_name, &COMPILER_OPTIONS));
            let layout = engine.get_type(base_class_name).unwrap();
            let builder = engine.object_pool().insert(layout.clone());

            scripts.insert(
                base_class_name.to_owned(),
                PartialObject {
                    instance: instance.clone(),
                    builder,
                    script: None,
                },
            );

            for script in alias.scripts.into_iter() {
                let instance = compiled_classes
                    .entry(script.name.clone())
                    .or_insert_with(|| engine.compile(&script.name, &COMPILER_OPTIONS));
                let layout = engine.get_type(&script.name).unwrap();
                let builder = engine.object_pool().insert(layout.clone());

                scripts.insert(
                    script.name.clone(),
                    PartialObject {
                        instance: instance.clone(),
                        builder,
                        script: Some(script),
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
    }

    let mut partial_object_addrs = HashMap::new();
    for (id, scripts) in &records {
        let mut addrs = HashMap::new();
        for (name, script) in scripts {
            addrs.insert(name.clone(), script.builder.addr());
        }
        partial_object_addrs.insert(*id, addrs);
    }

    let mut partial_alias_addrs = HashMap::new();
    for (key, alias) in &record_aliases {
        let mut addrs = HashMap::new();
        for (name, script) in alias {
            addrs.insert(name.clone(), script.builder.addr());
        }
        partial_alias_addrs.insert(*key, addrs);
    }

    for (record, scripts) in records {
        let mut final_instances = Vec::new();

        for (name, mut object) in scripts {
            register_static_events(&object, state);

            if let Some(script) = &object.script {
                tracing::debug!("instantiating script {:?} (record {:?})", name, record);

                build_script_object(
                    |form| esp_loader.resolve_form(form, record).unwrap(),
                    engine,
                    script,
                    &state.objects,
                    &partial_object_addrs,
                    &partial_alias_addrs,
                    &mut object.builder,
                    record,
                );
            }

            let layout = engine.get_type(&name).unwrap();
            final_instances.push(ScriptInstance {
                instance: object.instance,
                layout: layout.clone(),
                object: object.builder.finalize(),
                stacks: HashMap::new(),
            });
        }

        let rid = match kind {
            RecordInstantiationKind::BaseObject => Some(record),
            // Always register dependencies as base objects.
            RecordInstantiationKind::Instance if record != root => Some(record),
            RecordInstantiationKind::Instance => None,
        };

        if rid.is_some() {
            debug_assert!(state.objects.get_by_record(record).is_none());
        }

        let id = state.objects.insert(engine, final_instances, rid);

        if record == root {
            root_id = Some(id);
        }
    }

    let mut compiled_aliases = HashMap::new();
    for (key, scripts) in record_aliases {
        let mut final_instances = Vec::new();

        for (name, mut object) in scripts {
            register_static_events(&object, state);

            if let Some(script) = &object.script {
                tracing::debug!("instantiating alias script {:?} for {:?}", name, key);

                build_script_object(
                    |form| esp_loader.resolve_form(form, key.accessor).unwrap(),
                    engine,
                    script,
                    &state.objects,
                    &partial_object_addrs,
                    &partial_alias_addrs,
                    &mut object.builder,
                    key.accessor,
                );
            }

            let layout = engine.get_type(&name).unwrap();
            final_instances.push(ScriptInstance {
                instance: object.instance,
                layout: layout.clone(),
                object: object.builder.finalize(),
                stacks: HashMap::new(),
            });
        }

        let id = state.objects.insert(engine, final_instances, None);
        compiled_aliases.insert(
            key,
            ScriptObject(Arc::new(ScriptObjectInner { base: None, id })),
        );
    }

    let root_id = root_id?;

    Some(LoadResult {
        object: ScriptObject(Arc::new(ScriptObjectInner {
            base: Some(root),
            id: root_id,
        })),
        aliases: compiled_aliases,
    })
}

async fn load_alias_type(esp_loader: &EspLoader, id: RecordId, alias_id: u32) -> AliasType {
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
            return AliasType::Reference;
        }
    }

    for alias in &data.aliases.locations {
        if alias.id == alias_id {
            return AliasType::Location;
        }
    }

    for alias in &data.aliases.ref_collections {
        if alias.id == alias_id {
            return AliasType::RefCollection;
        }
    }

    todo!("alias {} not defined in quest {:?}", alias_id, id)
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

fn register_static_events(object: &PartialObject, state: &mut State) {
    for (name, kind) in [
        ("OnTriggerEnter", EventKind::TriggerEnter),
        ("OnTriggerLeave", EventKind::TriggerLeave),
    ] {
        if object.instance.get_func(name).is_none() {
            continue;
        }

        state.registration.register(
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
    builder: ObjectBuilder,
    script: Option<Script>,
}

fn record_script_name(data: &RecordData) -> Option<&'static str> {
    match data {
        RecordData::Action(_) => Some("Action"),
        RecordData::Activator(_) => Some("Activator"),
        RecordData::ActorValueInfo(_) => Some("ActorValue"),
        RecordData::Ammo(_) => Some("Ammo"),
        RecordData::Armor(_) => Some("Armor"),
        RecordData::AssocationType(_) => Some("AssociationType"),
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
        RecordData::FACT(_) => Some("Faction"),
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
        RecordData::PlacedObject(_) => Some("ObjectReference"),
        RecordData::PlacedNpc(_) => Some("Actor"),
        RecordData::DialogueTopic(_) => Some("Topic"),
        RecordData::Scene(_) => Some("Scene"),
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

// FIXME: Use script layout reflection to determine we're actually setting values of the correct type.
fn build_script_object(
    resolve_form: impl Fn(FormId) -> RecordId,
    engine: &Engine,
    script: &Script,
    scripts: &ScriptObjects,
    partial_addrs: &HashMap<RecordId, HashMap<String, ObjectAddr>>,
    aliases: &HashMap<AliasKey, HashMap<String, ObjectAddr>>,
    builder: &mut ObjectBuilder,
    script_defined_in: RecordId,
) -> ObjectRef {
    let ty = engine.get_type(&script.name).unwrap();
    let layout = engine.type_info(ty);

    #[derive(Copy, Clone, Debug)]
    struct Entry<'a> {
        builder_index: usize,
        inner: EntryInner<'a>,
        pop_builder: bool,
    }

    #[derive(Copy, Clone, Debug)]
    enum EntryInner<'a> {
        Property(&'a Property),
        /// Value, array index
        Value(&'a Value, usize),
    }

    enum Builder<'a> {
        ObjectRef(&'a mut ObjectBuilder),
        Object(ObjectBuilder),
        Array(ObjectArrayBuilder),
    }

    impl<'a> Builder<'a> {
        fn set<T>(&mut self, class: &str, name: &str, index: usize, value: T)
        where
            T: IntoValue,
        {
            match self {
                Self::ObjectRef(builder) => builder.set(class, name, value),
                Self::Object(builder) => builder.set(class, name, value),
                Self::Array(builder) => builder.set_at(index, class, name, value),
            }
        }

        fn finalize(self) -> ObjectRef {
            match self {
                Self::ObjectRef(builder) => unsafe { builder.finalize_copy() },
                Self::Object(builder) => builder.finalize(),
                Self::Array(builder) => builder.finalize(),
            }
        }
    }

    let mut builders: Vec<Builder<'_>> = vec![Builder::ObjectRef(builder)];
    let mut stack: Vec<Entry<'_>> = Vec::new();
    // Push root class properties
    for prop in &script.properties {
        stack.push(Entry {
            builder_index: 0,
            inner: EntryInner::Property(prop),
            pop_builder: false,
        });
    }

    while let Some(entry) = stack.pop() {
        let builder = &mut builders[entry.builder_index];

        let (class_name, var_name, array_index) = match &entry.inner {
            EntryInner::Property(property) => {
                // Index 0 is the root script. Only the root can be a class.
                // For classes we must resolve the property to a variable name,
                // for structs we can diretly use the property name.
                if entry.builder_index == 0 {
                    let ((_, name), _) = layout
                        .properties
                        .iter()
                        // We can also set property on parent classes.
                        // FIXME: Ensure that child properties are preferred when present on both child and
                        // parent classes.
                        .find(|((_, name), _)| *name == Lowercase::new(property.name.as_str()))
                        .unwrap();

                    (script.name.as_str(), name.as_str().as_inner(), 0)
                } else {
                    ("", property.name.as_str(), 0)
                }
            }
            EntryInner::Value(_, array_index) => ("", "", *array_index),
        };

        let value = match &entry.inner {
            EntryInner::Property(prop) => &prop.value,
            EntryInner::Value(value, _) => value,
        };

        match value {
            Value::Bool(value) => {
                builder.set(class_name, var_name, array_index, *value);
            }
            Value::Int(value) => {
                builder.set(class_name, var_name, array_index, *value);
            }
            Value::Float(value) => {
                builder.set(class_name, var_name, array_index, *value);
            }
            Value::String(value) => {
                let object = engine.object_pool().insert_string(&value);
                builder.set(class_name, var_name, array_index, object.addr());
            }
            Value::Object(value) => {
                let id = resolve_form(value.form);

                let addr = if let Some(alias) = value.alias {
                    let scripts = aliases
                        .get(&AliasKey {
                            accessor: script_defined_in,
                            owner: id,
                            id: alias.into(),
                        })
                        .unwrap();

                    // FIXME: Which script to choose here?
                    // We need to inspect the actual class layout to get this.
                    let addr = *scripts.values().next().unwrap();

                    addr
                } else {
                    if let Some(scripts) = scripts.get_by_record(id) {
                        // FIXME: Which script to choose here?
                        // We need to inspect the actual class layout to get this.
                        let script = scripts.iter().next().unwrap();

                        script.get().object.addr()
                    } else {
                        let scripts = partial_addrs.get(&id).unwrap();
                        // FIXME: Which script to choose here?
                        // We need to inspect the actual class layout to get this.
                        let addr = *scripts.values().next().unwrap();

                        addr
                    }
                };

                builder.set(class_name, var_name, array_index, addr);
            }
            Value::Struct(fields, _) => {
                if entry.pop_builder {
                    let object = builders.pop().unwrap().finalize();

                    // Reborrow after pop
                    let builder = &mut builders[entry.builder_index];
                    builder.set(class_name, var_name, array_index, object.addr());
                } else {
                    let prop = layout
                        .properties
                        .get(&(Lowercase::new(class_name), Lowercase::new(var_name)))
                        .unwrap();

                    let PropertyType::Pointer(ty) = prop.ty else {
                        todo!();
                    };

                    let object = engine.object_pool().insert(ty);

                    let builder_index = builders.len();
                    builders.push(Builder::Object(object));

                    stack.push(Entry {
                        builder_index: entry.builder_index,
                        inner: entry.inner,
                        pop_builder: true,
                    });

                    for field in fields {
                        stack.push(Entry {
                            builder_index,
                            inner: EntryInner::Property(field),
                            pop_builder: false,
                        });
                    }
                }
            }
            Value::Array(values, _) => {
                if entry.pop_builder {
                    let object = builders.pop().unwrap().finalize();

                    // Reborrow after pop
                    let builder = &mut builders[entry.builder_index];
                    builder.set(class_name, var_name, array_index, object.addr());
                } else {
                    // First time we see this array.
                    let prop = layout
                        .properties
                        .get(&(Lowercase::new(class_name), Lowercase::new(var_name)))
                        .unwrap();

                    let PropertyType::ArrayPointer(ty) = prop.ty else {
                        todo!();
                    };

                    let elem_ty = if is_primitive(engine, ty) {
                        ty
                    } else {
                        engine.ptr_type()
                    };

                    let object = engine.object_pool().insert_array(elem_ty, values.len());

                    let builder_index = builders.len();
                    builders.push(Builder::Array(object));

                    stack.push(Entry {
                        builder_index: entry.builder_index,
                        inner: entry.inner,
                        pop_builder: true,
                    });

                    for (array_index, value) in values.iter().enumerate() {
                        stack.push(Entry {
                            builder_index,
                            inner: EntryInner::Value(value, array_index),
                            pop_builder: false,
                        });
                    }
                }
            }
        }
    }

    debug_assert_eq!(builders.len(), 1);
    builders.pop().unwrap().finalize()
}

fn is_primitive(engine: &Engine, ty: TypeId) -> bool {
    ty == engine.none_type()
        || ty == engine.bool_type()
        || ty == engine.int_type()
        || ty == engine.float_type()
}

#[cfg(test)]
mod tests {
    use hashbrown::HashMap;
    use openfo4_core::record::RecordId;
    use openfo4_esp::vmad::{Property, Script, Value};
    use openfo4_script::assembler::assemble;
    use openfo4_script::engine::{Engine, Options};
    use openfo4_script::runtime::{ArrayObject, ObjectAddr, ObjectRef};

    use crate::systems::scripting::objects::ScriptObjects;

    use super::build_script_object;

    fn build_direct(engine: &Engine, script: &Script) -> ObjectRef {
        let type_id = engine.get_type(&script.name).unwrap();
        let mut builder = engine.object_pool().insert(type_id);

        let scripts = ScriptObjects::default();
        let partial_addrs = HashMap::new();
        let aliases = HashMap::new();

        build_script_object(
            |_| unreachable!(),
            &engine,
            &script,
            &scripts,
            &partial_addrs,
            &aliases,
            &mut builder,
            RecordId::DANGLING,
        )
    }

    fn assert_array_contents_eq(
        engine: &Engine,
        object: &ObjectRef,
        class: &str,
        name: &str,
        len: usize,
        data: &[u8],
    ) {
        let object = engine.object_pool().get(&object);

        unsafe {
            let ptr = object.get(class, name).unwrap();
            let ptr = ObjectAddr::from_raw(ptr.as_ptr().cast::<*mut u8>().read());

            let array = engine.object_pool().get(ptr);
            let array = &*array.as_ptr().cast::<ArrayObject>();

            assert_eq!(array.len, len);

            let mut mem = Vec::new();

            let mut next_ptr = array.ptr;
            for _ in 0..array.len {
                let addr = ObjectAddr::from_raw(next_ptr);
                let elem = engine.object_pool().get(addr);
                mem.extend_from_slice(elem.memory());

                next_ptr = next_ptr.add(elem.size());
            }

            assert_eq!(mem, data);
        }
    }

    #[test]
    fn build_script_object_basic() {
        let mut engine = Engine::new();

        let module = assemble(
            r#"
            class test:
            global a: int = 0
            global b: int = 0
            property a: int -> a
            property b: int -> b
            "#,
        );

        engine.register(&module);
        engine.compile("test", &Options::default());

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Int(1),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Int(2),
                },
            ],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(&object);

        unsafe {
            assert_eq!(object.get("test", "a").unwrap(), 1u32.to_ne_bytes());
            assert_eq!(object.get("test", "b").unwrap(), 2u32.to_ne_bytes());
        }
    }

    #[test]
    fn build_script_object_array() {
        let mut engine = Engine::new();

        let module = assemble(
            r#"
            class test:
            global a: int[] = null
            global b: float[] = null
            global c: bool[] = null
            property a: int[] -> a
            property b: float[] -> b
            property c: bool[] -> c
            "#,
        );

        engine.register(&module);
        engine.compile("test", &Options::default());

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Array(vec![Value::Int(0), Value::Int(1), Value::Int(2)], 0),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Array(
                        vec![
                            Value::Float(0.0),
                            Value::Float(1.0),
                            Value::Float(2.0),
                            Value::Float(3.0),
                        ],
                        0,
                    ),
                },
                Property {
                    name: "c".to_owned(),
                    value: Value::Array(Vec::new(), 0),
                },
            ],
        };

        let object = build_direct(&engine, &script);

        assert_array_contents_eq(
            &engine,
            &object,
            "test",
            "a",
            3,
            bytemuck::cast_slice(&[0_i32, 1_i32, 2_i32]),
        );
        assert_array_contents_eq(
            &engine,
            &object,
            "test",
            "b",
            4,
            bytemuck::cast_slice(&[0.0_f32, 1.0_f32, 2.0_f32, 3.0_f32]),
        );
        assert_array_contents_eq(&engine, &object, "test", "c", 0, &[]);
    }

    #[test]
    fn build_script_object_struct() {
        let mut engine = Engine::new();

        let module = assemble(
            r#"
            class test:

            struct MyStruct:
                field x: int
                field y: int

            global a: int = 0
            global b: test#MyStruct = null
            property a: int -> a
            property b: test#MyStruct -> b
            "#,
        );

        engine.register(&module);
        engine.compile("test", &Options::default());

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Int(1),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Struct(
                        vec![
                            Property {
                                name: "x".to_owned(),
                                value: Value::Int(2),
                            },
                            Property {
                                name: "y".to_owned(),
                                value: Value::Int(3),
                            },
                        ],
                        0,
                    ),
                },
            ],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(&object);

        unsafe {
            assert_eq!(object.get("test", "a").unwrap(), 1u32.to_ne_bytes());
            let struct_addr = object
                .get("test", "b")
                .unwrap()
                .as_ptr()
                .cast::<ObjectAddr>()
                .read();

            let st = engine.object_pool().get(struct_addr);
            assert_eq!(st.get("test#MyStruct", "x").unwrap(), 2u32.to_ne_bytes());
            assert_eq!(st.get("test#MyStruct", "y").unwrap(), 3u32.to_ne_bytes());
        }
    }
}
