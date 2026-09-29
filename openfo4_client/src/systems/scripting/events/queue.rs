use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use game_tasks::{TaskPool, TaskQueue};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use slab::Slab;

use crate::systems::scripting::invocations::Value;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject, ScriptObjectId};
use crate::systems::scripting::objects::{ScriptObjects, ScriptsRef};
use crate::systems::scripting::{LoadRecordKind, ScriptEntities};

use super::{DispatchEvent, ObjectValue, QueuedEvent, QueuedEventValue};

#[derive(Debug, Default)]
pub struct EventQueue {
    events: Slab<EventEntry>,
    queue: Arc<SegQueue<EventQueueEntry>>,
    tasks: TaskQueue<(usize, usize, ScriptObject)>,
    ready_events: Vec<usize>,
}

impl EventQueue {
    pub fn process(
        &mut self,
        objects: &ScriptObjects,
        pool: &TaskPool,
        loader: &Arc<ScriptLoader>,
        script_entities: &ScriptEntities,
    ) -> Vec<DispatchEvent> {
        let _span = trace_span!("EventQueue::process").entered();

        while let Some(event) = self.queue.pop() {
            let entry = self.events.vacant_entry();
            let index = entry.key();

            let num_objects = event.entities.len() + event.records.len();
            let mut objs = vec![None; num_objects];
            let mut num_loading = 0;

            let mut entities = HashMap::new();
            for (i, (entity, record)) in event.entities.into_iter().enumerate() {
                entities.insert(entity, i);

                if let Some(id) = script_entities.get(entity) {
                    objs[i] = Some(id);
                    continue;
                }

                let loader = Arc::clone(loader);
                self.tasks.push(pool.spawn(async move {
                    let object = record.load(&loader).await;
                    (index, i, object)
                }));
                num_loading += 1;
            }

            let offset = entities.len();
            let mut records = HashMap::new();
            for (i, record) in event.records.into_iter().enumerate() {
                let i = offset + i;
                records.insert(record, i);

                if let Some(scripts) = objects.get_by_record(record) {
                    objs[i] = Some(scripts.id());
                    continue;
                }

                let loader = Arc::clone(loader);
                self.tasks.push(pool.spawn(async move {
                    let object = loader.load(record).await.unwrap().object;
                    (index, i, object)
                }));
                num_loading += 1;
            }

            entry.insert(EventEntry {
                event: event.event,
                num_loading,
                objects: objs,
                entities,
                records,
            });

            if num_loading == 0 {
                self.ready_events.push(index);
            }
        }

        while let Some((event_index, object_index, object)) = self.tasks.pop() {
            let event = self.events.get_mut(event_index).unwrap();

            event.num_loading -= 1;
            event.objects[object_index] = Some(object.id());

            if event.num_loading == 0 {
                self.ready_events.push(event_index);
            }
        }

        let mut events = Vec::new();
        while let Some(key) = self.ready_events.pop() {
            let event = self.events.remove(key);

            debug_assert_eq!(event.num_loading, 0);

            let convert_object = |object| -> Result<Value, LoadError<'_>> {
                match object {
                    ObjectValue::None => Ok(Value::Null),
                    ObjectValue::Script(object, name) => {
                        let object = objects.get_by_id(object.id()).unwrap();
                        let Some(script) = object.get(name) else {
                            return Err(LoadError::NoClass { object, name });
                        };

                        Ok(Value::Object(script.get().object.clone()))
                    }
                    ObjectValue::Entity(entity, name) => {
                        let index = *event.entities.get(&entity).unwrap();
                        let object = event.objects[index].unwrap();

                        let object = objects.get_by_id(object).unwrap();
                        let Some(script) = object.get(name) else {
                            return Err(LoadError::NoClass { object, name });
                        };

                        Ok(Value::Object(script.get().object.clone()))
                    }
                    ObjectValue::Record(record, name) => {
                        let index = *event.records.get(&record).unwrap();
                        let object = event.objects[index].unwrap();

                        let object = objects.get_by_id(object).unwrap();
                        let Some(script) = object.get(name) else {
                            return Err(LoadError::NoClass { object, name });
                        };

                        Ok(Value::Object(script.get().object.clone()))
                    }
                }
            };

            let receiver = match event.event.receiver {
                QueuedEventValue::Object(object) => match convert_object(object) {
                    Ok(object) => match object {
                        Value::Object(v) => {
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

            events.push(DispatchEvent {
                receiver,
                kind: event.event.kind,
                args,
            });
        }

        events
    }

    pub fn producer(&self) -> EventQueueProducer {
        EventQueueProducer {
            queue: self.queue.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EventQueueProducer {
    queue: Arc<SegQueue<EventQueueEntry>>,
}

impl EventQueueProducer {
    pub fn push(&mut self, world: &World, mut event: QueuedEvent) {
        let mut entities = HashMap::new();
        let mut records = HashSet::new();

        for value in event
            .args
            .iter_mut()
            .chain(std::iter::once(&mut event.receiver))
        {
            let QueuedEventValue::Object(object) = value else {
                continue;
            };

            match object {
                ObjectValue::None => continue,
                ObjectValue::Script(_, _) => continue,
                ObjectValue::Entity(entity, name) => match world.get::<&ScriptObject>(*entity) {
                    Ok(s) => {
                        *object = ObjectValue::Script(s.clone(), name);
                        continue;
                    }
                    Err(_) => {
                        if entities.contains_key(entity) {
                            continue;
                        }

                        let object = LoadRecordKind::extract(world, *entity);
                        entities.insert(*entity, object);
                    }
                },
                ObjectValue::Record(id, _) => {
                    if records.contains(id) {
                        continue;
                    }

                    records.insert(*id);
                }
            }
        }

        self.queue.push(EventQueueEntry {
            event,
            entities,
            records,
        });
    }
}

#[derive(Clone, Debug)]
struct EventQueueEntry {
    event: QueuedEvent,
    /// The list of entity records that must be loaded.
    entities: HashMap<Entity, LoadRecordKind>,
    /// The list of base records that must be loaded.
    records: HashSet<RecordId>,
}

#[derive(Clone, Debug)]
struct EventEntry {
    event: QueuedEvent,
    /// Number of objects that are still loading.
    num_loading: usize,
    objects: Vec<Option<ScriptObjectId>>,
    entities: HashMap<Entity, usize>,
    records: HashMap<RecordId, usize>,
}

#[derive(Clone, Debug)]
enum LoadError<'a> {
    // FailedToLoad {
    //     record: RecordId,
    // },
    NoClass {
        object: ScriptsRef<'a>,
        name: &'static str,
    },
}

impl Display for LoadError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            // Self::FailedToLoad { record } => {
            //     write!(f, "script for record {record} failed to load")
            // }
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
