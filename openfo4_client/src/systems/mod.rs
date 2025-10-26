use glam::Quat;
use hecs::Entity;

pub mod actor_value;
pub mod aim;
pub mod animation_sync;
pub mod camera;
pub mod controller;
pub mod equipment;
pub mod hierarchy;
pub mod inventory;
pub mod lights;
pub mod ni_transform;
pub mod projectile;
pub mod skeleton;
pub mod weapon;

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
