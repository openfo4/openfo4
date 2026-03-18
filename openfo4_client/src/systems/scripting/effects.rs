use std::any::Any;
use std::collections::VecDeque;
use std::num::NonZeroU64;
use std::sync::Arc;

use game_common::collections::arena::{Arena, Key};
use game_tasks::{Task, TaskPool};
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;

use crate::events::inventory::RemoveItemEvent;
use crate::events::quest::{StartQuestScene, StopQuestScene};
use crate::state::equipment::load_item;
use crate::state::esp::EspLoader;
use crate::systems::inventory::{
    AddItemEvent, EquipItemEvent, InventoryKey, ItemStack, UnequipItemEvent,
};
use crate::systems::scripting::loader::ScriptLoader;

#[derive(Default)]
pub struct Effects {
    effects: Arena<EffectState>,
    queue: VecDeque<(EffectId, Box<dyn DynamicEffect>)>,
}

impl Effects {
    pub fn push<T>(&mut self, effect: T) -> EffectId
    where
        T: DynamicEffect,
    {
        let key = EffectId(self.effects.insert(EffectState {
            num_children: 1,
            tasks: Vec::new(),
            value: None,
        }));
        self.queue.push_back((key, Box::new(effect)));
        key
    }

    pub fn discard(&mut self, id: EffectId) {
        self.effects.remove(id.0);
    }

    pub fn take_value(&mut self, id: EffectId) -> Option<Box<dyn Any>> {
        let effect = self.effects.get_mut(id.0)?;
        effect.value.take()
    }

    pub fn run(
        &mut self,
        pool: &TaskPool,
        world: &mut World,
        events: &mut EventBus,
        esp_loader: &Arc<EspLoader>,
        script_loader: &Arc<ScriptLoader>,
    ) -> Vec<EffectId> {
        let mut effects_done = Vec::new();

        for (id, effect) in &mut self.effects {
            effect.tasks.retain_mut(|task| {
                if let Some(effect) = task.get_output() {
                    self.queue.push_back((EffectId(id), effect));
                    false
                } else {
                    true
                }
            });
        }

        while let Some((id, effect)) = self.queue.pop_front() {
            let ctx = Context {
                pool,
                world,
                events,
                esp_loader,
                script_loader,
            };

            match effect.run(ctx) {
                EffectResult::Done => {}
                EffectResult::Forked(tasks) => {
                    let effect = self.effects.get_mut(id.0).unwrap();
                    effect.num_children += tasks.len();
                    effect.tasks.extend(tasks);
                }
                EffectResult::DoneWithValue(value) => {
                    let effect = self.effects.get_mut(id.0).unwrap();
                    effect.value = Some(value);
                }
            }

            let effect = self.effects.get_mut(id.0).unwrap();
            effect.num_children -= 1;
            if effect.num_children == 0 {
                effects_done.push(id);
            }
        }

        effects_done
    }
}

struct EffectState {
    num_children: usize,
    tasks: Vec<Task<Box<dyn DynamicEffect>>>,
    value: Option<Box<dyn Any>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(Key);

pub trait Effect: DynamicEffect {
    type Result: Any;
}

pub trait DynamicEffect: 'static + Send {
    fn run(&self, ctx: Context<'_>) -> EffectResult;
}

pub enum EffectResult {
    Done,
    Forked(Vec<Task<Box<dyn DynamicEffect>>>),
    DoneWithValue(Box<dyn Any>),
}

pub struct Context<'a> {
    pool: &'a TaskPool,
    world: &'a mut World,
    events: &'a mut EventBus,
    esp_loader: &'a Arc<EspLoader>,
    script_loader: &'a Arc<ScriptLoader>,
}

#[derive(Copy, Clone, Debug)]
pub struct AddItemEffect {
    pub entity: Entity,
    pub item: RecordId,
    pub count: u64,
}

impl Effect for AddItemEffect {
    type Result = ();
}

impl DynamicEffect for AddItemEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        let esp_loader = ctx.esp_loader.clone();
        let script_loader = ctx.script_loader.clone();

        let entity = self.entity;
        let item = self.item;
        let count = self.count;

        let task = ctx.pool.spawn(async move {
            let items = load_item(&esp_loader, &script_loader, item, &[], count).await;
            Box::new(AddItemsImmediate { entity, items }) as Box<dyn DynamicEffect>
        });

        EffectResult::Forked(vec![task])
    }
}

#[derive(Clone, Debug)]
struct AddItemsImmediate {
    entity: Entity,
    items: Vec<ItemStack>,
}

impl Effect for AddItemsImmediate {
    type Result = ();
}

impl DynamicEffect for AddItemsImmediate {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        for item in &self.items {
            ctx.events.send(AddItemEvent {
                actor: self.entity,
                item: item.clone(),
            });
        }

        EffectResult::Done
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RemoveItemEffect {
    pub entity: Entity,
    pub item: InventoryKey,
    pub count: NonZeroU64,
}

impl Effect for RemoveItemEffect {
    type Result = ();
}

impl DynamicEffect for RemoveItemEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        ctx.events.send(RemoveItemEvent {
            entity: self.entity,
            slot: self.item,
            count: self.count,
        });

        EffectResult::Done
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TransferItemEffect {
    pub source: Entity,
    pub destination: Entity,
    pub item: InventoryKey,
    pub count: NonZeroU64,
}

impl Effect for TransferItemEffect {
    type Result = ();
}

impl DynamicEffect for TransferItemEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        ctx.events.send(TransferItemEffect {
            source: self.source,
            destination: self.destination,
            item: self.item,
            count: self.count,
        });

        EffectResult::Done
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EquipItemEffect {
    pub actor: Entity,
    pub item: RecordId,
}

impl Effect for EquipItemEffect {
    type Result = ();
}

impl DynamicEffect for EquipItemEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        ctx.events.send(EquipItemEvent {
            actor: self.actor,
            item: self.item,
        });

        EffectResult::Done
    }
}

#[derive(Copy, Clone, Debug)]
pub struct UnequipItemEffect {
    pub actor: Entity,
    pub slot: InventoryKey,
}

impl Effect for UnequipItemEffect {
    type Result = ();
}

impl DynamicEffect for UnequipItemEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        ctx.events.send(UnequipItemEvent {
            actor: self.actor,
            item: self.slot,
        });

        EffectResult::Done
    }
}

#[derive(Copy, Clone, Debug)]
pub struct StartQuestSceneEffect {
    pub scene: RecordId,
}

impl Effect for StartQuestSceneEffect {
    type Result = ();
}

impl DynamicEffect for StartQuestSceneEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        ctx.events.send(StartQuestScene { scene: self.scene });
        EffectResult::Done
    }
}

#[derive(Copy, Clone, Debug)]
pub struct StopQuestSceneEffect {
    pub scene: RecordId,
}

impl Effect for StopQuestSceneEffect {
    type Result = ();
}

impl DynamicEffect for StopQuestSceneEffect {
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        ctx.events.send(StopQuestScene { scene: self.scene });
        EffectResult::Done
    }
}
