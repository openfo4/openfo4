use std::num::NonZeroU64;

use hecs::Entity;

use crate::systems::inventory::InventoryKey;

/// Event sent after an item was added to an inventory.
#[derive(Copy, Clone, Debug)]
pub struct ItemAddedEvent {
    /// The container where the item was added.
    pub entity: Entity,
    /// The item that was added.
    pub slot: InventoryKey,
    /// The number of items that were added in the slot.
    pub count: NonZeroU64,
    /// The container the item originates from, if any.
    pub source: Option<Entity>,
}

/// Event sent after an item was equipped.
#[derive(Copy, Clone, Debug)]
pub struct ItemEquippedEvent {
    /// The actor that equipped the item.
    pub actor: Entity,
    /// The item that was equipped.
    pub slot: InventoryKey,
}

/// Event sent after an item was unequipped.
#[derive(Copy, Clone, Debug)]
pub struct ItemUnequippedEvent {
    /// The actor that unequipped the item.
    pub actor: Entity,
    /// The item that was unequipped.
    pub slot: InventoryKey,
}
