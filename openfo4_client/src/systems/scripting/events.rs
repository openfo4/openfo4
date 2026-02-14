use hecs::World;
use openfo4_script::runtime::ObjectAddr;

use crate::components::Player;
use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent};
use crate::events::trigger::{TriggerEnter, TriggerLeave};
use crate::systems::inventory::Inventory;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::classes::State;
use crate::systems::scripting::invocations::Value;
use crate::systems::scripting::loader::ScriptObjectId;
use crate::systems::scripting::registration::EventKind;

#[derive(Copy, Clone)]
pub struct Context<'a> {
    pub state: &'a State,
    pub world: &'a World,
    pub entities: &'a ScriptEntities,
}

#[derive(Clone, Debug)]
pub struct DispatchEvent {
    pub receiver: ScriptObjectId,
    pub kind: EventKind,
    pub args: Vec<Value>,
}

pub trait Event {
    fn extract(&self, ctx: Context<'_>) -> Vec<DispatchEvent>;
}

impl Event for ItemAddedEvent {
    fn extract(&self, ctx: Context<'_>) -> Vec<DispatchEvent> {
        let Ok(inventory) = ctx.world.get::<&Inventory>(self.entity) else {
            return Vec::new();
        };

        let Some(stack) = inventory.get(self.slot) else {
            return Vec::new();
        };

        let base_item = ctx.state.objects.get_by_record(stack.item).unwrap();
        let base_item = base_item.iter().next().unwrap();

        // TODO: Get or create
        let Some(actor) = ctx.entities.0.get_right(&self.entity) else {
            todo!();
        };

        // Close enough
        let count = i32::try_from(self.count.get()).unwrap_or(i32::MAX);

        vec![DispatchEvent {
            receiver: *actor,
            kind: EventKind::ItemAdded,
            args: vec![
                Value::ObjectRef(base_item.get().object.clone()),
                Value::Int(count),
                //  TODO
                Value::ObjectAddr(ObjectAddr::NULL),
            ],
        }]
    }
}

impl Event for ItemEquippedEvent {
    fn extract(&self, ctx: Context<'_>) -> Vec<DispatchEvent> {
        let Ok(inventory) = ctx.world.get::<&Inventory>(self.actor) else {
            return Vec::new();
        };

        let Some(stack) = inventory.get(self.slot) else {
            return Vec::new();
        };

        let base_item = ctx.state.objects.get_by_record(stack.item).unwrap();
        let base_item = base_item.iter().next().unwrap();

        // TODO: Get or create
        let Some(actor) = ctx.entities.0.get_right(&self.actor) else {
            return Vec::new();
        };

        vec![DispatchEvent {
            receiver: *actor,
            kind: EventKind::ItemEquipped,
            args: vec![
                Value::ObjectRef(base_item.get().object.clone()),
                // TODO
                Value::ObjectAddr(ObjectAddr::NULL),
            ],
        }]
    }
}

impl Event for TriggerEnter {
    fn extract(&self, ctx: Context<'_>) -> Vec<DispatchEvent> {
        let Some(zone) = ctx.entities.0.get_right(&self.zone) else {
            return Vec::new();
        };

        let Some(entity) = ctx.entities.0.get_right(&self.entity) else {
            return Vec::new();
        };

        let entity = ctx.state.objects.get_by_id(*entity).unwrap();
        let entity = entity.get("ObjectReference").unwrap();

        vec![DispatchEvent {
            receiver: *zone,
            kind: EventKind::TriggerEnter,
            args: vec![Value::ObjectRef(entity.get().object.clone())],
        }]
    }
}

impl Event for TriggerLeave {
    fn extract(&self, ctx: Context<'_>) -> Vec<DispatchEvent> {
        let Some(zone) = ctx.entities.0.get_right(&self.zone) else {
            return Vec::new();
        };

        let Some(entity) = ctx.entities.0.get_right(&self.entity) else {
            return Vec::new();
        };

        let entity = ctx.state.objects.get_by_id(*entity).unwrap();
        let entity = entity.get("ObjectReference").unwrap();

        vec![DispatchEvent {
            receiver: *zone,
            kind: EventKind::TriggerLeave,
            args: vec![Value::ObjectRef(entity.get().object.clone())],
        }]
    }
}
