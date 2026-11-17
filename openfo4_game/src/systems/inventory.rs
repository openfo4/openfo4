mod visibility;

use std::fmt::Debug;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use hashbrown::{HashMap, HashSet};
use openfo4_asset::AssetLoader;
use openfo4_common::components::Transform;
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_tasks::{TaskPool, TaskQueue};
use openfo4_tracing::trace_span;
use slab::Slab;

use crate::components::PlacedItem;
use crate::events::inventory::{
    ItemAddedEvent, ItemEquippedEvent, ItemUnequippedEvent, RemoveItemEvent,
};
use crate::state::equipment::{EquipmentItemModels, load_item};
use crate::state::esp::EspLoader;
use crate::state::hardcoded::KEYWORD_FEMALE;
use crate::state::npc::ObjectKeywords;
use crate::systems::equipment::{EquipmentKey, unequip_item};
use crate::systems::inventory::visibility::VisibilityTracker;
use crate::systems::scripting::Executor;
use crate::systems::scripting::loader::ScriptObject;
use crate::systems::{PickupItemEvent, TransferItemEvent};

/// Unique identifier for an item inside a container.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InventoryKey(usize);

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    items: Slab<ItemStack>,
    by_id: HashMap<RecordId, Vec<usize>>,
    visibility: VisibilityTracker,
}

impl Inventory {
    pub fn insert(&mut self, item: ItemStack) -> InventoryKey {
        // Attempt to merge the new items into an existing stack.
        for index in self.by_id.get(&item.item).into_iter().flatten() {
            if self.items[*index].merge(&item) {
                return InventoryKey(*index);
            }
        }

        let form = item.item;
        let index = self.items.insert(item);
        self.by_id.entry(form).or_default().push(index);

        InventoryKey(index)
    }

    pub fn remove(&mut self, id: InventoryKey) {
        if !self.items.contains(id.0) {
            return;
        }

        let item = self.items.remove(id.0);

        let keys = self.by_id.get_mut(&item.item).unwrap();
        keys.retain(|k| *k != id.0);
        if keys.is_empty() {
            self.by_id.remove(&item.item);
        }
    }

    pub fn get(&self, id: InventoryKey) -> Option<&ItemStack> {
        self.items.get(id.0)
    }

    pub fn get_mut(&mut self, id: InventoryKey) -> Option<&mut ItemStack> {
        self.items.get_mut(id.0)
    }

    pub fn get_by_form(
        &self,
        record: RecordId,
    ) -> impl Iterator<Item = (InventoryKey, &ItemStack)> {
        self.by_id
            .get(&record)
            .into_iter()
            .map(|v| v.into_iter())
            .flatten()
            .map(|v| (InventoryKey(*v), &self.items[*v]))
    }

    pub fn items(&self) -> impl Iterator<Item = (InventoryKey, &ItemStack)> {
        self.items.iter().map(|(k, v)| (InventoryKey(k), v))
    }

    pub fn items_mut(&mut self) -> impl Iterator<Item = (InventoryKey, &mut ItemStack)> {
        self.items.iter_mut().map(|(k, v)| (InventoryKey(k), v))
    }
}

#[derive(Clone, Debug)]
pub struct ItemStack {
    pub item: RecordId,
    pub name: String,
    /// Whether the item should not be seen by the player in UIs.
    pub hidden: bool,
    pub kind: ItemKind,
    pub count: u64,
    pub attachments: Vec<RecordId>,
    pub equipped: bool,
    pub base_keywords: HashSet<RecordId>,
    pub combined_keywords: HashSet<RecordId>,
    pub data: ItemData,
    pub equipment_keys: Vec<EquipmentKey>,
    pub equipment_slots: HashSet<u32>,
    pub script: Option<ScriptObject>,
    pub visibility: ItemVisibilityFlags,
    pub visible: HashSet<u32>,
    pub resistances: HashMap<RecordId, u32>,
}

impl ItemStack {
    /// Returns `true` if `other` can be merged into `self`.
    fn mergeable(&self, other: &Self) -> bool {
        self.item == other.item
            && self.hidden == other.hidden
            && self.attachments == other.attachments
            && self.base_keywords == other.base_keywords
            && self.combined_keywords == other.combined_keywords
            && self.script.is_none()
            && other.script.is_none()
    }

    fn merge(&mut self, other: &Self) -> bool {
        if self.mergeable(other) {
            self.count += other.count;
            true
        } else {
            false
        }
    }

    /// Returns whether the item is a "real" game item.
    ///
    /// This returns `false` for any items that are not tracked as such from the gameplay
    /// perspecitive, like skins or head parts.
    pub fn is_game_item(&self) -> bool {
        !self.hidden
    }
}

#[derive(Clone, Debug)]
pub enum ItemData {
    Weapon(WeaponItemData),
    Armor,
    None,
}

#[derive(Clone, Debug)]
pub struct WeaponItemData {
    /// Whether the weapon is currently drawn.
    ///
    /// This is different from the [`equipped`] bit, which only defines whether the weapon is in
    /// the active slot.
    ///
    /// [`equipped`]: ItemStack::equipped
    pub drawn: bool,
    /// Current ammo count of the weapon.
    pub current_ammo: u32,
    /// Maximum ammo count.
    pub ammo_capacity: u32,
    /// Damage of the weapon projectile for each damage type.
    ///
    /// The damage defined on the `AMMO` record is already included in this.
    pub damage: HashMap<RecordId, u32>,
    pub aim_props: AimProps,
    /// Ammo consumed by this weapon.
    pub ammo: Option<RecordId>,
    /// The node where weapon projectiles should be spawned.
    ///
    /// Only valid while the weapon is equipped and drawn.
    pub projectile_node: Option<WeaponProjectileNode>,
    /// Whether NPCs use ammo when reloading.
    ///
    /// If `false` NPCs have infinite ammo.
    pub npcs_use_ammo: bool,
}

/// Defines properties for the aiming of guns.
#[derive(Clone, Debug, PartialEq)]
pub struct AimProps {
    /// Min and max cone of fire angles in radians.
    pub cone_of_fire_angle_min: f32,
    pub cone_of_fire_angle_max: f32,
    /// Angle added to cone of fire per shot in radians.
    pub cone_of_fire_inc_per_shot: f32,
    pub cone_of_fire_dec_per_sec: f32,
    /// The idle time (no shots) fired until `cone_of_fire_dec_per_sec` kicks in.
    pub cone_of_fire_dec_delay: Duration,
}

#[derive(Copy, Clone, Debug)]
pub struct WeaponProjectileNode {
    pub entity: Entity,
    pub offset: Transform,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Weapon,
    Armor,
    Ammo,
    Misc,
    Aid,
    Junk,
    HeadPart,
}

impl ItemKind {
    const fn is_equippable(&self) -> bool {
        matches!(self, Self::Weapon | Self::Armor | Self::HeadPart)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ItemVisibilityFlags {
    pub slots: HashSet<u32>,
    pub priority: u8,
}

#[derive(Clone, Debug)]
pub struct AddItemEvent {
    pub actor: Entity,
    pub item: ItemStack,
}

/// Queues an item to be added to an inventory.
#[derive(Clone, Debug)]
pub struct AddItemEventAsync {
    pub actor: Entity,
    pub item: RecordId,
    pub attachments: Vec<RecordId>,
    pub count: u64,
    pub hidden: bool,
    /// Cause the item to get equipped when loaded.
    pub equip: bool,
}

#[derive(Clone, Debug)]
pub struct EquipItemEvent {
    pub actor: Entity,
    // FIXME: Change to `InventoryKey`.
    pub item: RecordId,
}

#[derive(Clone, Debug)]
pub struct UnequipItemEvent {
    pub actor: Entity,
    pub item: InventoryKey,
}

#[derive(Debug)]
pub struct InventorySystem {
    pool: Arc<TaskPool>,
    loader: AssetLoader,
    esp_loader: Arc<EspLoader>,
    tasks: TaskQueue<(Entity, InventoryKey, EquipmentItemModels, HashSet<u32>)>,
    async_item_add_tasks: TaskQueue<(Vec<AddItemEvent>, Vec<EquipItemEvent>)>,
}

impl InventorySystem {
    pub fn new(pool: Arc<TaskPool>, loader: AssetLoader, esp_loader: Arc<EspLoader>) -> Self {
        Self {
            pool,
            loader,
            esp_loader,
            tasks: TaskQueue::new(),
            async_item_add_tasks: TaskQueue::new(),
        }
    }

    pub fn handle_inventory_events(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        executor: &mut Executor,
    ) {
        let _span = trace_span!("handle_inventory_events").entered();

        for event in events.read::<AddItemEventAsync>() {
            let esp_loader = self.esp_loader.clone();
            let script_loader = executor.loader().clone();
            let event = event.clone();

            self.async_item_add_tasks.push(self.pool.spawn(async move {
                let mut items = load_item(
                    &esp_loader,
                    &script_loader,
                    event.item,
                    &event.attachments,
                    event.count,
                )
                .await;

                for item in &mut items {
                    item.hidden = event.hidden;
                }

                let equip_events = if event.equip {
                    items
                        .iter()
                        .map(|item| EquipItemEvent {
                            actor: event.actor,
                            item: item.item,
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                let add_events = items
                    .into_iter()
                    .map(|item| AddItemEvent {
                        actor: event.actor,
                        item,
                    })
                    .collect();

                (add_events, equip_events)
            }));
        }

        while let Some((add_events, equip_events)) = self.async_item_add_tasks.pop() {
            for event in add_events {
                events.send(event);
            }

            for event in equip_events {
                events.send(event);
            }
        }

        // Before we transfer anything we must first unequip it.
        let unequip_transfer_events = events
            .read::<TransferItemEvent>()
            .map(|e| UnequipItemEvent {
                actor: e.source,
                item: e.item,
            })
            .collect::<Vec<_>>();
        let unequip_remove_events = events
            .read::<RemoveItemEvent>()
            .map(|e| UnequipItemEvent {
                actor: e.entity,
                item: e.slot,
            })
            .collect::<Vec<_>>();
        events.extend(unequip_transfer_events);
        events.extend(unequip_remove_events);

        let mut items_added = Vec::new();
        for event in events.read::<AddItemEvent>() {
            let Some(count) = NonZeroU64::new(event.item.count) else {
                continue;
            };

            let Ok(mut inventory) = world.get::<&mut Inventory>(event.actor) else {
                continue;
            };

            let slot = inventory.insert(event.item.clone());

            items_added.push(ItemAddedEvent {
                entity: event.actor,
                slot,
                count,
                source: None,
            });
        }

        events.extend(items_added);

        self.handle_equip_unequip(world, events);

        // Transfering items must come after all unequip events have been processed.
        // This ensures that any items to transfer are not equipped.
        for event in events.read::<TransferItemEvent>() {
            // Transfering in the same container is a noop.
            if event.source == event.destination {
                continue;
            }

            let (Ok(mut src_inventory), Ok(mut dst_inventory)) = (
                world.get::<&mut Inventory>(event.source),
                world.get::<&mut Inventory>(event.destination),
            ) else {
                continue;
            };

            let Some(stack) = src_inventory.get_mut(event.item) else {
                continue;
            };

            let count = u64::min(event.count, stack.count);
            stack.count -= count;

            let mut new_stack = stack.clone();
            new_stack.count = count;
            dst_inventory.insert(new_stack);

            if stack.count == 0 {
                src_inventory.remove(event.item);
            }
        }

        // Remove items must come after all unequip events have been processed.
        // This ensures that any items to remove are not equipped.
        for event in events.read::<RemoveItemEvent>() {
            let Ok(mut inventory) = world.get::<&mut Inventory>(event.entity) else {
                continue;
            };

            let Some(stack) = inventory.get_mut(event.slot) else {
                continue;
            };

            let count = u64::min(event.count.get(), stack.count);
            stack.count -= count;

            if stack.count == 0 {
                inventory.remove(event.slot);
            }
        }

        for event in events.read::<PickupItemEvent>() {
            let Ok(mut inventory) = world.get::<&mut Inventory>(event.actor) else {
                continue;
            };

            let Ok(item) = world.get::<&mut PlacedItem>(event.item) else {
                continue;
            };

            inventory.insert(item.0.clone());
            drop(inventory);
            drop(item);

            world.despawn_recursive(event.item);
        }

        self.attach_ready_models(world);
    }

    fn handle_equip_unequip(&mut self, world: &mut World, events: &mut EventBus) {
        let mut unequip_conflicting_events = Vec::new();
        let mut items_equipped = Vec::new();
        let mut visibility_changed = HashSet::new();

        for event in events.read::<EquipItemEvent>() {
            let Ok(mut inventory) = world.get::<&mut Inventory>(event.actor) else {
                continue;
            };

            let Some((key, item)) = inventory.items_mut().find(|(_, v)| v.item == event.item)
            else {
                continue;
            };

            if !item.kind.is_equippable() || item.equipped {
                continue;
            }

            item.equipped = true;
            items_equipped.push(ItemEquippedEvent {
                actor: event.actor,
                slot: key,
            });

            match &mut item.data {
                ItemData::Weapon(weapon) => {
                    // Start without weapon drawn.
                    weapon.drawn = false;
                }
                _ => (),
            }

            let item = inventory.items.get(key.0).unwrap();
            for (other_key, other_item) in inventory.items() {
                if key == other_key {
                    continue;
                }

                // Unequip all other items that occupy the same equipment slot.
                if other_item
                    .equipment_slots
                    .intersection(&item.equipment_slots)
                    .count()
                    != 0
                {
                    unequip_conflicting_events.push(UnequipItemEvent {
                        actor: event.actor,
                        item: other_key,
                    });
                }
            }

            let vis_flags = item.visibility.clone();
            let inventory = &mut *inventory;
            let items = &mut inventory.items;
            inventory.visibility.add(vis_flags, key, |key, slots| {
                let item = items.get_mut(key.0).unwrap();
                item.visible.clear();
                item.visible.extend(slots);
                visibility_changed.insert((event.actor, key));
            });
        }

        events.extend(unequip_conflicting_events);
        events.extend(items_equipped);

        let mut items_unequipped = Vec::new();
        for event in events.read::<UnequipItemEvent>() {
            let Ok(mut inventory) = world.get::<&mut Inventory>(event.actor) else {
                continue;
            };

            let Some(item) = inventory.get_mut(event.item) else {
                continue;
            };

            if !item.kind.is_equippable() || !item.equipped {
                continue;
            }

            item.equipped = false;

            if let ItemData::Weapon(weapon) = &mut item.data {
                weapon.drawn = false;
            }

            items_unequipped.push(ItemUnequippedEvent {
                actor: event.actor,
                slot: event.item,
            });

            let inventory = &mut *inventory;
            let items = &mut inventory.items;
            inventory.visibility.remove(event.item, |key, slots| {
                let item = items.get_mut(key.0).unwrap();
                item.visible.clear();
                item.visible.extend(slots);
                visibility_changed.insert((event.actor, key));
            });
        }

        events.extend(items_unequipped);

        for (entity, key) in visibility_changed {
            let Ok(mut inventory) = world.get::<&mut Inventory>(entity) else {
                continue;
            };

            let item = inventory.items.get_mut(key.0).unwrap();

            if item.equipped {
                let female = world
                    .get::<&ObjectKeywords>(entity)
                    .is_ok_and(|kw| kw.contains(KEYWORD_FEMALE));

                let loader = self.loader.clone();
                let esp_loader = self.esp_loader.clone();
                let record = item.item;
                let attachments = item.attachments.clone();
                let visible = item.visible.clone();
                self.tasks.push(self.pool.spawn(async move {
                    let models = EquipmentItemModels::load(
                        &loader,
                        &esp_loader,
                        record,
                        &attachments,
                        female,
                        &visible,
                    )
                    .await;
                    (entity, key, models, visible)
                }));
            }

            let keys = core::mem::take(&mut item.equipment_keys);
            drop(inventory);
            unequip_item(entity, keys, world);
        }
    }

    fn attach_ready_models(&mut self, world: &mut World) {
        let _span = trace_span!("InventorySystem::attach_ready_models").entered();

        while let Some((entity, key, models, visible)) = self.tasks.pop() {
            // Recheck that we still should attach the scene.
            // The item may already be unequipped again.
            let Ok(inventory) = world.get::<&Inventory>(entity) else {
                continue;
            };

            let Some(item) = inventory.get(key) else {
                continue;
            };

            if !item.equipped || item.visible != visible {
                continue;
            }

            // This indicates that something was already attached to the item.
            // This can happen if the item is rapidly equipped and unequipped,
            // causing the scene to be loaded many times, but we must not
            // attach more than one once.
            if !item.equipment_keys.is_empty() {
                continue;
            }

            drop(inventory);
            let attached = models.attach_scene(world, entity);

            let Ok(mut inventory) = world.get::<&mut Inventory>(entity) else {
                continue;
            };

            let item = inventory.get_mut(key).unwrap();
            item.equipment_keys = attached.keys;

            if let ItemData::Weapon(weapon) = &mut item.data {
                weapon.projectile_node = attached.projectile_node;
            }
        }
    }
}
