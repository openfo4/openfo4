pub mod animation_sync;
pub mod camera;
pub mod controller;
pub mod equipment;
pub mod hierarchy;
pub mod lights;
pub mod ni_transform;
pub mod skeleton;

#[derive(Clone, Debug, Default)]
pub struct PlayerControlState {
    pub moving_front: bool,
    pub moving_back: bool,
    pub moving_left: bool,
    pub moving_right: bool,
    pub sneaking: bool,
    pub sprinting: bool,
    pub walking: bool,
}
