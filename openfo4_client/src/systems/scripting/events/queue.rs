use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use game_tasks::{TaskPool, TaskQueue};
use game_tracing::trace_span;
use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_script::runtime::ObjectAddr;
use slab::Slab;

use crate::components::BaseObject;
use crate::systems::scripting::invocations::Value;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};
use crate::systems::scripting::objects::{ScriptObjects, ScriptsRef};

use super::{DispatchEvent, ObjectValue, QueuedEvent, QueuedEventValue};

#[derive(Debug, Default)]
pub struct EventQueue {
    events: Slab<EventEntry>,
    queue: VecDeque<EventQueueEntry>,
    loading: HashMap<RecordId, LoadEntry>,
    tasks: TaskQueue<(RecordId, Option<ScriptObject>)>,
    ready_events: Vec<usize>,
}

impl EventQueue {
    pub fn process(
        &mut self,
        objects: &ScriptObjects,
        pool: &TaskPool,
        loader: &Arc<ScriptLoader>,
    ) -> Vec<DispatchEvent> {
        let _span = trace_span!("EventQueue::process").entered();

        while let Some(event) = self.queue.pop_front() {
            let entry = self.events.vacant_entry();
            let index = entry.key();
            let mut loading = HashSet::new();

            for object in &event.objects {
                let record = match object {
                    QueuedObject::Record(record) => *record,
                };

                loading.insert(record);

                match self.loading.entry(record) {
                    Entry::Occupied(mut entry) => {
                        let entry = entry.get_mut();

                        debug_assert!(!entry.events.contains(&index));
                        entry.events.insert(index);
                    }
                    Entry::Vacant(entry) => {
                        let loader = Arc::clone(loader);
                        self.tasks.push(pool.spawn(async move {
                            let object = loader.load(record).await.map(|res| res.object);
                            (record, object)
                        }));

                        entry.insert(LoadEntry {
                            events: HashSet::from([index]),
                            value: None,
                        });
                    }
                }
            }

            if loading.is_empty() {
                self.ready_events.push(index);
            }

            entry.insert(EventEntry {
                event: event.event,
                num_loading: loading.len(),
                objects: loading,
                entities: event.entities,
            });
        }

        while let Some((record, object)) = self.tasks.pop() {
            let entry = self.loading.get_mut(&record).unwrap();
            entry.value = object;

            for key in &entry.events {
                let event = &mut self.events[*key];

                event.num_loading -= 1;

                if event.num_loading == 0 {
                    self.ready_events.push(*key);
                }
            }
        }

        let mut events = Vec::new();
        while let Some(key) = self.ready_events.pop() {
            let event = self.events.remove(key);

            debug_assert_eq!(event.num_loading, 0);

            let convert_object = |object| -> Result<Value, LoadError<'_>> {
                match object {
                    ObjectValue::None => Ok(Value::ObjectAddr(ObjectAddr::NULL)),
                    ObjectValue::Script(object, name) => {
                        let object = objects.get_by_id(object.id()).unwrap();
                        let Some(script) = object.get(name) else {
                            return Err(LoadError::NoClass { object, name });
                        };

                        Ok(Value::ObjectRef(script.get().object.clone()))
                    }
                    ObjectValue::Entity(entity, name) => {
                        let record = *event.entities.get(&entity).unwrap();
                        let state = self.loading.get(&record).unwrap();

                        let Some(object) = &state.value else {
                            return Err(LoadError::FailedToLoad { record });
                        };

                        let object = objects.get_by_id(object.id()).unwrap();
                        let Some(script) = object.get(name) else {
                            return Err(LoadError::NoClass { object, name });
                        };

                        Ok(Value::ObjectRef(script.get().object.clone()))
                    }
                    ObjectValue::Record(record, name) => {
                        let state = self.loading.get(&record).unwrap();

                        let Some(object) = &state.value else {
                            return Err(LoadError::FailedToLoad { record });
                        };

                        let object = objects.get_by_id(object.id()).unwrap();
                        let Some(script) = object.get(name) else {
                            return Err(LoadError::NoClass { object, name });
                        };

                        Ok(Value::ObjectRef(script.get().object.clone()))
                    }
                }
            };

            let receiver = match event.event.receiver {
                QueuedEventValue::Object(object) => match convert_object(object) {
                    Ok(object) => match object {
                        Value::ObjectRef(v) => {
                            let object = objects.get_by_addr(v.addr()).unwrap();
                            object.id()
                        }
                        // FIXME: receiver should be an ScriptRef instead, not a
                        // ScriptObjectId.
                        _ => todo!(),
                    },
                    Err(err) => {
                        tracing::error!("failed to prepare object for event: {}", err);
                        tracing::error!("lost an event of type {:?}", event.event.kind);
                        continue;
                    }
                },
                _ => unreachable!("receiver must be an object"),
            };

            let mut args = Vec::with_capacity(event.event.args.len());
            for value in event.event.args {
                let value = match value {
                    QueuedEventValue::Bool(v) => Value::Bool(v),
                    QueuedEventValue::Int(v) => Value::Int(v),
                    QueuedEventValue::Object(object) => match convert_object(object) {
                        Ok(object) => object,
                        Err(err) => {
                            tracing::error!("failed to prepare object for event: {}", err);
                            tracing::error!("lost an event of type {:?}", event.event.kind);
                            continue;
                        }
                    },
                };

                args.push(value);
            }

            // Cleanup referenced records.
            for record in event.objects {
                let state = self.loading.get_mut(&record).unwrap();
                let existed = state.events.remove(&key);
                debug_assert!(existed);

                if state.events.is_empty() {
                    self.loading.remove(&record);
                }
            }

            events.push(DispatchEvent {
                receiver,
                kind: event.event.kind,
                args,
            });
        }

        events
    }

    pub fn push(&mut self, world: &World, mut event: QueuedEvent) {
        let mut records_visited = HashSet::new();

        let mut objects = Vec::new();
        let mut entities = HashMap::new();

        for value in event
            .args
            .iter_mut()
            .chain(std::iter::once(&mut event.receiver))
        {
            let QueuedEventValue::Object(object) = value else {
                continue;
            };

            let object = match object {
                ObjectValue::None => continue,
                ObjectValue::Script(_, _) => continue,
                ObjectValue::Entity(entity, name) => match world.get::<&ScriptObject>(*entity) {
                    Ok(s) => {
                        *object = ObjectValue::Script(s.clone(), name);
                        continue;
                    }
                    Err(_) => {
                        let Ok(object) = world.get::<&BaseObject>(*entity) else {
                            tracing::error!("failed to load base object type for {:?}", entity);
                            tracing::error!("lost an event of type {:?}", event.kind);
                            return;
                        };

                        entities.insert(*entity, object.record);

                        if !records_visited.insert(object.record) {
                            continue;
                        }

                        QueuedObject::Record(object.record)
                    }
                },
                ObjectValue::Record(id, _) => {
                    if !records_visited.insert(*id) {
                        continue;
                    }

                    QueuedObject::Record(*id)
                }
            };

            objects.push(object);
        }

        self.queue.push_back(EventQueueEntry {
            event,
            objects,
            entities,
        });
    }
}

#[derive(Clone, Debug)]
struct EventQueueEntry {
    event: QueuedEvent,
    /// The list of objects that must be loaded for this event.
    objects: Vec<QueuedObject>,
    entities: HashMap<Entity, RecordId>,
}

#[derive(Clone, Debug)]
enum QueuedObject {
    Record(RecordId),
}

#[derive(Clone, Debug)]
struct EventEntry {
    event: QueuedEvent,
    /// Number of objects that are still loading.
    num_loading: usize,
    /// Objects that are referenced by this event.
    objects: HashSet<RecordId>,
    entities: HashMap<Entity, RecordId>,
}

#[derive(Clone, Debug)]
struct LoadEntry {
    /// Events that are waiting for this entry to become ready.
    events: HashSet<usize>,
    value: Option<ScriptObject>,
}

#[derive(Clone, Debug)]
enum LoadError<'a> {
    FailedToLoad {
        record: RecordId,
    },
    NoClass {
        object: ScriptsRef<'a>,
        name: &'static str,
    },
}

impl Display for LoadError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FailedToLoad { record } => {
                write!(f, "script for record {record} failed to load")
            }
            Self::NoClass { object, name } => {
                write!(f, "object has no attached script {name} (available are: ")?;
                let mut iter = object.iter();
                if let Some(script) = iter.next() {
                    write!(f, "{}", script.name())?;
                }

                for script in iter {
                    write!(f, ", {}", script.name())?;
                }

                write!(f, ")")
            }
        }
    }
}
