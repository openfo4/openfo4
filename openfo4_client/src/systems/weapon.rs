use std::sync::Arc;
use std::time::{Duration, Instant};

use game_common::components::GlobalTransform;
use game_common::math::Ray;
use glam::Vec3;
use hecs::{CommandBuffer, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::{Collider, CollisionLayer};
use openfo4_physics::{PhysicsPipeline, QueryFilter};

use crate::render::physics_debug::{DespawnAt, HitscanRaycast};
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::AnimationEventQueue;
use crate::state::hardcoded_formids::ACTOR_VALUE_HEALTH;
use crate::state::npc::{ActorBodyParts, BodyPartBone, BodyParts};
use crate::systems::actor_value::ActorValueProperties;
use crate::systems::inventory::{Inventory, ItemData};
use crate::systems::{
    DeathEvent, FacingDirection, GunFireEvent, GunReloadEvent, GunReloadRequest, ProjectileHitEvent,
};

#[derive(Clone, Debug)]
pub struct EquippedGun {
    pub ammo: u32,
    pub damage: u32,
}

pub fn handle_weapon_reload_requests(world: &mut World, actions: &Actions, events: &EventBus) {
    for event in events.read::<GunReloadRequest>() {
        let Ok(entity) = world.entity(event.actor) else {
            continue;
        };

        let inventory = entity.get::<&Inventory>().unwrap();
        let mut animation_events = entity.get::<&mut AnimationEventQueue>().unwrap();

        let can_reload = inventory.items().any(|item| match &item.data {
            ItemData::Weapon(weapon) => {
                item.equipped && weapon.current_ammo != weapon.ammo_capacity
            }
            _ => false,
        });

        if can_reload {
            if let Some(action) = actions.0.get(&Action::Reload) {
                animation_events.0.push_back(*action);
            }
        }
    }
}

pub fn handle_weapon_fire_events(
    world: &mut World,
    physics: &PhysicsPipeline,
    events: &mut EventBus,
) {
    let mut commands = CommandBuffer::new();

    let mut hit_events = Vec::new();

    for event in events.read::<GunFireEvent>() {
        let Ok(entity) = world.entity(event.actor) else {
            continue;
        };

        let transform = entity.get::<&GlobalTransform>().unwrap();
        let facing_dir = entity.get::<&FacingDirection>().unwrap();
        let mut inventory = entity.get::<&mut Inventory>().unwrap();

        let mut equipped_weapons = Vec::new();

        for item in inventory.items_mut() {
            let ItemData::Weapon(weapon) = &mut item.data else {
                continue;
            };

            if !weapon.drawn {
                debug_assert!(!item.equipped);
                continue;
            }

            // Don't trigger weapons with no ammo.
            if weapon.current_ammo == 0 {
                continue;
            }

            weapon.current_ammo -= 1;

            equipped_weapons.push(weapon);
        }

        if equipped_weapons.is_empty() {
            continue;
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
                layers: &[CollisionLayer::STATIC],
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

        if let Some(hit) = hit {
            if let Ok(body_part) = world.get::<&BodyPartBone>(hit.entity) {
                for weapon in equipped_weapons {
                    hit_events.push(ProjectileHitEvent {
                        target: body_part.actor,
                        body_part: body_part.id,
                        damage: weapon.damage.clone(),
                        shooter: Some(event.actor),
                    });
                }
            }
        }
    }

    for event in hit_events {
        events.send(event);
    }

    commands.run_on(world);
}

pub fn handle_weapon_reload_events(world: &mut World, events: &mut EventBus) {
    for event in events.read::<GunReloadEvent>() {
        let Ok(entity) = world.entity(event.actor) else {
            continue;
        };

        let mut inventory = entity.get::<&mut Inventory>().unwrap();

        for item in inventory.items_mut() {
            let ItemData::Weapon(weapon) = &mut item.data else {
                continue;
            };

            if !weapon.drawn {
                debug_assert!(!item.equipped);
                continue;
            }

            weapon.current_ammo = weapon.ammo_capacity;
        }
    }
}

pub fn handle_weapon_hit_damage(world: &mut World, events: &mut EventBus) {
    let mut death_events = Vec::new();

    for event in events.read::<ProjectileHitEvent>() {
        let Ok(entity) = world.entity(event.target) else {
            continue;
        };

        let mut properties = entity.get::<&mut ActorValueProperties>().unwrap();
        let body_parts = entity.get::<&ActorBodyParts>().unwrap();

        let Some(body_part) = body_parts.get(event.body_part) else {
            continue;
        };

        let mut damage = 0.0;
        for (damage_type, damage_value) in &event.damage {
            damage += *damage_value as f32 * body_part.damage_factor;
            // TODO: Damage resistance
        }

        let mut health = properties.get(ACTOR_VALUE_HEALTH).unwrap();
        health = f32::max(health - damage, 0.0);
        properties.set(ACTOR_VALUE_HEALTH, health);

        death_events.push(DeathEvent {
            actor: event.target,
        });
    }

    for event in death_events {
        events.send(event);
    }
}
