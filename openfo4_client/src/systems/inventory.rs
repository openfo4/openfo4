use std::fmt::Debug;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use game_common::components::Transform;
use game_tasks::{TaskPool, TaskQueue};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use slab::Slab;

use crate::components::PlacedItem;
use crate::events::inventory::{ItemAddedEvent, ItemEquippedEvent, ItemUnequippedEvent};
use crate::state::equipment::{EquipmentItemModels, load_equipment_item_models, load_item};
use crate::state::esp::EspLoader;
use crate::state::hardcoded::KEYWORD_FEMALE;
use crate::state::npc::ObjectKeywords;
use crate::systems::equipment::{EquipmentKey, unequip_item};
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
    pub visibility_slots: HashSet<u32>,
    pub visible: bool,
    pub script: Option<ScriptObject>,
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
}

impl ItemStack {
    fn joinable(&self, other: &ItemStack) -> bool {
        debug_assert_eq!(self.item, other.item);
        self.attachments == other.attachments
    }
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub form: RecordId,
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
    tasks: TaskQueue<(Entity, InventoryKey, EquipmentItemModels)>,
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
                let items = load_item(
                    &esp_loader,
                    &script_loader,
                    event.item,
                    &event.attachments,
                    event.count,
                )
                .await;

                let equip_events = items
                    .iter()
                    .map(|item| EquipItemEvent {
                        actor: event.actor,
                        item: item.item,
                    })
                    .collect();
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
        events.extend(unequip_transfer_events);

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

        let mut unequip_conflicting_events = Vec::new();

        let mut items_equipped = Vec::new();
        for event in events.read::<EquipItemEvent>() {
            let female = world
                .get::<&ObjectKeywords>(event.actor)
                .is_ok_and(|kw| kw.contains(KEYWORD_FEMALE));

            let Ok(mut inventory) = world.get::<&mut Inventory>(event.actor) else {
                continue;
            };

            let Some((key, item)) = inventory.items_mut().find(|(_, v)| v.item == event.item)
            else {
                continue;
            };

            // Only weapons and armor can be equipped.
            if item.kind != ItemKind::Weapon && item.kind != ItemKind::Armor {
                continue;
            }

            if item.equipped {
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

            let loader = self.loader.clone();
            let esp_loader = self.esp_loader.clone();
            let form = item.item;
            let attachments = item.attachments.clone();
            let actor = event.actor;
            self.tasks.push(self.pool.spawn(async move {
                let model =
                    load_equipment_item_models(&loader, &esp_loader, form, &attachments, female)
                        .await;

                (actor, key, model)
            }));

            let Some((key, item)) = inventory.items().find(|(_, v)| v.item == event.item) else {
                continue;
            };

            let mut hide_slots = Vec::new();
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

                if other_item.equipped
                    && other_item.visible
                    && other_item
                        .visibility_slots
                        .intersection(&item.visibility_slots)
                        .count()
                        != 0
                {
                    hide_slots.push(other_key);
                }
            }

            let mut unequip_keys = Vec::new();
            for key in hide_slots {
                let slot = inventory.get_mut(key).unwrap();

                slot.visible = false;
                let keys = core::mem::take(&mut slot.equipment_keys);
                unequip_keys.extend(keys);
            }

            drop(inventory);
            unequip_item(event.actor, unequip_keys, world);
        }

        events.extend(items_equipped);

        let mut rescan_visiblity = HashSet::new();

        let mut items_unequipped = Vec::new();
        for event in events.read::<UnequipItemEvent>() {
            let Ok(mut inventory) = world.get::<&mut Inventory>(event.actor) else {
                continue;
            };

            let Some(item) = inventory.get_mut(event.item) else {
                continue;
            };

            // Only weapons and armor can be equipped.
            if item.kind != ItemKind::Weapon && item.kind != ItemKind::Armor {
                continue;
            }

            if !item.equipped {
                continue;
            }

            item.equipped = false;
            let keys = core::mem::take(&mut item.equipment_keys);
            items_unequipped.push(ItemUnequippedEvent {
                actor: event.actor,
                slot: event.item,
            });

            // Reset visibility state.
            item.visible = true;

            if !item.visibility_slots.is_empty() {
                rescan_visiblity.insert(event.actor);
            }

            drop(inventory);
            unequip_item(event.actor, keys, world);
        }

        events.extend(items_unequipped);

        for actor in rescan_visiblity {
            let female = world
                .get::<&ObjectKeywords>(actor)
                .is_ok_and(|kw| kw.contains(KEYWORD_FEMALE));

            let Ok(mut inventory) = world.get::<&mut Inventory>(actor) else {
                continue;
            };

            let mut newly_visible = HashSet::new();

            for (key, item) in inventory.items() {
                if !item.equipped || item.visible {
                    continue;
                }

                let mut can_be_visible = true;

                for (other_key, other) in inventory.items() {
                    if key == other_key || !other.equipped {
                        continue;
                    }

                    if item
                        .visibility_slots
                        .intersection(&other.visibility_slots)
                        .count()
                        == 0
                    {
                        continue;
                    }

                    // If another item is already visible keep it as is.
                    if other.visible || newly_visible.contains(&other_key) {
                        can_be_visible = false;
                        break;
                    }
                }

                if can_be_visible {
                    newly_visible.insert(key);
                }
            }

            for key in newly_visible {
                let item = inventory.get_mut(key).unwrap();
                item.visible = true;

                let loader = self.loader.clone();
                let esp_loader = self.esp_loader.clone();
                let form = item.item;
                let attachments = item.attachments.clone();
                self.tasks.push(self.pool.spawn(async move {
                    let model = load_equipment_item_models(
                        &loader,
                        &esp_loader,
                        form,
                        &attachments,
                        female,
                    )
                    .await;

                    (actor, key, model)
                }));
            }
        }

        // Transfering items must come after all unequip events have been processed.
        // This ensures that any items to transfer are not unequipped.
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

    fn attach_ready_models(&mut self, world: &mut World) {
        let _span = trace_span!("InventorySystem::attach_ready_models").entered();

        while let Some((entity, key, models)) = self.tasks.pop() {
            // Recheck that we still should attach the scene.
            // The item may already be unequipped again.
            let Ok(inventory) = world.get::<&Inventory>(entity) else {
                continue;
            };

            let Some(item) = inventory.get(key) else {
                continue;
            };

            if !item.equipped || !item.visible {
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
