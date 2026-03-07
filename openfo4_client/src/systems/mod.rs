use glam::Quat;
use hashbrown::HashMap;
use hecs::Entity;
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;

use crate::systems::inventory::InventoryKey;

pub mod activation;
pub mod actor_value;
pub mod ai;
pub mod aim;
pub mod animation_sync;
pub mod camera;
pub mod controller;
pub mod door;
pub mod equipment;
pub mod global;
pub mod hierarchy;
pub mod inventory;
pub mod lights;
pub mod linked_refs;
pub mod ni_transform;
pub mod pathing;
pub mod projectile;
pub mod quests;
pub mod scripting;
pub mod skeleton;
pub mod time;
pub mod trigger_box;
pub mod weapon;
pub mod weather;

#[derive(Clone, Debug, Default)]
pub struct PlayerControlState {
    pub moving_front: bool,
    pub moving_back: bool,
    pub moving_left: bool,
    pub moving_right: bool,
    pub sneaking: bool,
    pub sprinting: bool,
    pub walking: bool,
    pub jumping: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FacingDirection(pub Quat);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GunFireEvent {
    pub actor: Entity,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GunReloadEvent {
    pub actor: Entity,
}

#[derive(Clone, Debug)]
pub struct ProjectileHitEvent {
    /// The actor that was hit.
    pub target: Entity,
    /// The ID of the body part that was hit.
    pub body_part: u32,
    /// The actor that has shot the projectile.
    pub shooter: Option<Entity>,
    /// The damage applied by the projectile.
    pub damage: HashMap<RecordId, u32>,
}

#[derive(Copy, Clone, Debug)]
pub struct GunReloadRequest {
    pub actor: Entity,
}

#[derive(Clone, Debug)]
pub struct DeathEvent {
    pub actor: Entity,
}

/// Event sent when an actor wants to draw their equipped weapon.
#[derive(Copy, Clone, Debug)]
pub struct WeaponDrawRequest {
    pub actor: Entity,
}

#[derive(Clone, Debug)]
pub struct WeaponDrawnEvent {
    pub actor: Entity,
    // TODO: Replace with InventorySlotId or similar.
    pub item: RecordId,
}

#[derive(Clone, Debug)]
pub struct WeaponSheathEvent {
    pub actor: Entity,
    // TODO: Replace with InventorySlotId or similar.
    pub item: RecordId,
}

#[derive(Copy, Clone, Debug)]
pub struct ActivationEvent {
    pub actor: Entity,
    pub target: Entity,
}

#[derive(Clone, Debug)]
pub struct TransferItemEvent {
    pub source: Entity,
    pub destination: Entity,
    pub item: InventoryKey,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct PickupItemEvent {
    pub actor: Entity,
    /// The item in the world that is picked up.
    pub item: Entity,
}
