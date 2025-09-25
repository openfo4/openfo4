use hecs::World;

#[derive(Clone, Debug)]
pub struct EquippedGun {
    pub ammo: u32,
    pub damage: u32,
}
