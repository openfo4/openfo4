use std::f32::consts::FRAC_PI_2;
use std::time::{Duration, Instant};

use glam::{Quat, Vec3};
use openfo4_common::components::Transform;
use openfo4_common::math::Ray;
use openfo4_core::ecs::Entity;
use openfo4_core::transform::ConvertCoordinateSystem;

use crate::assets::nif::NIF_TO_METERS;
use crate::systems::ai::movement::{Face, MoveTo};
use crate::systems::ai::{Behavior, Context, Status};
use crate::systems::inventory::{EquipItemEvent, Inventory, InventoryKey, ItemData};
use crate::systems::{
    FacingDirection, GunFireRequest, GunReloadEvent, GunReloadRequest, WeaponDrawRequest,
    WeaponDrawnEvent,
};

/// Below this distance NPCs with guns seem to consistently backoff until they reach this distance.
const SHORT_DISTANCE: f32 = 350.0 * NIF_TO_METERS;

const MEDIUM_DISTANCE: f32 = 1024.0 * NIF_TO_METERS;
const LONG_DISTANCE: f32 = 2048.0 * NIF_TO_METERS;

/// Minimum distance we must travel if we decide to move.
const MIN_STEP_DIST: f32 = 20.0 * NIF_TO_METERS;

#[derive(Copy, Clone, Debug, Default)]
pub struct DefaultCombatPackageConfig {
    /// The larger this factor, the larger the chance that the actor prefers offensive actions.
    ///
    /// More offensive actors will:
    /// - Attack more often
    /// - Use power attacks more often
    /// - More more aggressively
    pub offensive_factor: f32,
    /// More defensive actors will:
    /// - Block more often
    /// - Gun bash more often
    pub defensive_factor: f32,
    /// Chance that the actor will hit the target.
    ///
    /// This factor is combined with weapon the accuracy.
    pub gun_accuracy_factor: f32,
    /// Chance that the actor will attack staggered targets.
    pub melee_attack_staggered_target: f32,
    /// Chance that the actor will power attack staggered targets.
    pub melee_power_attack_staggered_target: f32,
    /// Chance that the actor will power attack a blocking target.
    pub melee_power_attack_blocking_target: f32,
    /// Chance that the actor will gun bash.
    pub bash: f32,
    /// Chance that the actor will bash staggered targets.
    pub bash_staggered_target: f32,
    /// Chance that the actor will bash a target currently attacking.
    pub bash_attacking_target: f32,
    /// Chance that the actor will bash a target currently power attacking.
    pub bash_power_attacking_target: f32,
    /// Chance that the actor will circle around the target in either direction.
    pub dueling_circle: f32,
    /// Chance that the actor will fall back.
    pub dueling_fallback: f32,
}

#[derive(Clone, Debug)]
pub struct DefaultCombatPackage {
    config: DefaultCombatPackageConfig,
}

impl DefaultCombatPackage {
    fn update(&mut self) {}
}

#[derive(Clone, Debug)]
pub struct CombatBehavior {
    config: DefaultCombatPackageConfig,
    /// Enemy combat targets.
    targets: Vec<Entity>,
    wait_for_equipped_weapon: Option<InventoryKey>,
    wait_for_weapon_draw_event: bool,
    wait_for_reload_event: bool,
    last_attack: Instant,
    movement: MovementStrategy,
}

impl CombatBehavior {
    pub fn new(config: DefaultCombatPackageConfig, targets: Vec<Entity>) -> Self {
        Self {
            config,
            targets,
            wait_for_equipped_weapon: None,
            wait_for_weapon_draw_event: false,
            wait_for_reload_event: false,
            last_attack: Instant::now(),
            movement: MovementStrategy {
                movement: None,
                target: Vec3::ZERO,
                last_action: Instant::now(),
            },
        }
    }
}

impl Behavior for CombatBehavior {
    fn start(&mut self, ctx: &mut Context<'_>) {}

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        let Some(target) = self.select_target() else {
            return Status::Success;
        };

        let pos = *ctx.world.get::<&Transform>(ctx.entity).unwrap();
        let target_pos = *ctx.world.get::<&Transform>(target).unwrap();

        let distance = target_distance(ctx, target);

        self.apply_movement(ctx, target);

        self.equip_best_weapon(ctx);

        if distance < SHORT_DISTANCE {
            self.look_at(ctx, target_pos.translation);

            if !self.draw_weapon(ctx) {
                return Status::Running;
            }

            if !self.reload_weapon(ctx) {
                return Status::Running;
            }

            self.shoot(ctx);
        }

        // let fallback_distance = f32::min(SHORT_DISTANCE - distance, 300.0 * NIF_TO_METERS);
        // if fallback_distance > 0.0 {
        //     let fallback =
        //         (pos.translation - target_pos.translation).normalize() * fallback_distance;
        //     let mut fallback_point = pos.translation + fallback;

        //     let navmesh = ctx.navmeshes.get(&ctx.worldspace).unwrap();

        //     if let Some(toi) = navmesh.cast_ray(Ray::new(fallback_point, Vec3::Y), f32::MAX) {
        //         fallback_point = fallback_point + toi * Vec3::Y;
        //     } else if let Some(toi) = navmesh.cast_ray(Ray::new(fallback_point, -Vec3::Y), f32::MAX)
        //     {
        //         fallback_point = fallback_point - toi * Vec3::Y;
        //     }

        //     let mut movement = MoveTo::new(fallback_point, Face::Forward);
        //     movement.start(ctx);
        //     self.movement = Some(movement);
        // }

        // match distance {
        //     TargetDistance::Short => {
        //         let fallback = 0.0;

        //         let nm = ctx.navmeshes.get(&ctx.worldspace).unwrap();
        //         nm.cast_ray(ray, max_toi);
        //     }
        //     TargetDistance::Medium => {}
        //     TargetDistance::Long => {}
        // }

        Status::Running
    }

    fn finish(&mut self, ctx: &mut Context<'_>) {}
}

impl CombatBehavior {
    fn select_target(&self) -> Option<Entity> {
        self.targets.first().copied()
    }

    fn equip_best_weapon(&mut self, ctx: &mut Context<'_>) {
        let Ok(inventory) = ctx.world.get::<&Inventory>(ctx.entity) else {
            return;
        };

        if let Some(id) = self.wait_for_equipped_weapon {
            if let Some(item) = inventory.get(id)
                && item.equipped
            {
                return;
            }

            self.wait_for_equipped_weapon = None;
        }

        let mut best_weapon = None;
        let mut best_weapon_score = 0;

        for (id, item) in inventory.items() {
            let ItemData::Weapon(weapon) = &item.data else {
                continue;
            };

            let damage = weapon.damage.values().sum();
            if damage > best_weapon_score {
                best_weapon = Some(id);
                best_weapon_score = damage;
            }
        }

        if let Some(id) = best_weapon {
            let item = inventory.get(id).unwrap();

            if !item.equipped {
                let item_id = item.item;
                drop(inventory);

                ctx.send_event(EquipItemEvent {
                    actor: ctx.entity,
                    item: item_id,
                });
                self.wait_for_equipped_weapon = Some(id);
            }
        }
    }

    /// Draws the weapon and returns whether the weapon was drawn successfully.
    fn draw_weapon(&mut self, ctx: &mut Context<'_>) -> bool {
        let Ok(inventory) = ctx.world.get::<&Inventory>(ctx.entity) else {
            return false;
        };

        if self.wait_for_weapon_draw_event {
            for event in ctx.read_events::<WeaponDrawnEvent>() {
                if event.actor == ctx.entity {
                    self.wait_for_weapon_draw_event = false;
                }
            }
        }

        let mut has_equipped_weapon = false;
        for (_, item) in inventory.items() {
            if !item.equipped {
                continue;
            }

            let ItemData::Weapon(weapon) = &item.data else {
                continue;
            };

            has_equipped_weapon = true;
            if weapon.drawn {
                return true;
            }
        }

        drop(inventory);

        if has_equipped_weapon {
            ctx.send_event(WeaponDrawRequest { actor: ctx.entity });
        }

        false
    }

    fn reload_weapon(&mut self, ctx: &mut Context<'_>) -> bool {
        let Ok(inventory) = ctx.world.get::<&Inventory>(ctx.entity) else {
            return false;
        };

        if self.wait_for_reload_event {
            for event in ctx.read_events::<GunReloadEvent>() {
                if event.actor == ctx.entity {
                    self.wait_for_reload_event = false;
                }
            }

            if self.wait_for_reload_event {
                return false;
            }
        }

        for (_, item) in inventory.items() {
            if !item.equipped {
                continue;
            }

            let ItemData::Weapon(weapon) = &item.data else {
                continue;
            };

            if !weapon.drawn {
                continue;
            }

            if weapon.current_ammo > 0 {
                return true;
            }
        }

        drop(inventory);

        ctx.send_event(GunReloadRequest { actor: ctx.entity });
        self.wait_for_reload_event = true;

        false
    }

    fn look_at(&mut self, ctx: &mut Context<'_>, mut target: Vec3) {
        let (Ok(mut transform), Ok(mut facing_dir)) = (
            ctx.world.get::<&mut Transform>(ctx.entity),
            ctx.world.get::<&mut FacingDirection>(ctx.entity),
        ) else {
            return;
        };

        facing_dir.0 = transform.looking_at(target, Vec3::Y).rotation * Quat::IDENTITY.to_opengl();

        target.y = transform.translation.y;
        transform.rotation =
            transform.looking_at(target, Vec3::Y).rotation * Quat::IDENTITY.to_opengl();
    }

    fn shoot(&mut self, ctx: &mut Context<'_>) {
        if self.last_attack.elapsed() > Duration::from_secs(5) {
            self.last_attack = Instant::now();
            ctx.send_event(GunFireRequest { actor: ctx.entity });
        }
    }

    fn apply_movement(&mut self, ctx: &mut Context<'_>, target: Entity) {
        if let Some(movement) = &mut self.movement.movement {
            // Periodically chose a new action, even if we are still moving.
            if self.movement.last_action.elapsed() < Duration::from_secs(10)
                && matches!(movement.update(ctx), Status::Running)
            {
                return;
            }

            movement.finish(ctx);
            self.movement.movement = None;
        }

        let pos = *ctx.world.get::<&Transform>(ctx.entity).unwrap();
        let target_pos = *ctx.world.get::<&Transform>(target).unwrap();
        let distance = target_distance(ctx, target);

        // Backoff from the target.
        if distance < SHORT_DISTANCE {
            let fallback_distance = f32::min(SHORT_DISTANCE - distance, 300.0 * NIF_TO_METERS);
            if fallback_distance > MIN_STEP_DIST {
                let fallback =
                    (pos.translation - target_pos.translation).normalize() * fallback_distance;
                self.move_to(ctx, pos.translation + fallback);
                return;
            }
        }

        // return;

        // Circle around the target.
        if distance < MEDIUM_DISTANCE {
            if rand::random_bool(0.3) {
                let mut angle = rand::random_range(0.0..FRAC_PI_2);
                if rand::random_bool(0.5) {
                    angle = -angle;
                }

                let p = Quat::from_rotation_y(angle) * Vec3::new(0.0, 0.0, -distance);
                let dst = target_pos.translation + p;

                if dst.distance_squared(pos.translation) < MIN_STEP_DIST * MIN_STEP_DIST {
                    return;
                }

                self.move_to(ctx, dst);
                return;
            }
        }
    }

    fn move_to(&mut self, ctx: &mut Context<'_>, mut target: Vec3) {
        let navmesh = ctx.navmeshes.get(&ctx.worldspace).unwrap();

        if let Some(toi) = navmesh.cast_ray(Ray::new(target, Vec3::Y), f32::MAX) {
            target = target + toi * Vec3::Y;
        } else if let Some(toi) = navmesh.cast_ray(Ray::new(target, -Vec3::Y), f32::MAX) {
            target = target - toi * Vec3::Y;
        }

        let mut moveto = MoveTo::new(target, Face::Forward);
        moveto.start(ctx);
        self.movement.movement = Some(moveto);
        self.movement.last_action = Instant::now();
    }
}

fn target_distance(ctx: &mut Context<'_>, target: Entity) -> f32 {
    if let (Ok(me), Ok(target)) = (
        ctx.world.get::<&Transform>(ctx.entity),
        ctx.world.get::<&Transform>(target),
    ) {
        me.translation.distance(target.translation)
    } else {
        f32::MAX
    }
}

fn has_weapon_los(ctx: &mut Context<'_>, target: Entity) -> bool {
    todo!()
}

#[derive(Copy, Clone, Debug)]
enum TargetDistance {
    Short,
    Medium,
    Long,
}

#[derive(Clone, Debug)]
struct MovementStrategy {
    movement: Option<MoveTo>,
    target: Vec3,
    last_action: Instant,
}

impl From<f32> for TargetDistance {
    fn from(value: f32) -> Self {
        if value <= SHORT_DISTANCE {
            Self::Short
        } else if value <= MEDIUM_DISTANCE {
            Self::Medium
        } else {
            Self::Long
        }
    }
}

#[derive(Clone, Debug)]
struct DrawWeapon;

impl Behavior for DrawWeapon {
    fn start(&mut self, ctx: &mut Context<'_>) {
        ctx.send_event(WeaponDrawRequest { actor: ctx.entity });
    }

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        for event in ctx.read_events::<WeaponDrawnEvent>() {
            if event.actor == ctx.entity {
                return Status::Success;
            }
        }

        Status::Running
    }

    fn finish(&mut self, _ctx: &mut Context<'_>) {}
}

#[derive(Clone, Debug)]
struct Reload;

impl Behavior for Reload {
    fn start(&mut self, ctx: &mut Context<'_>) {
        ctx.send_event(GunReloadRequest { actor: ctx.entity });
    }

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        for event in ctx.read_events::<GunReloadEvent>() {
            if event.actor == ctx.entity {
                return Status::Success;
            }
        }

        Status::Running
    }

    fn finish(&mut self, ctx: &mut Context<'_>) {}
}

#[derive(Clone, Debug)]
struct Attack;

impl Behavior for Attack {
    fn start(&mut self, ctx: &mut Context<'_>) {
        ctx.send_event(GunFireRequest { actor: ctx.entity });
    }

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        Status::Success
    }

    fn finish(&mut self, ctx: &mut Context<'_>) {}
}
