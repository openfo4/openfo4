use std::sync::Arc;

use game_tasks::TaskPool;
use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_esp::{FormId, RecordData, RecordKind};
use slab::Slab;

use crate::assets::nif_v2::NifScene;
use crate::state::equipment::load_equipment_item;
use crate::state::esp::{self, EspLoader};
use crate::systems::equipment::{Equipment, equip_item};

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
    pub count: u64,
    pub attachments: Vec<FormId>,
    pub equipped: bool,
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

        inventory.insert(ItemStack {
            item: event.item,
            count: 1,
            attachments: event.attachments.clone(),
            equipped: false,
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
        let item = item.clone();
        drop(inventory);

        let equipment = futures_lite::future::block_on(load_equipment_item(
            loader,
            esp_loader,
            item.item,
            &item.attachments,
            false,
        ));

        equipment.attach_scene(world, event.actor);
    }
}
