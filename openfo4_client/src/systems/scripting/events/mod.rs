pub mod queue;

use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_script::runtime::{ObjectAddr, ObjectRef};

use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent};
use crate::events::trigger::{TriggerEnter, TriggerLeave};
use crate::systems::DeathStartEvent;
use crate::systems::inventory::Inventory;
use crate::systems::scripting::invocations::Value;
use crate::systems::scripting::loader::{ScriptObject, ScriptObjectId};
use crate::systems::scripting::registration::EventKind;

#[derive(Copy, Clone)]
pub struct Context<'a> {
    pub world: &'a World,
}

#[derive(Clone, Debug)]
pub struct DispatchEvent {
    /// The script that receives the event.
    pub receiver: ScriptObjectId,
    pub kind: EventKind,
    pub args: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct QueuedEvent {
    pub receiver: QueuedEventValue,
    pub kind: EventKind,
    pub args: Vec<QueuedEventValue>,
}

#[derive(Clone, Debug)]
pub enum QueuedEventValue {
    Object(ObjectValue),
    Int(i32),
}

impl QueuedEventValue {
    fn to_raw(
        &self,
        get_entity: impl Fn(Entity, &str) -> ObjectRef,
        get_record: impl Fn(RecordId, &str) -> ObjectRef,
        get_object: impl Fn(&ScriptObject, &str) -> ObjectRef,
    ) -> Value {
        match self {
            Self::Object(ObjectValue::None) => Value::ObjectAddr(ObjectAddr::NULL),
            Self::Object(ObjectValue::Entity(entity, name)) => {
                Value::ObjectRef(get_entity(*entity, &name))
            }
            Self::Object(ObjectValue::Record(id, name)) => Value::ObjectRef(get_record(*id, &name)),
            Self::Object(ObjectValue::Script(object, name)) => {
                Value::ObjectRef(get_object(object, &name))
            }
            Self::Int(val) => Value::Int(*val),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum ObjectValue {
    #[default]
    None,
    Entity(Entity, &'static str),
    Record(RecordId, &'static str),
    Script(ScriptObject, &'static str),
}

impl ObjectValue {
    fn entity_opt(entity: Option<Entity>, script: &'static str) -> Self {
        match entity {
            Some(entity) => Self::Entity(entity, script),
            None => Self::None,
        }
    }
}

pub trait ExtractEvent {
    fn extract(&self, ctx: Context<'_>) -> Vec<QueuedEvent>;
}

impl ExtractEvent for ItemAddedEvent {
    fn extract(&self, ctx: Context<'_>) -> Vec<QueuedEvent> {
        let Ok(inventory) = ctx.world.get::<&Inventory>(self.entity) else {
            return Vec::new();
        };

        let Some(stack) = inventory.get(self.slot) else {
            return Vec::new();
        };

        let item_ref = stack
            .script
            .as_ref()
            .map(|obj| ObjectValue::Script(obj.clone(), "ObjectReference"))
            .unwrap_or_default();

        let source = self
            .source
            .map(|source| ObjectValue::Entity(source, "ObjectReference"))
            .unwrap_or_default();

        // OnItemAdded
        vec![QueuedEvent {
            receiver: QueuedEventValue::Object(ObjectValue::Entity(self.entity, "ObjectReference")),
            kind: EventKind::ItemAdded,
            args: vec![
                // akBaseItem
                QueuedEventValue::Object(ObjectValue::Record(stack.item, "Form")),
                // aiItemCount
                QueuedEventValue::Int(self.count.get().try_into().unwrap_or(i32::MAX)),
                // akItemReference
                QueuedEventValue::Object(item_ref),
                // akSourceContainer
                QueuedEventValue::Object(source),
            ],
        }]
    }
}

impl ExtractEvent for ItemEquippedEvent {
    fn extract(&self, ctx: Context<'_>) -> Vec<QueuedEvent> {
        let Ok(inventory) = ctx.world.get::<&Inventory>(self.actor) else {
            return Vec::new();
        };

        let Some(stack) = inventory.get(self.slot) else {
            return Vec::new();
        };

        let item_ref = stack
            .script
            .as_ref()
            .map(|obj| ObjectValue::Script(obj.clone(), "ObjectReference"))
            .unwrap_or_default();

        // OnItemEquipped
        let mut events = vec![QueuedEvent {
            receiver: QueuedEventValue::Object(ObjectValue::Entity(self.actor, "Actor")),
            kind: EventKind::ItemEquipped,
            args: vec![
                // akBaseObject
                QueuedEventValue::Object(ObjectValue::Record(stack.item, "Form")),
                // akReference
                QueuedEventValue::Object(item_ref.clone()),
            ],
        }];

        // OnEquipped
        if stack.script.is_some() {
            events.push(QueuedEvent {
                receiver: QueuedEventValue::Object(item_ref),
                kind: EventKind::Equipped,
                args: vec![
                    // akActor
                    QueuedEventValue::Object(ObjectValue::Entity(self.actor, "Actor")),
                ],
            });
        }

        events
    }
}

impl ExtractEvent for TriggerEnter {
    fn extract(&self, _ctx: Context<'_>) -> Vec<QueuedEvent> {
        // OnTriggerEnter
        vec![QueuedEvent {
            receiver: QueuedEventValue::Object(ObjectValue::Entity(self.zone, "ObjectReference")),
            kind: EventKind::TriggerEnter,
            args: vec![
                // akActionRef
                QueuedEventValue::Object(ObjectValue::Entity(self.entity, "ObjectReference")),
            ],
        }]
    }
}

impl ExtractEvent for TriggerLeave {
    fn extract(&self, _ctx: Context<'_>) -> Vec<QueuedEvent> {
        // OnTriggerLeave
        vec![QueuedEvent {
            receiver: QueuedEventValue::Object(ObjectValue::Entity(self.zone, "ObjectReference")),
            kind: EventKind::TriggerLeave,
            args: vec![
                // akActionRef
                QueuedEventValue::Object(ObjectValue::Entity(self.entity, "ObjectReference")),
            ],
        }]
    }
}

impl ExtractEvent for DeathStartEvent {
    fn extract(&self, _ctx: Context<'_>) -> Vec<QueuedEvent> {
        let mut events = Vec::new();

        // OnDying
        events.push(QueuedEvent {
            receiver: QueuedEventValue::Object(ObjectValue::Entity(self.actor, "Actor")),
            kind: EventKind::Dying,
            args: vec![
                // akKiller
                QueuedEventValue::Object(ObjectValue::entity_opt(self.killer, "Actor")),
            ],
        });

        // OnKill
        if let Some(killer) = self.killer {
            events.push(QueuedEvent {
                receiver: QueuedEventValue::Object(ObjectValue::Entity(killer, "Actor")),
                kind: EventKind::Kill,
                args: vec![
                    // akVictim
                    QueuedEventValue::Object(ObjectValue::Entity(self.actor, "Actor")),
                ],
            });
        }

        events
    }
}
