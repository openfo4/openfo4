use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::{Duration, Instant};

use event_listener::Event;
use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_script::runtime::{Error, ObjectAddr, ObjectRef};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use slab::Slab;

use crate::components::BaseObject;
use crate::state::esp::EspLoader;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::classes::{Int, Object};
use crate::systems::scripting::effects::{Effect, EffectId, Effects};
use crate::systems::scripting::loader::ScriptLoader;
use crate::systems::scripting::objects::{ScriptObjects, ScriptRef, ScriptsRef};
use crate::systems::scripting::registration::RegistrationSet;

#[derive(Clone)]
pub struct State {
    pub core_objects: CoreObjects,
    pub real_timers: TimerSet,
    pub game_timers: TimerSet,
    pub borrowed: Arc<SharedBorrowCell<BorrowedState>>,
    pub registration: Arc<RegistrationSet>,
    pub objects: Arc<ScriptObjects>,
    pub aliases: AliasSet,
    pub effects: Arc<Mutex<Effects>>,
    pub loader: Arc<ScriptLoader>,
    pub globals: Arc<GlobalProperties>,
    pub wait_for_effects: Arc<Mutex<HashMap<EffectId, Event>>>,
    pub esp_loader: Arc<EspLoader>,
    pub deletion: HashSet<Entity>,
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
    pub async fn world(&self) -> Borrow<'_, World> {
        let s = self.borrowed.borrow().await;
        Borrow::map(s, |state| unsafe { &*state.world })
    }

    pub async fn quests(&mut self) -> BorrowMut<'_, QuestManager> {
        let s = self.borrowed.borrow_mut().await;
        BorrowMut::map(s, |state| unsafe { &mut *state.quests })
    }

    pub async fn entities(&self) -> Borrow<'_, ScriptEntities> {
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

    pub async fn get_or_create_record_class(
        &self,
        id: RecordId,
        class: &str,
    ) -> Result<ScriptRef<'_>, Error> {
        let scripts = self.get_or_create_record(id).await?;
        scripts
            .get(class)
            .ok_or_else(|| Error::msg(format!("{} has no object of type {}", id, class)))
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

    pub async fn run_effect_and_wait<T>(&mut self, effect: T) -> Result<T::Result, Error>
    where
        T: Effect,
    {
        let id = self.effects.lock().push(effect);

        let event = Event::new();
        let listen = event.listen();

        {
            let mut wait = self.wait_for_effects.lock();
            let exits = wait.insert(id, event);
            debug_assert!(exits.is_none());
        }

        listen.await;

        let mut effects = self.effects.lock();

        let value = effects
            .take_value(id)
            .map(|v| v.downcast().ok())
            .flatten()
            .map(|v| *v)
            .unwrap();

        effects.discard(id);
        Ok(value)
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

/// A cell that allows a thread to provide a value to another thread temporarily.
#[derive(Debug, Default)]
pub struct SharedBorrowCell<T> {
    value: RwLock<Option<T>>,
    event: Event,
}

impl<T> SharedBorrowCell<T> {
    pub fn new() -> Self {
        Self {
            value: RwLock::new(None),
            event: Event::new(),
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
    pub world: *mut World,
    pub quests: *mut QuestManager,
    pub entities: *const ScriptEntities,
    pub events: *mut EventBus,
}

unsafe impl Send for BorrowedState {}
unsafe impl Sync for BorrowedState {}
