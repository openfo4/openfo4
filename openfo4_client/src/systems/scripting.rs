mod classes;

use std::borrow::Cow;
use std::f32::consts::E;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use async_lock::Mutex;
use futures::future::BoxFuture;
use futures_lite::FutureExt;
use game_common::collections::arena::{Arena, Key};
use game_common::string::Lowercase;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::vmad::{Property, Script, Value};
use openfo4_esp::{FormId, RecordData};
use openfo4_pex::Module;
use openfo4_script::compiler::{FieldType, ObjectField, ObjectProperty, TypeLayout};
use openfo4_script::engine::{Engine, Instance, Options};
use openfo4_script::runtime::object::{IntoValue, ObjectArrayBuilder, ObjectBuilder};
use openfo4_script::runtime::{
    Context, Exception, Func, NativeTuple, ObjectAddr, ObjectRef, Stack,
};
use slab::Slab;

use crate::loader::{FileLoader, fix_path};
use crate::state::esp::EspLoader;
use crate::systems::scripting::classes::CoreObjects;

use classes::State;

const OPTIONS: Options = Options {
    skip_native: true,
    skip_empty: true,
    only_events: true,
    use_native_stubs: false,
};

pub struct Executor {
    loader: Arc<ScriptLoader>,
    invocations: Arena<Invocation>,
    state: Option<State>,
    suspended_calls: Vec<SuspendedScriptCall>,
}

impl Executor {
    pub async fn new(
        esp_loader: Arc<EspLoader>,
        file_loader: Arc<FileLoader>,
        pool: Arc<TaskPool>,
    ) -> Self {
        let _span = trace_span!("Executor::new").entered();

        let mut engine = Engine::new();

        for path in file_loader.files() {
            let path = fix_path(path);
            if path.starts_with("scripts/") && path.ends_with(".pex") {
                let mut reader = file_loader.open(&path).await.unwrap();
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf).unwrap();

                let module = Module::decode(&buf[..]).unwrap();
                engine.register(&module);
            }
        }

        classes::register_builtins(&mut engine);

        let state = core_actor_values(&mut engine);

        let engine = Arc::new(Mutex::new(engine));

        let loader = Arc::new(ScriptLoader {
            engine,
            classes: Mutex::default(),
            instances: Mutex::default(),
            base_records: Mutex::default(),
            esp_loader,
        });

        Self {
            loader,
            invocations: Arena::new(),
            state: Some(state),
            suspended_calls: Vec::new(),
        }
    }

    pub fn loader(&self) -> &Arc<ScriptLoader> {
        &self.loader
    }

    pub fn invoke(&mut self, invocation: Invocation) -> InvocationId {
        let mut id = self.invocations.insert(invocation);
        InvocationId(id)
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus, delta: Duration) {
        let _span = trace_span!("Executor::update").entered();

        let state = self.state.as_mut().unwrap();
        state.world = world;

        self.resume_suspended_tasks();

        let engine = futures_lite::future::block_on(self.loader.engine.lock());
        let mut instances = futures_lite::future::block_on(self.loader.instances.lock());

        for invocation in self.invocations.values() {
            let object = match invocation {
                Invocation::QuestInit { quest } => quest,
                Invocation::QuestSetStage {
                    quest,
                    stage: _,
                    item: _,
                } => quest,
            };

            let scripts = instances.get_mut(object.0.id.0).unwrap();

            let suspended_calls = invocation.call(&engine, scripts, &mut self.state);
            for (name, call) in suspended_calls {
                self.suspended_calls.push(SuspendedScriptCall {
                    object: object.clone(),
                    script: name,
                    call,
                    invocation: invocation.clone(),
                });
            }
        }

        let state = self.state.as_mut().unwrap();
        // FIXME: Not necessarly required, we can leave pointers dangling.
        state.world = core::ptr::null_mut();
        state.quests = core::ptr::null_mut();

        // for entry in real_timers.tick(delta) {
        //     self.invocations.insert(Invocation::Timer {
        //         object: entry.object,
        //         timer: entry.id,
        //     });
        // }
    }

    fn resume_suspended_tasks(&mut self) {
        let _span = trace_span!("Executor::resume_suspended_tasks").entered();

        let engine = futures_lite::future::block_on(self.loader.engine.lock());
        let mut instances = futures_lite::future::block_on(self.loader.instances.lock());

        let mut ctx = std::task::Context::from_waker(std::task::Waker::noop());
        self.suspended_calls.retain_mut(|call| {
            if call.call.future.poll(&mut ctx).is_pending() {
                return true;
            }

            let scripts = instances.get_mut(call.object.0.id.0).unwrap();
            let script = scripts.get_mut(&call.script).unwrap();

            if let Some(next_call) = call
                .invocation
                .call_single(&engine, script, &mut self.state)
            {
                debug_assert_eq!(next_call.name, call.call.name);
                call.call.future = next_call.future;
                // Back to suspension...
                return true;
            }

            false
        });
    }
}

struct SuspendedScriptCall {
    object: ScriptObject,
    script: String,
    invocation: Invocation,
    call: SuspendedCall,
}

struct SuspendedCall {
    name: &'static str,
    future: BoxFuture<'static, ()>,
}

fn call_all_and_await<Params>(
    engine: &Engine,
    scripts: &mut HashMap<String, ScriptInstance>,
    state: &mut Option<State>,
    name: &'static str,
    params: impl Fn(&ScriptInstance) -> Params,
) -> HashMap<String, SuspendedCall>
where
    Params: NativeTuple,
{
    let mut calls = HashMap::new();
    for (script_name, script) in scripts.iter_mut() {
        if let Some(call) = call_func(engine, script, state, name, params(&script)) {
            calls.insert(script_name.clone(), call);
        }
    }
    calls
}

fn call_func<Params>(
    engine: &Engine,
    script: &mut ScriptInstance,
    state: &mut Option<State>,
    name: &'static str,
    params: Params,
) -> Option<SuspendedCall>
where
    Params: NativeTuple,
{
    let Some(func) = script.instance.get_func(name) else {
        return None;
    };

    let stack = script
        .stacks
        .entry(name)
        .or_insert_with(|| Stack::new(func.stack_size()));

    let Ok(func) = func.typed::<Params, ()>() else {
        return None;
    };

    let s = state.take().unwrap();
    let mut ctx = Context::new(engine, stack, s);
    tracing::debug!("calling {} on {:?}", name, script.object.addr());
    let res = func.call(&mut ctx, params);

    let mut data = ctx.into_data();
    let mut suspended_call = None;
    match res {
        Ok(()) | Err(Exception::Error) => {
            // Execution is done, reset the stack.
            script.stacks.remove(name);
        }
        Err(Exception::Suspend) => {
            debug_assert!(data.waker.is_some(), "suspended script without waker");

            if let Some(waker) = data.waker.take() {
                suspended_call = Some(SuspendedCall {
                    future: waker,
                    name,
                });
            }
        }
    }

    *state = Some(data);
    suspended_call
}

fn core_actor_values(engine: &mut Engine) -> State {
    engine.compile("Actor", &OPTIONS);
    engine.compile("ActorValue", &OPTIONS);

    let actor = engine.get_class_layout("Actor").unwrap();
    let actor_value = engine.get_class_layout("ActorValue").unwrap();

    let player = engine.object_pool().insert(actor.clone()).finalize();

    let [
        health,
        strength,
        perception,
        endurance,
        charisma,
        intelligence,
        agility,
        luck,
        aggression,
        confidence,
        suspicious,
    ] = std::array::from_fn(|_| engine.object_pool().insert(actor_value.clone()).finalize());

    let provider = Provider {
        objects: Objects {
            keywords: HashMap::new(),
            globals: HashMap::new(),
            actor_values: HashMap::new(),
            objects: HashMap::new(),
            quests: HashMap::new(),
        },
    };

    State {
        core_objects: CoreObjects {
            player,
            health,
            strength,
            perception,
            endurance,
            charisma,
            intelligence,
            agility,
            luck,
            aggression,
            confidence,
            suspicious,
        },
        real_timers: Default::default(),
        game_timers: Default::default(),
        world: core::ptr::null_mut(),
        quests: core::ptr::null_mut(),
        objects: Default::default(),
        waker: None,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct ScriptObjectId(usize);

/// The script object associated with an instanced record.
///
/// Note that a single `ScriptObject` may represent more than one script if more than one script
/// was attached to a record.
#[derive(Clone, Debug)]
pub struct ScriptObject(Arc<ScriptObjectInner>);

#[derive(Debug)]
struct ScriptObjectInner {
    base: RecordId,
    id: ScriptObjectId,
    // TODO: Cleanup when this is dropped.
}

#[derive(Debug)]
pub struct ScriptLoader {
    engine: Arc<Mutex<Engine>>,
    instances: Mutex<Slab<HashMap<String, ScriptInstance>>>,
    base_records: Mutex<HashMap<RecordId, ScriptObjectId>>,
    /// Cache of compiled classes.
    classes: Mutex<HashMap<String, Instance>>,
    esp_loader: Arc<EspLoader>,
}

impl ScriptLoader {
    /// Loads the script object attached to a record.
    ///
    /// Returns `None` if no script is attached to the record.
    #[tracing::instrument(name = "ScriptLoader::load", skip_all)]
    pub async fn load(&self, record: RecordId) -> Option<ScriptObject> {
        tracing::debug!("loading script for {:?}", record);

        let mut engine = self.engine.lock().await;
        let mut instances = self.instances.lock().await;
        let mut base_records = self.base_records.lock().await;
        let mut classes = self.classes.lock().await;

        let id = instantiate_form(
            &mut engine,
            &mut instances,
            &mut base_records,
            &self.esp_loader,
            record,
            &mut classes,
        )
        .await?;

        Some(ScriptObject(Arc::new(ScriptObjectInner {
            base: record,
            id,
        })))
    }

    /// Returns a deep clone of the [`ScriptObject`].
    ///
    /// Only the top level properties are cloned.
    pub async fn clone_object(&self, object: &ScriptObject) -> ScriptObject {
        let engine = self.engine.lock().await;
        let mut instances = self.instances.lock().await;
        let instance = instances.get(object.0.id.0).unwrap();

        let scripts = instance
            .iter()
            .map(|(key, val)| {
                (
                    key.to_owned(),
                    ScriptInstance {
                        instance: val.instance.clone(),
                        layout: val.layout.clone(),
                        stacks: val.stacks.clone(),
                        object: unsafe {
                            engine.object_pool().clone_object(&val.object).finalize()
                        },
                    },
                )
            })
            .collect();

        let id = ScriptObjectId(instances.insert(scripts));

        ScriptObject(Arc::new(ScriptObjectInner {
            base: object.0.base,
            id,
        }))
    }
}

async fn instantiate_form(
    engine: &mut Engine,
    instances: &mut Slab<HashMap<String, ScriptInstance>>,
    base_instances: &mut HashMap<RecordId, ScriptObjectId>,
    esp_loader: &EspLoader,
    root: RecordId,
    compiled_classes: &mut HashMap<String, Instance>,
) -> Option<ScriptObjectId> {
    if let Some(id) = base_instances.get(&root) {
        return Some(*id);
    }

    let mut root_id = None;

    let mut records = HashMap::new();
    let mut queue = vec![root];
    while let Some(record) = queue.pop() {
        if base_instances.contains_key(&record) {
            continue;
        }

        let record = esp_loader.lookup(None, record).await.unwrap();

        let vmad = match record.local.data.clone() {
            RecordData::Activator(data) => data.virtual_machine_adapter,
            RecordData::Armor(data) => data.virtual_machine_adapter,
            RecordData::Book(data) => data.virtual_machine_adapter,
            RecordData::Container(data) => data.virtual_machine_adapter,
            RecordData::Door(data) => data.virtual_machine_adapter,
            RecordData::FLOR(data) => data.vmad,
            RecordData::Furniture(data) => data.virtual_machine_adapter,
            RecordData::INGR(data) => data.vmad,
            RecordData::Key(data) => data.virtual_machine_adapter,
            RecordData::Light(data) => data.virtual_machine_adapter,
            RecordData::MagicEffect(data) => data.virtual_machine_adpater,
            RecordData::MiscObject(data) => data.virtual_machine_adapter,
            RecordData::MSTT(data) => data.virtual_machine_adapter,
            RecordData::Npc(data) => data.virtual_machine_adapter,
            RecordData::Note(data) => data.virtual_machine_adapter,
            RecordData::Package(data) => data.virtual_machine_adapter,
            RecordData::Perk(data) => data.virtual_machine_adapter,
            RecordData::Quest(data) => data.virtual_machine_adapter.map(|v| v.without_fragments()),
            RecordData::STAT(data) => data.virtual_machine_adapter,
            RecordData::TACT(data) => data.vmad,
            RecordData::Terminal(data) => data.virtual_machine_adapter,
            RecordData::Tree(data) => data.virtual_machine_adapter,
            RecordData::Weapon(data) => data.virtual_machine_adapter,
            RecordData::PlacedObject(data) => data.virtual_machine_adapter,
            RecordData::PlacedNpc(data) => data.virtual_machine_adapter,
            _ => None,
        };

        // Find all other forms that the scripts reference.
        // Note that scripts may reference the same form multiple times, which
        // is why we use a HashSet instead of a Vec or directly pushing to queue.
        let mut depends = HashSet::new();
        for script in vmad.iter().flat_map(|v| &v.scripts) {
            let mut values_queue: Vec<_> = script.properties.iter().map(|v| &v.value).collect();

            while let Some(value) = values_queue.pop() {
                match value {
                    Value::Object(value) => {
                        if let Some(alias) = &value.alias {
                            // TODO
                        } else {
                            let id = esp_loader.resolve_form(value.form, record.id).unwrap();

                            if base_instances.contains_key(&id) || records.contains_key(&id) {
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
            debug_assert!(!base_instances.contains_key(&id));
            debug_assert!(!records.contains_key(&id));
            queue.push(id);
        }

        let mut scripts = HashMap::new();

        // Can't attach scripts to objects with no base class.
        let base_class_name = record_script_name(&record.local.data).unwrap();
        let instance = compiled_classes
            .entry(base_class_name.to_owned())
            .or_insert_with(|| engine.compile(base_class_name, &OPTIONS));
        let layout = engine.get_class_layout(base_class_name).unwrap();
        let builder = engine.object_pool().insert(layout.clone());

        scripts.insert(
            base_class_name.to_owned(),
            PartialObject {
                instance: instance.clone(),
                builder,
                script: None,
            },
        );

        for script in vmad.into_iter().flat_map(|v| v.scripts) {
            let instance = compiled_classes
                .entry(script.name.clone())
                .or_insert_with(|| engine.compile(&script.name, &OPTIONS));
            let layout = engine.get_class_layout(&script.name).unwrap();
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
    }

    let mut partial_object_addrs = HashMap::new();
    for (id, scripts) in &records {
        let mut addrs = HashMap::new();
        for (name, script) in scripts {
            addrs.insert(name.clone(), script.builder.addr());
        }
        partial_object_addrs.insert(*id, addrs);
    }

    for (record, scripts) in records {
        let mut final_instances = HashMap::new();

        for (name, mut object) in scripts {
            if let Some(script) = &object.script {
                tracing::debug!("instantiating script {} (record {:?})", name, record);

                build_script_object(
                    |form| esp_loader.resolve_form(form, record).unwrap(),
                    engine,
                    script,
                    instances,
                    base_instances,
                    &partial_object_addrs,
                    &mut object.builder,
                );
            }

            let layout = engine.get_class_layout(&name).unwrap();
            final_instances.insert(
                name.clone(),
                ScriptInstance {
                    instance: object.instance,
                    layout: layout.clone(),
                    object: object.builder.finalize(),
                    stacks: HashMap::new(),
                },
            );
        }

        debug_assert!(!base_instances.contains_key(&record));
        let id = ScriptObjectId(instances.insert(final_instances));
        base_instances.insert(record, id);

        if record == root {
            root_id = Some(id);
        }
    }

    root_id
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

enum NativeObject {
    Spell,
}

#[derive(Clone, Debug)]
struct ObjectInstance {
    scripts: HashMap<String, ScriptInstance>,
}

#[derive(Clone, Debug)]
struct ScriptInstance {
    instance: Instance,
    layout: TypeLayout,
    object: ObjectRef,
    stacks: HashMap<&'static str, Stack>,
}

pub enum Event {
    PlayerSleepStop { interrupted: bool, bed: RecordId },
}

struct Provider {
    objects: Objects,
}

impl Provider {}

#[derive(Clone, Debug, Default)]
struct Objects {
    globals: HashMap<ObjectAddr, RecordId>,
    keywords: HashMap<ObjectAddr, RecordId>,
    objects: HashMap<ObjectAddr, Entity>,
    actor_values: HashMap<ObjectAddr, RecordId>,
    quests: HashMap<ObjectAddr, RecordId>,
}

impl Objects {
    pub fn quest(&self, addr: ObjectAddr) -> Option<RecordId> {
        self.quests.get(&addr).copied()
    }
}

/// An invocation event.
#[derive(Clone, Debug)]
pub enum Invocation {
    QuestInit {
        quest: ScriptObject,
    },
    QuestSetStage {
        quest: ScriptObject,
        stage: u16,
        item: u16,
    },
    // Timer {
    //     object: ObjectAddr,
    //     timer: TimerId,
    // },
}

impl Invocation {
    fn call(
        &self,
        engine: &Engine,
        scripts: &mut HashMap<String, ScriptInstance>,
        state: &mut Option<State>,
    ) -> HashMap<String, SuspendedCall> {
        match self {
            Self::QuestInit { quest: _ } => {
                call_all_and_await(engine, scripts, state, "OnQuestInit", |script| {
                    script.object.addr()
                })
            }
            Self::QuestSetStage {
                quest: _,
                stage,
                item,
            } => call_all_and_await(engine, scripts, state, "OnStageSet", |script| {
                (script.object.addr(), *stage as i32, *item as i32)
            }),
        }
    }

    fn call_single(
        &self,
        engine: &Engine,
        script: &mut ScriptInstance,
        state: &mut Option<State>,
    ) -> Option<SuspendedCall> {
        match self {
            Self::QuestInit { quest: _ } => {
                call_func(engine, script, state, "OnQuestInit", script.object.addr())
            }
            Self::QuestSetStage {
                quest: _,
                stage,
                item,
            } => call_func(
                engine,
                script,
                state,
                "OnStageSet",
                (script.object.addr(), *stage as i32, *item as i32),
            ),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(Key);

// FIXME: Use script layout reflection to determine we're actually setting values of the correct type.
fn build_script_object(
    resolve_form: impl Fn(FormId) -> RecordId,
    engine: &Engine,
    script: &Script,
    instances: &Slab<HashMap<String, ScriptInstance>>,
    base_instances: &HashMap<RecordId, ScriptObjectId>,
    partial_addrs: &HashMap<RecordId, HashMap<String, ObjectAddr>>,
    builder: &mut ObjectBuilder,
) -> ObjectRef {
    let layout = engine.get_class_layout(&script.name).unwrap();

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
                    let var = layout
                        .properties
                        .iter()
                        // We can also set property on parent classes.
                        // FIXME: Ensure that child properties are preferred when present on both child and
                        // parent classes.
                        .find(|p| p.name == Lowercase::new(property.name.as_str()))
                        .unwrap();

                    (script.name.as_str(), var.name.as_str().as_inner(), 0)
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
                if let Some(alias) = value.alias {
                    // TODO
                } else {
                    let id = resolve_form(value.form);

                    let addr = if let Some(instance) = base_instances.get(&id) {
                        let scripts = &instances[instance.0];
                        // FIXME: Which script to choose here?
                        // We need to inspect the actual class layout to get this.
                        let script = scripts.values().next().unwrap();

                        script.object.addr()
                    } else {
                        let scripts = partial_addrs.get(&id).unwrap();
                        // FIXME: Which script to choose here?
                        // We need to inspect the actual class layout to get this.
                        let addr = *scripts.values().next().unwrap();

                        addr
                    };

                    builder.set(class_name, var_name, array_index, addr);
                }
            }
            Value::Struct(fields, _) => {
                if entry.pop_builder {
                    let object = builders.pop().unwrap().finalize();

                    // Reborrow after pop
                    let builder = &mut builders[entry.builder_index];
                    builder.set(class_name, var_name, array_index, object.addr());
                } else {
                    // First time we see this struct.
                    let layout = build_struct_layout(&fields);
                    let object = engine.object_pool().insert(layout);

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
                    let elem_layout = build_array_layout(values);
                    let object = engine.object_pool().insert_array(elem_layout, values.len());

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

// FIXME: Can we expose this from the compiler instead?
fn build_struct_layout(props: &[Property]) -> TypeLayout {
    let mut fields = Vec::new();
    let mut properties = Vec::new();

    let mut offset = 0;
    for prop in props {
        let ty = match prop.value {
            Value::Bool(_) => FieldType::I32,
            Value::Int(_) => FieldType::I32,
            Value::Float(_) => FieldType::F32,
            Value::Array(_, _) => FieldType::Pointer,
            Value::String(_) => FieldType::Pointer,
            Value::Struct(_, _) => FieldType::Pointer,
            Value::Object(_) => FieldType::Pointer,
        };

        offset = (offset + ty.align() - 1) & !(ty.align() - 1);
        debug_assert_eq!(offset % ty.align(), 0);
        offset += ty.size();

        properties.push(ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new(prop.name.as_str()),
            var: fields.len(),
        });
        fields.push(ObjectField {
            class: Lowercase::new(""),
            name: Lowercase::new(prop.name.as_str()),
            offset,
            ty,
            ty_name: Lowercase::new(""),
        });
    }

    TypeLayout {
        header: None,
        fields,
        properties,
    }
}

fn build_array_layout(values: &[Value]) -> TypeLayout {
    let ty = match &values[0] {
        Value::Bool(_) => FieldType::Bool,
        Value::Int(_) => FieldType::I32,
        Value::Float(_) => FieldType::F32,
        Value::String(_) => FieldType::Pointer,
        Value::Array(_, _) => FieldType::Pointer,
        Value::Struct(_, _) => FieldType::Pointer,
        Value::Object(_) => FieldType::Pointer,
    };

    TypeLayout {
        header: None,
        fields: vec![ObjectField {
            class: Lowercase::new(""),
            name: Lowercase::new(""),
            offset: 0,
            ty,
            ty_name: Lowercase::new(""),
        }],
        properties: vec![ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new(""),
            var: 0,
        }],
    }
}

#[cfg(test)]
mod tests {
    use hashbrown::HashMap;
    use openfo4_esp::vmad::{Property, Script, Value};
    use openfo4_script::assembler::assemble;
    use openfo4_script::engine::{Engine, Options};
    use openfo4_script::runtime::{ObjectAddr, ObjectRef};
    use slab::Slab;

    use super::build_script_object;

    fn build_direct(engine: &Engine, script: &Script) -> ObjectRef {
        let layout = engine.get_class_layout(&script.name).unwrap();
        let mut builder = engine.object_pool().insert(layout.clone());

        let instances = Slab::new();
        let base_instances = HashMap::new();
        let partial_addrs = HashMap::new();

        build_script_object(
            |_| unreachable!(),
            &engine,
            &script,
            &instances,
            &base_instances,
            &partial_addrs,
            &mut builder,
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
            // FIXME: This relies on undocumented ABI. We should expose the array ABI
            // information from the Engine.
            let a_ptr = array.get("", "ptr").unwrap();
            let a_len = array.get("", "len").unwrap();
            let a_len = usize::from_ne_bytes(a_len.try_into().unwrap());

            assert_eq!(a_len, len);

            let mut mem = Vec::new();

            let mut next_ptr = a_ptr.as_ptr().cast::<*mut u8>().read();
            for _ in 0..a_len {
                let addr = ObjectAddr::from_raw(next_ptr);
                let elem = engine.object_pool().get(addr);
                mem.extend_from_slice(elem.memory());

                next_ptr = next_ptr.add(elem.layout().size_padded());
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
            property a: int[] -> a
            property b: float[] -> b
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
    }
}
