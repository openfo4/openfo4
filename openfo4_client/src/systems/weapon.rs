use std::time::{Duration, Instant};

use game_common::components::GlobalTransform;
use game_common::math::Ray;
use glam::Vec3;
use hecs::{CommandBuffer, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::Collider;
use openfo4_physics::{PhysicsPipeline, QueryFilter};

use crate::render::physics_debug::{DespawnAt, HitscanRaycast};
use crate::systems::inventory::{Inventory, ItemData};
use crate::systems::{FacingDirection, GunFireEvent};

#[derive(Clone, Debug)]
pub struct EquippedGun {
    pub ammo: u32,
    pub damage: u32,
}

pub fn handle_weapon_fire_events(world: &mut World, physics: &PhysicsPipeline, events: &EventBus) {
    let mut commands = CommandBuffer::new();

    for event in events.read::<GunFireEvent>() {
        let Ok(entity) = world.entity(event.actor) else {
            continue;
        };

        let transform = entity.get::<&GlobalTransform>().unwrap();
        let facing_dir = entity.get::<&FacingDirection>().unwrap();
        let inventory = entity.get::<&Inventory>().unwrap();

        for item in inventory.items() {
            let ItemData::Weapon(weapon) = &item.data else {
                continue;
            };

            if !weapon.drawn {
                debug_assert!(!item.equipped);
                continue;
            }

            dbg!(&item);
        }

        // We must exclude the actor itself, otherwise we're just gonna
        // hit ourselves immediately.
        let mut exclude_colliders = Vec::new();
        let mut queue = vec![event.actor];
        while let Some(entity) = queue.pop() {
            if world.entity(entity).unwrap().has::<Collider>() {
                exclude_colliders.push(entity);
            }

            if let Some(children) = world.entity(entity).unwrap().get::<&Children>() {
                queue.extend_from_slice(&children.0);
            }
        }

        let ray = Ray {
            origin: transform.0.translation,
            direction: facing_dir.0 * -Vec3::Z,
        };
        let max_toi = 1000.0;
        let hit = physics.cast_ray(
            ray,
            max_toi,
            QueryFilter {
                exclude_colliders: &exclude_colliders,
            },
        );

        commands.spawn((
            HitscanRaycast {
                ray,
                max_toi,
                impact_toi: hit.map(|v| v.toi),
            },
            DespawnAt(Instant::now() + Duration::from_secs(5)),
        ));
    }

    commands.run_on(world);
}
