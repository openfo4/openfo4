use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use game_tasks::TaskPool;
use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_esp::FormId;
use slab::Slab;

use crate::state::anim_starter::AnimationEventQueue;
use crate::state::equipment::{load_equipment_item, load_equipment_item_models};
use crate::state::esp::EspLoader;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InventoryKey(usize);

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    items: Slab<ItemStack>,
    by_id: HashMap<FormId, Vec<usize>>,
}

impl Inventory {
    pub fn insert(&mut self, item: ItemStack) -> InventoryKey {
        let index = self.items.insert(item);
        InventoryKey(index)
    }

    pub fn get(&self, id: InventoryKey) -> Option<&ItemStack> {
        self.items.get(id.0)
    }

    pub fn get_mut(&mut self, id: InventoryKey) -> Option<&mut ItemStack> {
        self.items.get_mut(id.0)
    }

    pub fn items(&self) -> impl Iterator<Item = &ItemStack> {
        self.items.iter().map(|(_, v)| v)
    }

    pub fn items_mut(&mut self) -> impl Iterator<Item = &mut ItemStack> {
        self.items.iter_mut().map(|(_, v)| v)
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
}

/// Defines properties for the aiming of guns.
#[derive(Clone, Debug)]
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
            count: 1,
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

        let Some(item) = inventory.items_mut().find(|v| v.item == event.item) else {
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
