use hecs::World;
use openfo4_animation::AnimationEvents;

#[derive(Clone, Debug)]
pub struct EquippedGun {
    pub ammo: u32,
    pub damage: u32,
}

pub fn handle_weapon_fire_events(world: &mut World) {
    for (entity, (gun, events)) in world.query_mut::<(&mut EquippedGun, &mut AnimationEvents)>() {}
}
