use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use game_tasks::TaskPool;
use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_esp::FormId;
use slab::Slab;

use crate::state::equipment::{load_equipment_item, load_equipment_item_models};
use crate::state::esp::EspLoader;

/// Unique identifier for an item inside a container.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InventoryKey(usize);

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    items: Slab<ItemStack>,
    by_id: HashMap<FormId, Vec<usize>>,
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

    pub fn get_by_form(&self, form: FormId) -> impl Iterator<Item = (InventoryKey, &ItemStack)> {
        self.by_id
            .get(&form)
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
    pub item: FormId,
    pub name: String,
    /// Whether the item should not be seen by the player in UIs.
    pub hidden: bool,
    pub kind: ItemKind,
    pub count: u64,
    pub attachments: Vec<FormId>,
    pub equipped: bool,
    pub base_keywords: HashSet<FormId>,
    pub combined_keywords: HashSet<FormId>,
    pub data: ItemData,
}

impl ItemStack {
    /// Returns `true` if `other` can be merged into `self`.
    fn mergeable(&self, other: &Self) -> bool {
        self.item == other.item
            && self.hidden == other.hidden
            && self.attachments == other.attachments
            && self.base_keywords == other.base_keywords
            && self.combined_keywords == other.combined_keywords
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
    pub damage: HashMap<FormId, u32>,
    pub aim_props: AimProps,
    /// Ammo consumed by this weapon.
    pub ammo: Option<FormId>,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Weapon,
    Armor,
    Ammo,
    Misc,
}

impl ItemStack {
    fn joinable(&self, other: &ItemStack) -> bool {
        debug_assert_eq!(self.item, other.item);
        self.attachments == other.attachments
    }
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub form: FormId,
}

#[derive(Clone, Debug)]
pub struct AddItemEvent {
    pub actor: Entity,
    pub item: FormId,
    pub attachments: Vec<FormId>,
    pub hidden: bool,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct EquipItemEvent {
    pub actor: Entity,
    pub item: FormId,
}

pub fn handle_inventory_events(
    world: &mut World,
    loader: &AssetLoader,
    esp_loader: &Arc<EspLoader>,
    events: &mut EventBus,
    tasks: &TaskPool,
) {
    for event in events.read::<AddItemEvent>() {
        let mut inventory = world.get::<&mut Inventory>(event.actor).unwrap();

        let item = futures_lite::future::block_on(load_equipment_item(
            esp_loader,
            event.item,
            &event.attachments,
        ));

        inventory.insert(ItemStack {
            item: event.item,
            kind: item.kind,
            count: event.count,
            attachments: event.attachments.clone(),
            equipped: false,
            base_keywords: item.base_keywords,
            combined_keywords: item.combined_keywords,
            data: item.data,
            name: item.name,
            hidden: event.hidden,
        });
    }

    for event in events.read::<EquipItemEvent>() {
        let mut inventory = world.get::<&mut Inventory>(event.actor).unwrap();

        let Some((_, item)) = inventory.items_mut().find(|(_, v)| v.item == event.item) else {
            continue;
        };

        if item.equipped {
            continue;
        }

        item.equipped = true;

        match &mut item.data {
            ItemData::Weapon(weapon) => {
                // Start without weapon drawn.
                weapon.drawn = false;
            }
            _ => (),
        }

        let equipment = futures_lite::future::block_on(load_equipment_item_models(
            loader,
            esp_loader,
            item.item,
            &item.attachments,
            false,
        ));

        drop(inventory);
        equipment.attach_scene(world, event.actor);
    }
}
