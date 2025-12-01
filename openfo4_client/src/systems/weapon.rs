use std::f32::consts::PI;
use std::time::{Duration, Instant};

use game_common::components::GlobalTransform;
use game_common::math::Ray;
use glam::{Quat, Vec2, Vec3};
use hecs::{CommandBuffer, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::{Collider, CollisionLayer};
use openfo4_physics::{PhysicsPipeline, QueryFilter};

use crate::render::physics_debug::{DespawnAt, HitscanRaycast};
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::AnimationEventQueue;
use crate::state::hardcoded_formids::ACTOR_VALUE_HEALTH;
use crate::state::npc::{ActorBodyParts, BodyPartBone};
use crate::systems::actor_value::ActorValueProperties;
use crate::systems::inventory::{Inventory, ItemData};
use crate::systems::{
    DeathEvent, FacingDirection, GunFireEvent, GunReloadEvent, GunReloadRequest,
    ProjectileHitEvent, WeaponDrawRequest, WeaponDrawnEvent, WeaponSheathEvent,
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

        let mut ammo = None;
        let can_reload = inventory.items().any(|(_, item)| match &item.data {
            ItemData::Weapon(weapon) => {
                ammo = weapon.ammo;
                item.equipped && weapon.current_ammo != weapon.ammo_capacity
            }
            _ => false,
        });

        let Some(ammo) = ammo else {
            continue;
        };

        let Some((_, ammo)) = inventory.get_by_form(ammo).next() else {
            continue;
        };

        if can_reload && ammo.count > 0 {
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

        for (_, item) in inventory.items_mut() {
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

            // FIXME: Implement increase of cof when shot and decrease on timer.
            // FIXME: We need a radius, do we divde by 2 here?
            let cof = weapon.aim_props.cone_of_fire_angle_min / 2.0;

            // Sample a random point on the circle that represents
            // the cone of fire.
            // See https://stackoverflow.com/a/50746409
            let r = cof * rand::random::<f32>().sqrt();
            let theta = rand::random::<f32>() * 2.0 * PI;
            let offset = Vec2::new(r * theta.cos(), r * theta.sin());

            let direction = facing_dir.0
                * Quat::from_axis_angle(Vec3::X, offset.x)
                * Quat::from_axis_angle(Vec3::Y, offset.y)
                * -Vec3::Z;

            let ray = Ray {
                origin: transform.0.translation,
                direction,
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

            commands.spawn((
                HitscanRaycast {
                    ray: Ray {
                        origin: transform.0.translation,
                        direction: facing_dir.0 * -Vec3::Z,
                    },
                    max_toi,
                    impact_toi: None,
                },
                DespawnAt(Instant::now() + Duration::from_secs(5)),
            ));

            if let Some(hit) = hit {
                if let Ok(body_part) = world.get::<&BodyPartBone>(hit.entity) {
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

        let mut weapons_reloaded = Vec::new();

        for (weapon_id, item) in inventory.items() {
            let ItemData::Weapon(weapon) = &item.data else {
                continue;
            };

            if !weapon.drawn {
                debug_assert!(!item.equipped);
                continue;
            }

            let Some(ammo) = weapon.ammo else {
                continue;
            };

            let Some((ammo_id, ammo)) = inventory.items().find(|(_, v)| v.item == ammo) else {
                continue;
            };

            let amount = u32::min(
                ammo.count as u32,
                weapon.ammo_capacity.saturating_sub(weapon.current_ammo),
            );
            weapons_reloaded.push((weapon_id, ammo_id, amount));
        }

        for (weapon_id, ammo_id, amount) in weapons_reloaded {
            let weapon = inventory.get_mut(weapon_id).unwrap();
            let ItemData::Weapon(weapon) = &mut weapon.data else {
                unreachable!();
            };

            weapon.current_ammo += amount;
            debug_assert!(weapon.current_ammo <= weapon.ammo_capacity);

            let ammo = inventory.get_mut(ammo_id).unwrap();
            ammo.count -= u64::from(amount);
            if ammo.count == 0 {
                inventory.remove(ammo_id);
            }
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

pub fn handle_weapon_draw_events(world: &mut World, events: &mut EventBus) {
    let mut drawn_events = Vec::new();
    let mut sheath_events = Vec::new();

    for event in events.read::<WeaponDrawRequest>() {
        let Ok(entity) = world.entity(event.actor) else {
            continue;
        };

        let mut inventory = entity.get::<&mut Inventory>().unwrap();

        for (_, item) in inventory.items_mut() {
            match &mut item.data {
                ItemData::Weapon(weapon) => {
                    if weapon.drawn {
                        sheath_events.push(WeaponSheathEvent {
                            actor: event.actor,
                            item: item.item,
                        });
                    } else {
                        drawn_events.push(WeaponDrawnEvent {
                            actor: event.actor,
                            item: item.item,
                        });
                    }

                    weapon.drawn ^= true;
                }
                _ => (),
            }
        }
    }

    for event in drawn_events {
        events.send(event);
    }

    for event in sheath_events {
        events.send(event);
    }
}
