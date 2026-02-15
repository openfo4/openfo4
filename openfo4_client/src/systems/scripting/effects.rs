use std::collections::VecDeque;
use std::sync::Arc;

use game_common::collections::arena::{Arena, Key};
use game_tasks::{Task, TaskPool};
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;

use crate::state::equipment::load_item;
use crate::state::esp::EspLoader;
use crate::systems::inventory::{AddItemEvent, ItemStack};
use crate::systems::scripting::loader::ScriptLoader;

#[derive(Default)]
pub struct Effects {
    effects: Arena<EffectState>,
    queue: VecDeque<(EffectId, Box<dyn Effect>)>,
}

impl Effects {
    pub fn push<T>(&mut self, effect: T) -> EffectId
    where
        T: Effect,
    {
        let key = EffectId(self.effects.insert(EffectState {
            num_children: 1,
            tasks: Vec::new(),
        }));
        self.queue.push_back((key, Box::new(effect)));
        key
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
            }

            let effect = self.effects.get_mut(id.0).unwrap();
            effect.num_children -= 1;
            if effect.num_children == 0 {
                self.effects.remove(id.0);
                effects_done.push(id);
            }
        }

        effects_done
    }
}

struct EffectState {
    num_children: usize,
    tasks: Vec<Task<Box<dyn Effect>>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(Key);

pub trait Effect: 'static + Send {
    fn run(&self, ctx: Context<'_>) -> EffectResult;
}

enum EffectResult {
    Done,
    Forked(Vec<Task<Box<dyn Effect>>>),
}

struct Context<'a> {
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
    fn run(&self, ctx: Context<'_>) -> EffectResult {
        let esp_loader = ctx.esp_loader.clone();
        let script_loader = ctx.script_loader.clone();

        let entity = self.entity;
        let item = self.item;
        let count = self.count;

        let task = ctx.pool.spawn(async move {
            let items = load_item(&esp_loader, &script_loader, item, &[], count).await;
            Box::new(AddItemsImmediate { entity, items }) as Box<dyn Effect>
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
