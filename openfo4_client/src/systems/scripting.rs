use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use game_common::collections::arena::{Arena, Key};
use game_tasks::TaskPool;
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_esp::vmad::{QuestFragments, Value};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_pex::Module;
use openfo4_script::classes::{self, CoreActorValues, State, StateProvider};
use openfo4_script::compiler::TypeLayout;
use openfo4_script::engine::{Engine, Instance};
use openfo4_script::runtime::{
    Context, Exception, NativeTuple, Object, ObjectAddr, ObjectPool, ObjectRef, Stack,
};
use parking_lot::Mutex;

use crate::loader::{FileLoader, fix_path};
use crate::state::esp::EspLoader;
use crate::systems::actor_value::ActorValueProperties;
use crate::systems::quests::QuestManager;

pub struct Executor {
    esp_loader: Arc<EspLoader>,
    file_loader: Arc<FileLoader>,
    engine: Engine,
    objects: HashMap<FormId, ObjectRef>,
    invocations: Arena<Invocation>,
    scripts: HashMap<FormId, HashMap<String, ScriptInstance>>,
    state: Option<State>,
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

        // test_build_script(&mut engine, &esp_loader, state).await;

        Self {
            esp_loader,
            engine,
            file_loader,
            objects: HashMap::new(),
            invocations: Arena::new(),
            scripts: HashMap::new(),
            state: Some(state),
        }
    }

    pub fn invoke(&mut self, invocation: Invocation) -> InvocationId {
        let mut id = self.invocations.insert(invocation);
        InvocationId(id)
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus, delta: Duration) {
        let _span = trace_span!("Executor::update").entered();

        let state = self.state.as_mut().unwrap();

        for invocation in self.invocations.values() {
            let form = match invocation {
                Invocation::QuestInit { quest } => *quest,
                Invocation::QuestSetStage {
                    quest,
                    stage: _,
                    item: _,
                } => *quest,
            };

            if !self.scripts.contains_key(&form) {
                futures_lite::future::block_on(instantiate_form(
                    &mut self.engine,
                    &mut self.scripts,
                    &self.esp_loader,
                    form,
                ));
            }

            let scripts = self.scripts.get_mut(&form).unwrap();

            invocation.call(&self.engine, scripts, &mut self.state);
        }

        let state = self.state.as_ref().unwrap();
        let mut real_timers = state.real_timers.lock();
        let mut game_timers = state.game_timers.lock();

        // for entry in real_timers.tick(delta) {
        //     self.invocations.insert(Invocation::Timer {
        //         object: entry.object,
        //         timer: entry.id,
        //     });
        // }
    }
}

fn call_all_and_await<Params>(
    engine: &Engine,
    scripts: &mut HashMap<String, ScriptInstance>,
    state: &mut Option<State>,
    name: &'static str,
    params: impl Fn(&ScriptInstance) -> Params,
) where
    Params: NativeTuple,
{
    for script in scripts.values_mut() {
        call_func(engine, script, state, name, params(&script));
    }
}

fn call_func<Params>(
    engine: &Engine,
    script: &mut ScriptInstance,
    state: &mut Option<State>,
    name: &'static str,
    params: Params,
) where
    Params: NativeTuple,
{
    let Some(func) = script.instance.get_func(name) else {
        return;
    };

    let stack = script
        .stacks
        .entry(name)
        .or_insert_with(|| Stack::new(func.stack_size()));

    let Ok(func) = func.typed::<Params, ()>() else {
        return;
    };

    let s = state.take().unwrap();
    let mut ctx = Context::new(engine, stack, s);
    tracing::debug!("calling {} on {:?}", name, script.object.addr());
    let res = func.call(&mut ctx, params);

    *state = Some(ctx.into_data());
    match res {
        Ok(()) | Err(Exception::Error) => {
            // Execution is done, reset the stack.
            script.stacks.remove(name);
        }
        Err(Exception::Suspend) => {}
    }
}

fn core_actor_values(engine: &mut Engine) -> State {
    // engine.compile("Actor");
    engine.compile("ActorValue");

    let dummy_layout = TypeLayout {
        header: None,
        fields: Vec::new(),
        properties: Vec::new(),
    };

    let actor_value = dummy_layout.clone();

    let player = engine.object_pool().insert(dummy_layout).finalize();

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
        player,
        actor_values: CoreActorValues {
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
        provider: Box::new(provider),
        real_timers: Default::default(),
        game_timers: Default::default(),
    }
}

async fn test_build_script(engine: &mut Engine, esp_loader: &EspLoader, mut state: State) {
    // let quest = match esp_loader
    //     .lookup(Some(RecordKind::Quest), FormId(0x1c53cb))
    //     .await
    //     .unwrap()
    //     .data
    // {
    //     RecordData::Quest(q) => q,
    //     _ => todo!(),
    // };

    // let vmad = quest.virtual_machine_adapter.as_ref().unwrap();

    // for script in &vmad.scripts {
    //     let name = &script.name;

    //     let instance = engine.compile(&script.name);

    //     let layout = engine.get_class_layout(&script.name).unwrap();

    //     dbg!(&layout);

    //     for property in &script.properties {
    //         let var = layout
    //             .properties
    //             .iter()
    //             .find(|p| {
    //                 p.class.to_lowercase() == script.name.to_lowercase()
    //                     && p.name.to_lowercase() == property.name.to_lowercase()
    //             })
    //             .unwrap();

    //         let var = &layout.fields[var.var];

    //         let (object, object_ref) = pool.insert(layout.clone());

    //         match &property.value {
    //             Value::Bool(value) => {
    //                 object.set(&script.name, &var.name, (*value).into());
    //             }
    //             Value::Int(value) => {
    //                 object.set(&script.name, &var.name, (*value).into());
    //             }
    //             Value::Float(value) => {
    //                 object.set(&script.name, &var.name, value.to_bits().into());
    //             }
    //             Value::Object(value) => {}
    //             _ => todo!(),
    //         }
    //     }

    //     let f = instance.get_func("OnPlayerSleepStop").unwrap();

    //     let mut stack = Stack::new(f.stack_size());
    //     let mut ctx = Context::new(&engine, stack, state);

    //     let typed = f.typed::<(isize, isize), ()>().unwrap();

    //     typed.call(&mut ctx, (0, 0)).unwrap();

    //     state = ctx.into_data();
    // }

    // let mut instances = HashMap::new();

    // instantiate_form(
    //     engine,
    //     &mut instances,
    //     esp_loader,
    //     // FormId(0x1c53cb),
    //     // FormId(0x80e),
    //     FormId(0x89c),
    // )
    // .await;

    // let instance = instances.get(&FormId(0x89c)).unwrap();
    // dbg!(&instance);
    // let instance = instance
    //     .get(&"DefaultQuestShutdownScript".to_lowercase())
    //     .unwrap();

    // let func = instance.instance.get_func("OnTimerGameTime").unwrap();
    // let typed = func.typed::<(ObjectAddr, i32), ()>().unwrap();

    // let stack = Stack::new(func.stack_size());
    // let mut ctx = Context::new(engine, stack, ());
    // typed.call(&mut ctx, (instance.object.addr(), 0)).unwrap();

    // dbg!(instances);

    todo!();
}

async fn instantiate_form(
    engine: &mut Engine,
    instances: &mut HashMap<FormId, HashMap<String, ScriptInstance>>,
    esp_loader: &EspLoader,
    form: FormId,
) {
    let mut queue = vec![form];
    let mut dependencies = HashMap::<FormId, HashSet<FormId>>::new();
    let mut dependents = HashMap::<FormId, HashSet<FormId>>::new();

    while let Some(form) = queue.pop() {
        if instances.contains_key(&form) {
            continue;
        }

        dbg!(&form);

        let record = esp_loader.lookup(None, form).await.unwrap();

        let vmad = match record.data.clone() {
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

        let mut scripts = HashMap::new();

        if let Some(vmad) = vmad {
            let mut depends = HashSet::new();
            for script in &vmad.scripts {
                let mut values_queue: Vec<_> = script.properties.iter().map(|v| &v.value).collect();

                while let Some(value) = values_queue.pop() {
                    match value {
                        Value::Object(value) => {
                            depends.insert(value.form);
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

            depends.retain(|f| !instances.contains_key(f));

            if !depends.is_empty() {
                for depens_on_form in depends {
                    if !instances.contains_key(&depens_on_form) {
                        queue.push(depens_on_form);
                        dependencies.entry(form).or_default().insert(depens_on_form);
                        dependents.entry(depens_on_form).or_default().insert(form);
                    }
                }

                continue;
            }

            for script in &vmad.scripts {
                let instance = engine.compile(&script.name);
                let layout = engine.get_class_layout(&script.name).unwrap();

                let mut object = engine.object_pool().insert(layout.clone());

                for property in &script.properties {
                    let var = layout
                        .properties
                        .iter()
                        .find(|p| {
                            p.class.to_lowercase() == script.name.to_lowercase()
                                && p.name.to_lowercase() == property.name.to_lowercase()
                        })
                        .unwrap();

                    let var = &layout.fields[var.var];

                    match &property.value {
                        Value::Bool(value) => {
                            object.set(&script.name, &var.name, *value);
                        }
                        Value::Int(value) => {
                            object.set(&script.name, &var.name, *value);
                        }
                        Value::Float(value) => {
                            object.set(&script.name, &var.name, *value);
                        }
                        Value::Object(value) => {
                            let instance_scripts = instances.get(&value.form).unwrap();
                            let instance = instance_scripts.get(&var.ty_name).unwrap();

                            object.set(&script.name, &var.name, instance.object.addr());
                        }
                        Value::String(value) => {}
                        _ => {}
                        _ => todo!(),
                    }
                }

                scripts.insert(
                    script.name.to_lowercase(),
                    ScriptInstance {
                        instance,
                        layout: layout.clone(),
                        object: object.finalize(),
                        stacks: HashMap::new(),
                    },
                );
            }
        }

        let name = match &record.data {
            RecordData::Action(_) => "Action",
            RecordData::Activator(_) => "Activator",
            RecordData::ActorValueInfo(_) => "ActorValue",
            RecordData::Ammo(_) => "Ammo",
            RecordData::Armor(_) => "Armor",
            RecordData::AssocationType(_) => "AssociationType",
            RecordData::Book(_) => "Book",
            RecordData::CameraShot(_) => "CameraShot",
            RecordData::Cell(_) => "Cell",
            RecordData::Class(_) => "Class",
            RecordData::CombatStyle(_) => "CombatStyle",
            RecordData::Component(_) => "Component",
            RecordData::ConstructibleObject(_) => "ConstructibleObject",
            RecordData::Container(_) => "Container",
            RecordData::Door(_) => "Door",
            RecordData::EffectShader(_) => "EffectShader",
            RecordData::EncounterZone(_) => "EncounterZone",
            RecordData::Explosion(_) => "Explosion",
            RecordData::FACT(_) => "Faction",
            RecordData::FLOR(_) => "Flora",
            RecordData::FormList(_) => "FormList",
            RecordData::Furniture(_) => "Furniture",
            RecordData::GLOB(_) => "GlobalVariable",
            RecordData::Hazard(_) => "Hazard",
            RecordData::HeadPart(_) => "HeadPart",
            RecordData::Animation(_) => "Idle",
            RecordData::IdleMarker(_) => "IdleMarker",
            RecordData::ImpactDataSet(_) => "ImpactDataSet",
            RecordData::Ingestible(_) => "Potion",
            RecordData::InstanceNamingRules(_) => "InstanceNamingRules",
            RecordData::Key(_) => "Key",
            RecordData::Keyword(_) => "Keyword",
            RecordData::LeveledItem(_) => "LeveledItem",
            RecordData::LeveledActor(_) => "LeveledActor",
            RecordData::Light(_) => "Light",
            RecordData::Location(_) => "Location",
            RecordData::LocationReferenceType(_) => "LocationRefType",
            RecordData::MagicEffect(_) => "MagicEffect",
            RecordData::Message(_) => "Message",
            RecordData::MiscObject(_) => "MiscObject",
            RecordData::MSTT(_) => "MovableStatic",
            RecordData::MusicType(_) => "MusicType",
            RecordData::Npc(_) => "ActorBase",
            RecordData::ObjectModification(_) => "ObjectMod",
            RecordData::Outfit(_) => "Outfit",
            RecordData::Package(_) => "Package",
            RecordData::Perk(_) => "Perk",
            RecordData::Projectile(_) => "Projectile",
            RecordData::Quest(_) => "Quest",
            RecordData::Race(_) => "Race",
            RecordData::ShaderParticleGeometry(_) => "ShaderParticleGeometry",
            RecordData::SoundDescriptor(_) => "Sound",
            RecordData::SoundCategory(_) => "SoundCategory",
            RecordData::SPEL(_) => "Spell",
            RecordData::STAT(_) => "Static",
            RecordData::TACT(_) => "TalkingActivator",
            RecordData::Terminal(_) => "Terminal",
            RecordData::TextureSet(_) => "TextureSet",
            RecordData::VisualEffect(_) => "VisualEffect",
            RecordData::VoiceType(_) => "VoiceType",
            RecordData::Weapon(_) => "Weapon",
            RecordData::Weather(_) => "Weather",
            RecordData::Worldspace(_) => "WorldSpace",
            RecordData::PlacedObject(_) => "ObjectReference",
            RecordData::PlacedNpc(_) => "Actor",
            x => todo!("{:?}", x),
        };

        dbg!(&name);
        let instance = engine.compile(&name);
        let layout = engine.get_class_layout(&name).unwrap();

        let object = engine.object_pool().insert(layout.clone());

        scripts.insert(
            name.to_lowercase(),
            ScriptInstance {
                instance,
                layout: layout.clone(),
                object: object.finalize(),
                stacks: HashMap::new(),
            },
        );

        instances.insert(form, scripts);

        if let Some(forms) = dependents.get_mut(&form) {
            for f in &*forms {
                let deps = dependencies.get_mut(f).unwrap();
                deps.remove(&form);
                if deps.is_empty() {
                    dependencies.remove(f);
                    queue.push(*f);
                }
            }
        }
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
    PlayerSleepStop { interrupted: bool, bed: FormId },
}

struct Provider {
    objects: Objects,
}

impl Provider {}

#[derive(Clone, Debug)]
struct Objects {
    globals: HashMap<ObjectAddr, FormId>,
    keywords: HashMap<ObjectAddr, FormId>,
    objects: HashMap<ObjectAddr, Entity>,
    actor_values: HashMap<ObjectAddr, FormId>,
    quests: HashMap<ObjectAddr, FormId>,
}

impl StateProvider for Provider {
    fn get_global(&self, global: ObjectAddr) -> Option<f32> {
        todo!()
    }

    fn set_global(&self, global: ObjectAddr, value: f32) {}

    fn get_actor_value(&self, object: ObjectAddr, actor_value: ObjectAddr) -> Option<f32> {
        let entity = self.objects.objects.get(&object)?;
        let actor_value = self.objects.actor_values.get(&actor_value)?;
        // let props = self.world.get::<&ActorValueProperties>(*entity).ok()?;

        // props.get(*actor_value)
        todo!()
    }

    fn set_actor_value(&self, object: ObjectAddr, actor_value: ObjectAddr, value: f32) {
        // let Some(entity) = self.objects.get(&object) else {
        //     return;
        // };

        // let Some(actor_value) = self.actor_values.get(&actor_value) else {
        //     return;
        // };

        // let Ok(mut props) = self.world.get::<&mut ActorValueProperties>(*entity) else {
        //     return;
        // };

        // props.set(*actor_value, value);
    }

    fn get_owner(&self, object: ObjectAddr) -> ObjectAddr {
        todo!()
    }

    fn set_owner(&self, object: ObjectAddr, owner: ObjectAddr) {}

    fn linked_ref(&self, object: ObjectAddr, keyword: ObjectAddr) -> Option<ObjectAddr> {
        None
    }

    fn start_quest(&self, quest: ObjectAddr) {
        let Some(id) = self.objects.quests.get(&quest) else {
            return;
        };
    }

    fn stop_quest(&self, quest: ObjectAddr) {}
}

/// An invocation event.
#[derive(Clone, Debug)]
pub enum Invocation {
    QuestInit {
        quest: FormId,
    },
    QuestSetStage {
        quest: FormId,
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
    ) {
        match self {
            Self::QuestInit { quest: _ } => {
                call_all_and_await(engine, scripts, state, "OnQuestInit", |script| {
                    script.object.addr()
                });
            }
            Self::QuestSetStage {
                quest: _,
                stage,
                item,
            } => {
                call_all_and_await(engine, scripts, state, "OnStageSet", |script| {
                    (script.object.addr(), *stage as i32, *item as i32)
                });
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(Key);
