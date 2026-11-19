use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

use event_listener::Event;
use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_script::runtime::object::ObjectHandle;
use openfo4_script::runtime::{Error, ObjectAddr};
use parking_lot::{MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard};
use slab::Slab;

use crate::state::esp::EspLoader;
use crate::state::level::loader::EntityLoader;
use crate::state::level::partial::PartialEntityFlags;
use crate::systems::global::GlobalProperties;
use crate::systems::scripting::classes::{Int, Object};
use crate::systems::scripting::effects::{Effect, EffectId, Effects};
use crate::systems::scripting::loader::ScriptLoader;
use crate::systems::scripting::objects::{ScriptObjects, ScriptRef, ScriptsRef};
use crate::systems::scripting::registration::RegistrationSet;
use crate::systems::scripting::{LoadRecordKind, ScriptEntities};

#[derive(Clone)]
pub struct State {
    pub core_objects: CoreObjects,
    pub real_timers: TimerSet,
    pub game_timers: TimerSet,
    pub borrowed: Arc<SharedBorrowCell<BorrowedState>>,
    pub registration: Arc<RegistrationSet>,
    pub objects: Arc<ScriptObjects>,
    pub effects: Arc<Mutex<Effects>>,
    pub loader: Arc<ScriptLoader>,
    pub globals: Arc<GlobalProperties>,
    pub wait_for_effects: Arc<Mutex<HashMap<EffectId, Event>>>,
    pub esp_loader: Arc<EspLoader>,
    pub deletion: HashSet<Entity>,
    pub entity_loader: Arc<EntityLoader>,
    pub entities: Arc<ScriptEntities>,
}

impl State {
    pub async fn world(&self) -> Borrow<'_, World> {
        let s = self.borrowed.borrow().await;
        Borrow::map(s, |state| unsafe { &*state.world })
    }

    /// Loads the [`RecordId`] for the given `object`.
    ///
    /// # Panics
    ///
    /// Panics if the given `object` has no associated [`RecordId`].
    pub fn record(&self, object: ObjectAddr) -> RecordId {
        let scripts = self.objects.get_by_addr(object).unwrap();
        scripts.record().expect("object has no associated record")
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

    /// Return the entity of the given object.
    pub async fn entity(&self, addr: ObjectAddr) -> Result<Entity, Error> {
        let object = self.objects.get_by_addr(addr).unwrap();
        if let Some(entity) = self.entities.resolve(object.id()) {
            return Ok(entity);
        }

        // For runtime entities the spawner of the entity MUST place the entity
        // into the `ScriptObject` that we queried previously before using the
        // script object.
        // This section is only reachable for records that were loaded as a dependency
        // as another entity inside the `ScriptLoader`. Only placed references can
        // be marked as dependencies, so the `ScriptLoader` cannot load a runtime entity.
        let Some(record) = object.record() else {
            unreachable!("attempted to lazy load runtime entity {addr:?}");
        };

        let entity = self
            .entity_loader
            .load(record, PartialEntityFlags::NONE)
            .await
            .unwrap();

        Ok(entity.entity())
    }

    pub async fn get_or_create_entity(&self, entity: Entity) -> Result<ScriptsRef<'_>, Error> {
        if let Some(id) = self.entities.get(entity)
            && let Some(object) = self.objects.get_by_id(id)
        {
            // The borrow checker is too conservative here and `self.objects` is
            // borrowed for '_ if `None` is returned.
            // We need to drop the lifetime here to keep using `self` mutable if this
            // branch is not taken.
            // This should not be necessary when the new borrow checker stabilizes:
            // https://smallcultfollowing.com/babysteps/blog/2018/06/15/mir-based-borrow-check-nll-status-update/#polonius
            let object = unsafe { core::mem::transmute::<ScriptsRef<'_>, ScriptsRef<'_>>(object) };
            return Ok(object);
        }

        let kind = { LoadRecordKind::extract(&*self.world().await, entity) };

        let object = kind.load(&self.loader).await;

        let scripts = match self.entities.set(entity, object.id()) {
            Ok(()) => self.objects.get_by_id(object.id()).unwrap(),
            Err(id) => self.objects.get_by_id(id).unwrap(),
        };

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
    pub health: Option<ObjectHandle>,
    // SPECIAL
    pub strength: Option<ObjectHandle>,
    pub perception: Option<ObjectHandle>,
    pub endurance: Option<ObjectHandle>,
    pub charisma: Option<ObjectHandle>,
    pub intelligence: Option<ObjectHandle>,
    pub agility: Option<ObjectHandle>,
    pub luck: Option<ObjectHandle>,
    // AI
    pub aggression: Option<ObjectHandle>,
    pub confidence: Option<ObjectHandle>,
    pub suspicious: Option<ObjectHandle>,
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
pub struct BorrowedState {
    pub world: *mut World,
    pub events: *mut EventBus,
}

unsafe impl Send for BorrowedState {}
unsafe impl Sync for BorrowedState {}
