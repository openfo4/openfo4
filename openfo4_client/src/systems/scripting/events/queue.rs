use std::collections::VecDeque;
use std::sync::Arc;

use game_tasks::{Task, TaskPool};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_script::runtime::ObjectRef;
use slab::Slab;

use crate::components::BaseObject;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};
use crate::systems::scripting::objects::ScriptObjects;

use super::{DispatchEvent, ObjectValue, QueuedEvent, QueuedEventValue};

#[derive(Debug, Default)]
pub struct EventQueue {
    events: Slab<EventEntry>,
    entities_queued: VecDeque<(Entity, usize)>,
    entities: HashMap<Entity, LoadEntry<LoadState>>,
    records_queued: VecDeque<(RecordId, usize)>,
    records: HashMap<RecordId, LoadEntry<LoadState>>,
    ready_events: Vec<usize>,
    dropped_events: Vec<usize>,
}

impl EventQueue {
    pub fn process(
        &mut self,
        world: &World,
        objects: &ScriptObjects,
        pool: &TaskPool,
        loader: &Arc<ScriptLoader>,
    ) -> Vec<DispatchEvent> {
        let _span = trace_span!("EventQueue::process").entered();

        for (entity, event) in self.entities_queued.drain(..) {
            if let Some(entry) = self.entities.get_mut(&entity) {
                entry.events.insert(event);

                if entry.state.is_ready() {
                    self.events[event].num_loading -= 1;

                    if self.events[event].num_loading == 0 {
                        self.ready_events.push(event);
                    }
                }

                continue;
            }

            let state = match world.get::<&ScriptObject>(entity) {
                Ok(s) => LoadState::Ready((*s).clone()),
                Err(_) => {
                    let Ok(object) = world.get::<&BaseObject>(entity) else {
                        tracing::error!("failed to load base object type for {:?}", entity);

                        self.dropped_events.push(event);

                        let event = &self.events[event];
                        tracing::error!("lost an event of type {:?}", event.event.kind);

                        continue;
                    };

                    let loader = Arc::clone(&loader);
                    let record = object.record;

                    {
                        let event = &self.events[event];
                        tracing::trace!(
                            "loading script for {:?} for event {:?}",
                            record,
                            event.event.kind
                        );
                    }

                    LoadState::Load(
                        pool.spawn(async move { loader.load(record).await.map(|res| res.object) }),
                    )
                }
            };

            if state.is_ready() {
                self.events[event].num_loading -= 1;

                if self.events[event].num_loading == 0 {
                    self.ready_events.push(event);
                }
            }

            let mut events = EventSet::default();
            events.insert(event);

            self.entities.insert(entity, LoadEntry { events, state });
        }

        for (entity, entry) in &mut self.entities {
            // FIXME: We should let the caller notify us if the script is ready,
            // not poll it every frame.
            if let LoadState::Load(task) = &mut entry.state
                && let Some(object) = task.get_output()
            {
                let Some(object) = object else {
                    for index in entry.events.iter() {
                        let event = &self.events[index];

                        tracing::error!(
                            "dropping event of type {:?}: could not resolve script for {:?}",
                            event.event.kind,
                            entity
                        );
                        self.dropped_events.push(index);
                    }

                    continue;
                };

                entry.state = LoadState::Ready(object);

                for index in entry.events.iter() {
                    let event = &mut self.events[index];
                    event.num_loading -= 1;

                    if event.num_loading == 0 {
                        self.ready_events.push(index);
                    }
                }
            }
        }

        for (record, event) in self.records_queued.drain(..) {
            if let Some(entry) = self.records.get_mut(&record) {
                entry.events.insert(event);

                if entry.state.is_ready() {
                    self.events[event].num_loading -= 1;

                    if self.events[event].num_loading == 0 {
                        self.ready_events.push(event);
                    }
                }

                continue;
            }

            {
                let event = &self.events[event];
                tracing::trace!(
                    "loading script for {:?} for event {:?}",
                    record,
                    event.event.kind
                );
            }

            // FIXME: Check if record already exists in `objects`.
            let loader = Arc::clone(loader);
            let state = LoadState::Load(
                pool.spawn(async move { loader.load(record).await.map(|res| res.object) }),
            );

            let mut events = EventSet::default();
            events.insert(event);

            self.records.insert(record, LoadEntry { events, state });
        }

        for (record, entry) in &mut self.records {
            // FIXME: We should let the caller notify us if the script is ready,
            // not poll it every frame.
            if let LoadState::Load(task) = &mut entry.state
                && let Some(object) = task.get_output()
            {
                let Some(object) = object else {
                    for index in entry.events.iter() {
                        let event = &self.events[index];

                        tracing::error!(
                            "dropping event of type {:?}: could not resolve script for {:?}",
                            event.event.kind,
                            record
                        );

                        self.dropped_events.push(index);
                    }

                    continue;
                };

                entry.state = LoadState::Ready(object);

                for index in entry.events.iter() {
                    let event = &mut self.events[index];
                    event.num_loading -= 1;

                    if event.num_loading == 0 {
                        self.ready_events.push(index);
                    }
                }
            }
        }

        let mut events = Vec::new();
        for index in self.ready_events.drain(..) {
            let event = &self.events[index];

            debug_assert_eq!(event.num_loading, 0);

            let get_entity = |entity: Entity, name: &str| -> ObjectRef {
                let entry = self.entities.get(&entity).unwrap();
                match &entry.state {
                    LoadState::Ready(object) => {
                        let object = objects.get_by_id(object.id()).unwrap();
                        object.get(name).unwrap().get().object.clone()
                    }
                    _ => unreachable!(),
                }
            };
            let get_record = |record: RecordId, name: &str| -> ObjectRef {
                let entry = self.records.get(&record).unwrap();
                match &entry.state {
                    LoadState::Ready(object) => {
                        let object = objects.get_by_id(object.id()).unwrap();
                        object.get(name).unwrap().get().object.clone()
                    }
                    _ => unreachable!(),
                }
            };
            let get_object = |object: &ScriptObject, name: &str| -> ObjectRef {
                let object = objects.get_by_id(object.id()).unwrap();
                object.get(name).unwrap().get().object.clone()
            };

            events.push(DispatchEvent {
                receiver: match &event.event.receiver {
                    QueuedEventValue::Object(ObjectValue::Entity(entity, _)) => {
                        let entry = self.entities.get(entity).unwrap();
                        match &entry.state {
                            LoadState::Ready(object) => {
                                let object = objects.get_by_id(object.id()).unwrap();
                                object.id()
                            }
                            _ => unreachable!(),
                        }
                    }
                    QueuedEventValue::Object(ObjectValue::Record(id, _)) => {
                        let entry = self.records.get(id).unwrap();
                        match &entry.state {
                            LoadState::Ready(object) => {
                                let object = objects.get_by_id(object.id()).unwrap();
                                object.id()
                            }
                            _ => unreachable!(),
                        }
                    }
                    QueuedEventValue::Object(ObjectValue::Script(object, _)) => object.id(),
                    _ => unreachable!(),
                },
                kind: event.event.kind.clone(),
                args: event
                    .event
                    .args
                    .iter()
                    .map(|val| val.to_raw(get_entity, get_record, get_object))
                    .collect(),
            });

            self.dropped_events.push(index);
        }

        for index in self.dropped_events.drain(..) {
            // The same event can be dropped multiple times.
            let Some(event) = self.events.try_remove(index) else {
                continue;
            };

            for value in event
                .event
                .args
                .iter()
                .chain(std::iter::once(&event.event.receiver))
            {
                match value {
                    QueuedEventValue::Bool(_) => {}
                    QueuedEventValue::Int(_) => {}
                    QueuedEventValue::Object(object) => match object {
                        ObjectValue::None | ObjectValue::Script(_, _) => {}
                        ObjectValue::Entity(entity, _) => {
                            if let Some(entry) = self.entities.get_mut(entity) {
                                entry.events.remove(index);

                                if entry.events.is_empty() {
                                    self.entities.remove(entity);
                                }
                            }
                        }
                        ObjectValue::Record(id, _) => {
                            if let Some(entry) = self.records.get_mut(id) {
                                entry.events.remove(index);

                                if entry.events.is_empty() {
                                    self.records.remove(id);
                                }
                            }
                        }
                    },
                }
            }
        }

        events
    }

    pub fn push(&mut self, event: QueuedEvent) {
        let index = self.events.vacant_key();

        let mut num_loading = 0;
        let mut entities_visited = HashSet::new();
        let mut records_visited = HashSet::new();

        for value in event.args.iter().chain(std::iter::once(&event.receiver)) {
            match value {
                QueuedEventValue::Bool(_) => {}
                QueuedEventValue::Int(_) => {}
                QueuedEventValue::Object(object) => match object {
                    ObjectValue::None | ObjectValue::Script(_, _) => {}
                    ObjectValue::Entity(entity, _) => {
                        if entities_visited.insert(*entity) {
                            self.entities_queued.push_back((*entity, index));
                            num_loading += 1;
                        }
                    }
                    ObjectValue::Record(id, _) => {
                        if records_visited.insert(*id) {
                            self.records_queued.push_back((*id, index));
                            num_loading += 1;
                        }
                    }
                },
            }
        }

        let real_index = self.events.insert(EventEntry { event, num_loading });
        debug_assert_eq!(index, real_index);

        if num_loading == 0 {
            self.ready_events.push(index);
        }
    }
}

#[derive(Clone, Debug)]
struct EventEntry {
    event: QueuedEvent,
    /// Number of objects that are still loading.
    num_loading: usize,
}

#[derive(Clone, Debug)]
struct LoadEntry<T> {
    /// Events that are waiting for this entry to become ready.
    events: EventSet,
    state: T,
}

#[derive(Debug)]
enum LoadState {
    Ready(ScriptObject),
    Load(Task<Option<ScriptObject>>),
}

impl LoadState {
    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }
}

#[derive(Clone, Debug, Default)]
struct EventSet {
    set: HashMap<usize, usize>,
}

impl EventSet {
    fn insert(&mut self, event: usize) {
        *self.set.entry(event).or_default() += 1;
    }

    fn remove(&mut self, event: usize) {
        let Some(counter) = self.set.get_mut(&event) else {
            return;
        };

        *counter -= 1;
        if *counter == 0 {
            self.set.remove(&event);
        }
    }

    fn iter(&self) -> impl Iterator<Item = usize> {
        self.set.keys().copied()
    }

    fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}
