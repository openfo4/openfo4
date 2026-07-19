mod f4se;
mod mcm;

mod activator;
mod active_magic_effect;
mod actor;
mod actor_base;
mod alias;
mod cell;
mod debug;
mod effect_shader;
mod encounter_zone;
mod faction;
mod form;
mod form_list;
mod furniture;
mod game;
mod global_variable;
mod image_space_modifier;
mod input_enable_layer;
mod keyword;
mod leveled_actor;
mod leveled_item;
mod location;
mod location_alias;
mod magic_effect;
mod math;
mod message;
mod misc_object;
mod music_type;
mod object_reference;
mod package;
mod potion;
mod quest;
mod ref_collection_alias;
mod reference_alias;
mod scene;
mod script_object;
mod sound;
mod sound_category;
mod sound_category_snapshot;
mod spell;
mod terminal;
mod topic;
mod topic_info;
mod utility;
mod visual_effect;
mod weapon;
mod weather;

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_script::RegisterNativeError;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Error, IntoFunc, ObjectAddr, ObjectRef};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use slab::Slab;

use crate::components::BaseObject;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::effects::{Effect, EffectId, Effects};
use crate::systems::scripting::loader::ScriptLoader;
use crate::systems::scripting::objects::{ScriptObjects, ScriptsRef};
use crate::systems::scripting::registration::RegistrationSet;

pub type Int = i32;
pub type Float = f32;
pub type Object = ObjectAddr;
pub type Bool = bool;

// Don't reformat long lines in here. This function is essentially a big table
// and one line per entry makes it clearer.
// FIXME: Optimally we would like to only skip long lines check, but is this possible?
#[rustfmt::skip]
pub fn register_builtins(engine: &mut Engine) {
    math::register(engine);
    object_reference::register(engine);
    input_enable_layer::register(engine);
    music_type::register(engine);
    alias::register(engine);
    scene::register(engine);
    debug::register(engine);
    actor_base::register(engine);
    weapon::register(engine);
    ref_collection_alias::register(engine);
    cell::register(engine);
    location::register(engine);
    location_alias::register(engine);
    game::register(engine);
    reference_alias::register(engine);
    quest::register(engine);
    leveled_item::register(engine);
    leveled_actor::register(engine);
    topic_info::register(engine);
    faction::register(engine);
    image_space_modifier::register(engine);
    message::register(engine);

    script_object::register(engine);
    form::register(engine);
    actor::register(engine);
    utility::register(engine);
    visual_effect::register(engine);
    sound_category_snapshot::register(engine);
    encounter_zone::register(engine);
    weather::register(engine);
    misc_object::register(engine);
    topic::register(engine);
    package::register(engine);
    activator::register(engine);
    furniture::register(engine);
    sound_category::register(engine);
    effect_shader::register(engine);
    active_magic_effect::register(engine);
    terminal::register(engine);
    global_variable::register(engine);
    form_list::register(engine);

    engine.register_if_exists("Sound", "SetInstanceVolume", sound::set_instance_volume);
    engine.register_if_exists("Sound", "StopInstance", sound::stop_instance);
    engine.register_if_exists("Sound", "Play", sound::play);
    engine.register_if_exists("Sound", "PlayAndWait", sound::play_and_wait);

    engine.register_if_exists("Spell", "Cast", spell::cast);
    engine.register_if_exists("Spell", "IsHostile", spell::is_hostile);
    engine.register_if_exists("Spell", "RemoteCast", spell::remote_cast);


    engine.register_if_exists("Keyword", "SendStoryEvent", keyword::send_story_event);
    engine.register_if_exists("Keyword", "SendStoryEventAndWait", keyword::send_story_event_and_wait);

    engine.register_if_exists("MagicEffect", "GetAssociatedSkill", magic_effect::get_assocaited_skill);

    engine.register_if_exists("Potion", "IsHostile", potion::is_hostile);

    f4se::register(engine);
    mcm::register(engine);
}

#[derive(Clone)]
pub struct State {
    pub core_objects: CoreObjects,
    pub real_timers: TimerSet,
    pub game_timers: TimerSet,
    pub borrowed: Arc<SharedBorrowCell<BorrowedState>>,
    pub registration: Arc<RegistrationSet>,
    pub objects: Arc<ScriptObjects>,
    pub object_data: HashMap<ObjectAddr, ObjectData>,
    pub aliases: AliasSet,
    pub effects: Arc<Mutex<Effects>>,
    pub wait_on_effect: Option<EffectId>,
    pub loader: Arc<ScriptLoader>,
    pub globals: Arc<GlobalProperties>,
}

#[derive(Clone, Debug, Default)]
pub struct AliasSet {
    aliases: HashMap<ObjectAddr, AliasState>,
    /// Maps "real" objects to aliases.
    objects: HashMap<ObjectAddr, HashSet<ObjectAddr>>,
}

impl AliasSet {
    pub fn get(&self, addr: ObjectAddr) -> Option<AliasState> {
        self.aliases.get(&addr).copied()
    }

    pub fn set(&mut self, alias: ObjectAddr, object: Option<ObjectAddr>) {
        if let Some(state) = self.aliases.get_mut(&alias) {
            if state.object == object {
                return;
            }

            if let Some(prev_object) = state.object {
                self.objects.get_mut(&prev_object).unwrap().remove(&alias);
            }

            state.object = object;

            if let Some(object) = object {
                self.objects.entry(object).or_default().insert(alias);
            }
        }
    }

    /// Returns all alias objects that point to the given object.
    pub fn get_alises_for_object(
        &self,
        object: ObjectAddr,
    ) -> impl Iterator<Item = ObjectAddr> + '_ {
        self.objects.get(&object).into_iter().flatten().copied()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AliasState {
    /// The quest that owns this alias.
    pub quest: RecordId,
    /// The real object that this alias points to.
    pub object: Option<ObjectAddr>,
}

impl State {
    async fn world(&self) -> Borrow<'_, World> {
        let s = self.borrowed.borrow().await;
        Borrow::map(s, |state| unsafe { &*state.world })
    }

    async fn quests(&mut self) -> BorrowMut<'_, QuestManager> {
        let s = self.borrowed.borrow_mut().await;
        BorrowMut::map(s, |state| unsafe { &mut *state.quests })
    }

    async fn entities(&self) -> Borrow<'_, ScriptEntities> {
        let s = self.borrowed.borrow().await;
        Borrow::map(s, |state| unsafe { &*state.entities })
    }

    /// Returns the game world entity associated with a script object, if any exists.
    pub async fn entity(&self, addr: ObjectAddr) -> Option<Entity> {
        let id = self.objects.get_by_addr(addr)?.id();
        self.entities().await.0.get_left(&id).copied()
    }

    /// Returns the scripts associated with the given [`RecordId`].
    ///
    /// If the object with the associated [`RecordId`] is already loaded, this function returns
    /// immediately.
    /// Otherwise it will suspend and resume once the object has been loaded asynchronously.
    pub async fn get_or_create_record(&self, id: RecordId) -> Result<ScriptsRef<'_>, Error> {
        if let Some(object) = self.objects.get_by_record(id) {
            // The borrow checker is too conservative here and `self.objects` is
            // borrowed for '_ if `None` is returned.
            // We need to drop the lifetime here to keep using `self` mutable if this
            // branch is not taken.
            // This should not be necessary when the new borrow checker stabilizes:
            // https://smallcultfollowing.com/babysteps/blog/2018/06/15/mir-based-borrow-check-nll-status-update/#polonius
            let object = unsafe { core::mem::transmute::<ScriptsRef<'_>, ScriptsRef<'_>>(object) };
            return Ok(object);
        }

        let Some(object) = self.loader.load(id).await else {
            return Err(Error::msg(format!(
                "failed to load scripts for record {}",
                id
            )));
        };

        let scripts = self.objects.get_by_id(object.object.id()).unwrap();
        Ok(scripts)
    }

    pub async fn get_or_create_entity(&self, entity: Entity) -> Result<ScriptsRef<'_>, Error> {
        {
            let entities = self.entities().await;

            if let Some(id) = entities.0.get_right(&entity)
                && let Some(object) = self.objects.get_by_id(*id)
            {
                // The borrow checker is too conservative here and `self.objects` is
                // borrowed for '_ if `None` is returned.
                // We need to drop the lifetime here to keep using `self` mutable if this
                // branch is not taken.
                // This should not be necessary when the new borrow checker stabilizes:
                // https://smallcultfollowing.com/babysteps/blog/2018/06/15/mir-based-borrow-check-nll-status-update/#polonius
                let object =
                    unsafe { core::mem::transmute::<ScriptsRef<'_>, ScriptsRef<'_>>(object) };
                return Ok(object);
            }
        }

        let base = {
            let world = self.world().await;
            let Ok(base) = world.get::<&BaseObject>(entity).map(|v| v.record) else {
                return Err(Error::msg(
                    "failed to load script for entity {:?}: unknown base object",
                ));
            };

            base
        };

        let obj = self.loader.load(base).await.unwrap().object;
        let obj = self.loader.clone_object(&obj).await;

        let scripts = self.objects.get_by_id(obj.id()).unwrap();
        Ok(scripts)
    }

    pub async fn run_effect_and_wait<T>(&mut self, effect: T) -> Result<Option<T::Result>, Error>
    where
        T: Effect,
    {
        // Previous effect complete, wake up.
        if let Some(effect) = self.wait_on_effect.take() {
            let value = self
                .effects
                .lock()
                .take_value(effect)
                .map(|v| v.downcast().ok())
                .flatten()
                .map(|v| *v);

            self.effects.lock().discard(effect);

            return Ok(value);
        }

        let id = self.effects.lock().push(effect);
        self.wait_on_effect = Some(id);
        // todo!();
        return Err(Error::msg("TODO"));
        // Err(Exception::Suspend)
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State").finish_non_exhaustive()
    }
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

/// Set of engine-builtin objects that are always loaded.
#[derive(Clone, Debug)]
pub struct CoreObjects {
    pub health: ObjectRef,
    // SPECIAL
    pub strength: ObjectRef,
    pub perception: ObjectRef,
    pub endurance: ObjectRef,
    pub charisma: ObjectRef,
    pub intelligence: ObjectRef,
    pub agility: ObjectRef,
    pub luck: ObjectRef,
    // AI
    pub aggression: ObjectRef,
    pub confidence: ObjectRef,
    pub suspicious: ObjectRef,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimerId(pub Int);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Timer {
    RealTime(Duration),
    GameTime(Duration),
}

#[derive(Clone, Debug)]
pub struct TimerSet {
    now: Instant,
    entries: Slab<TimerEntry>,
    next: BTreeMap<Instant, usize>,
    objects: HashMap<Object, HashMap<TimerId, usize>>,
}

impl TimerSet {
    pub fn insert(&mut self, object: Object, id: TimerId, duration: Duration) {
        self.remove(object, id);

        let mut at = self.now + duration;
        // Find next slot that is not empty.
        // FIXME: This pushes tasks slightly back, and is O(n),
        // can we do it in O(log(n)) instead?
        while self.next.contains_key(&at) {
            at += Duration::from_nanos(1);
        }

        let index = self.entries.insert(TimerEntry { object, id, at });
        self.next.insert(at, index);
        self.objects.entry(object).or_default().insert(id, index);
    }

    pub fn remove(&mut self, object_id: Object, id: TimerId) {
        let Some(object) = self.objects.get_mut(&object_id) else {
            return;
        };

        let Some(index) = object.remove(&id) else {
            return;
        };

        let entry = self.entries.remove(index);
        self.next.remove(&entry.at);

        if object.is_empty() {
            self.objects.remove(&object_id);
        }
    }

    pub fn tick(&mut self, time: Duration) -> impl Iterator<Item = TimerEntry> {
        self.now += time;

        let mut timers = Vec::new();
        for (_, index) in self.next.extract_if(..self.now, |_, _| true) {
            let entry = &self.entries[index];
            timers.push(*entry);
        }

        for e in &timers {
            self.remove(e.object, e.id);
        }

        timers.into_iter()
    }
}

impl Default for TimerSet {
    fn default() -> Self {
        Self {
            now: Instant::now(),
            entries: Slab::new(),
            next: BTreeMap::new(),
            objects: HashMap::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TimerEntry {
    pub object: ObjectAddr,
    pub id: TimerId,
    pub at: Instant,
}

#[derive(Clone, Debug)]
pub enum ObjectData {
    InputEnableLayer { layer: usize },
}

pub trait EngineExt {
    fn register_if_exists<F, T, Marker, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Marker, Params, Returns>;
}

impl EngineExt for Engine {
    fn register_if_exists<F, T, Marker, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Marker, Params, Returns>,
    {
        match self.register_native_function(class, func, f) {
            Ok(()) => {}
            Err(RegisterNativeError::UnknownFunction(_)) => {
                tracing::debug!("not registering {}.{}: not found", class, func);
            }
            Err(RegisterNativeError::IncompatibleSignature { expected, found }) => {
                tracing::warn!(
                    "not registering {}.{}: incompatible signature: {found:?}, expected {expected:?}",
                    class,
                    func,
                );
            }
        }
    }
}

/// A cell that allows a thread to provide a value to another thread temporarily.
#[derive(Debug, Default)]
pub struct SharedBorrowCell<T> {
    value: RwLock<Option<T>>,
    event: event_listener::Event,
}

impl<T> SharedBorrowCell<T> {
    pub fn new() -> Self {
        Self {
            value: RwLock::new(None),
            event: event_listener::Event::new(),
        }
    }

    /// Sets the current value.
    pub fn set(&self, value: T) {
        *self.value.write() = Some(value);

        // Notify all listeners that a value became available.
        // Note that a thread must recheck the value after it has registered
        // a listener, so it is not a problem that we may not wake up all listeners
        // here.
        self.event.notify(self.event.total_listeners());
    }

    pub fn take(&self) -> Option<T> {
        self.value.write().take()
    }

    /// Borrows the current value. If not value currently exists, this function will wait until
    /// some value is provided.
    pub async fn borrow(&self) -> Borrow<'_, T> {
        if let Some(guard) = self.try_borrow() {
            return guard;
        }

        loop {
            let listen = self.event.listen();

            // We need to recheck whether a value became available
            // between our last check and the time we registered the listener.
            if let Some(guard) = self.try_borrow() {
                return guard;
            }

            listen.await;
        }
    }

    /// Borrows the current value, if it exists. Returns `None` if not value is set currently.
    pub fn try_borrow(&self) -> Option<Borrow<'_, T>> {
        let guard = self.value.read();
        if guard.is_none() {
            return None;
        }

        let guard = RwLockReadGuard::map(guard, |value| value.as_ref().unwrap());
        Some(Borrow { guard })
    }

    /// Borrows the current value. If not value currently exists, this function will wait until
    /// some value is provided.
    pub async fn borrow_mut(&self) -> BorrowMut<'_, T> {
        if let Some(guard) = self.try_borrow_mut() {
            return guard;
        }

        loop {
            let listen = self.event.listen();

            // We need to recheck whether a value became available
            // between our last check and the time we registered the listener.
            if let Some(guard) = self.try_borrow_mut() {
                return guard;
            }

            listen.await;
        }
    }

    /// Borrows the current value, if it exists. Returns `None` if not value is set currently.
    pub fn try_borrow_mut(&self) -> Option<BorrowMut<'_, T>> {
        let guard = self.value.write();
        if guard.is_none() {
            return None;
        }

        let guard = RwLockWriteGuard::map(guard, |value| value.as_mut().unwrap());
        Some(BorrowMut { guard })
    }
}

#[derive(Debug)]
pub struct Borrow<'a, T> {
    guard: MappedRwLockReadGuard<'a, T>,
}

impl<'a, T> Borrow<'a, T> {
    pub fn map<U, F>(s: Self, f: F) -> Borrow<'a, U>
    where
        F: FnOnce(&T) -> &U,
    {
        let guard = MappedRwLockReadGuard::map(s.guard, f);
        Borrow { guard }
    }
}

impl<'a, T> Deref for Borrow<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

#[derive(Debug)]
pub struct BorrowMut<'a, T> {
    guard: MappedRwLockWriteGuard<'a, T>,
}

impl<'a, T> BorrowMut<'a, T> {
    pub fn map<U, F>(s: Self, f: F) -> BorrowMut<'a, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let guard = MappedRwLockWriteGuard::map(s.guard, f);
        BorrowMut { guard }
    }
}

impl<'a, T> Deref for BorrowMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<'a, T> DerefMut for BorrowMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

#[derive(Debug)]
pub struct BorrowedState {
    pub world: *const World,
    pub quests: *mut QuestManager,
    pub entities: *const ScriptEntities,
}

unsafe impl Send for BorrowedState {}
unsafe impl Sync for BorrowedState {}
