use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use std::time::{Duration, Instant};

use glam::{Quat, Vec3};
use openfo4_common::components::Transform;
use openfo4_common::math::Ray;
use openfo4_core::ecs::Entity;
use openfo4_core::transform::ConvertCoordinateSystem;

use crate::assets::nif::NIF_TO_METERS;
use crate::systems::ai::movement::MoveTo;
use crate::systems::ai::{Behavior, Context, Status};
use crate::systems::inventory::{EquipItemEvent, Inventory, InventoryKey, ItemData};
use crate::systems::{
    FacingDirection, GunReloadEvent, GunReloadRequest, TargetedGunFireEvent, WeaponDrawRequest,
    WeaponDrawnEvent,
};

/// Below this distance NPCs with guns seem to consistently backoff until they reach this distance.
const SHORT_DISTANCE: f32 = 350.0 * NIF_TO_METERS;

const MEDIUM_DISTANCE: f32 = 1024.0 * NIF_TO_METERS;
const LONG_DISTANCE: f32 = 2048.0 * NIF_TO_METERS;

/// Minimum distance we must travel if we decide to move.
const MIN_STEP_DIST: f32 = 20.0 * NIF_TO_METERS;

const MIN_FALLBACK_DISTANCE: f32 = SHORT_DISTANCE * 0.3;
const MAX_FALLBACK_DISTANCE: f32 = SHORT_DISTANCE * 1.5;
const MAX_FALLBACK_ANGLE: f32 = FRAC_PI_4;

const MIN_STEP_DISTANCE: f32 = 50.0;
const MAX_STEP_DISTANCE: f32 = 200.0;

const MIN_MOVE_INTERRUPT_TIME: Duration = Duration::from_secs(3);
const MAX_MOVE_INTERRUPT_TIME: Duration = Duration::from_secs(10);

const MIN_MOVE_IDLE_TIME: Duration = Duration::from_secs(0);
const MAX_MOVE_IDLE_TIME: Duration = Duration::from_secs(1);

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
                last_action: Instant::now(),
                idle_until: Instant::now(),
                interrupt_at: Instant::now(),
            },
        }
    }
}

impl Behavior for CombatBehavior {
    fn start(&mut self, _ctx: &mut Context<'_>) {}

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        let Some(target) = self.select_target() else {
            return Status::Success;
        };

        let target_pos = *ctx.world.get::<&Transform>(target).unwrap();

        self.apply_movement(ctx, target);

        self.equip_best_weapon(ctx);

        self.look_at(ctx, target_pos.translation);

        if !self.draw_weapon(ctx) {
            return Status::Running;
        }

        if !self.reload_weapon(ctx) {
            return Status::Running;
        }

        self.shoot(ctx, target);

        Status::Running
    }

    fn finish(&mut self, _ctx: &mut Context<'_>) {}
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

        facing_dir.0 = transform.looking_at(target, Vec3::Y).rotation;

        target.y = transform.translation.y;
        transform.rotation =
            transform.looking_at(target, Vec3::Y).rotation * Quat::IDENTITY.to_opengl();
    }

    fn shoot(&mut self, ctx: &mut Context<'_>, target: Entity) {
        let transform = *ctx.world.get::<&Transform>(target).unwrap();

        if self.last_attack.elapsed() > Duration::from_secs(5) {
            self.last_attack = Instant::now();
            ctx.send_event(TargetedGunFireEvent {
                actor: ctx.entity,
                target: transform.translation + Vec3::Y,
            });
        }
    }

    fn apply_movement(&mut self, ctx: &mut Context<'_>, target: Entity) {
        if let Some(movement) = &mut self.movement.movement {
            if movement.update(ctx) != Status::Running {
                movement.finish(ctx);
                self.movement.movement = None;
            }
        }

        if self.movement.movement.is_some() && self.movement.interrupt_at > Instant::now() {
            return;
        }

        if self.movement.idle_until > Instant::now() {
            return;
        }

        // Cancel current movement.
        if let Some(mut movement) = self.movement.movement.take() {
            movement.finish(ctx);
        }

        self.movement.idle_until =
            Instant::now() + rand::random_range(MIN_MOVE_IDLE_TIME..MAX_MOVE_IDLE_TIME);
        self.movement.last_action = Instant::now();
        let pos = *ctx.world.get::<&Transform>(ctx.entity).unwrap();
        let target_pos = *ctx.world.get::<&Transform>(target).unwrap();
        let distance = target_distance(ctx, target);

        // At short distances NPCs will typically backoff until they reach about
        // 300-400 NIF units distance.
        // Otherwise they may circle around the player in close proximity. If there
        // is not enough space to backoff they will circle instead.
        if distance < SHORT_DISTANCE {
            if rand::random_bool(0.8) {
                let fallback_distance =
                    rand::random_range(MIN_FALLBACK_DISTANCE..MAX_FALLBACK_DISTANCE);
                let fallback_angle = rand::random_range(-MAX_FALLBACK_ANGLE..MAX_FALLBACK_ANGLE);
                let fallback_dir = (pos.translation - target_pos.translation)
                    .normalize()
                    .rotate_y(fallback_angle)
                    * fallback_distance;
                let fallback_point = pos.translation + fallback_dir;

                if fallback_distance > MIN_STEP_DIST {
                    self.move_to(ctx, fallback_point);
                    return;
                }
            }

            if rand::random_bool(0.8) {
                let angle = rand::random_range(-FRAC_PI_2..FRAC_PI_2);

                let p = Quat::from_rotation_y(angle) * Vec3::new(0.0, 0.0, -distance);
                let dst = target_pos.translation + p;

                if dst.distance_squared(pos.translation) > MIN_STEP_DIST * MIN_STEP_DIST {
                    self.move_to(ctx, dst);
                    return;
                }
            }

            return;
        }

        // Circle around the target.
        if distance < MEDIUM_DISTANCE {
            if rand::random_bool(0.3) {
                let angle = rand::random_range(-FRAC_PI_2..FRAC_PI_2);

                let p = Quat::from_rotation_y(angle) * Vec3::new(0.0, 0.0, -distance);
                let dst = target_pos.translation + p;

                if dst.distance_squared(pos.translation) > MIN_STEP_DIST * MIN_STEP_DIST {
                    self.move_to(ctx, dst);
                    return;
                }
            }

            // Small step left or right.
            if rand::random_bool(0.8) {
                let dir = Vec3::cross(target_pos.translation - pos.translation, Vec3::Y)
                    .normalize_or_zero();
                let dst = pos.translation
                    + dir * rand::random_range(MIN_STEP_DISTANCE..MAX_STEP_DISTANCE);

                self.move_to(ctx, dst);
                return;
            }

            // Move towards target.
            if rand::random_bool(0.3) {
                let dir = (target_pos.translation - pos.translation).normalize_or_zero();
                let dst = pos.translation
                    + dir * rand::random_range(MIN_STEP_DISTANCE..MAX_STEP_DISTANCE);

                self.move_to(ctx, dst);
                return;
            }
        }
    }

    fn move_to(&mut self, ctx: &mut Context<'_>, mut target: Vec3) {
        debug_assert!(self.movement.movement.is_none());

        let navmesh = ctx.navmeshes.get(&ctx.worldspace).unwrap();

        if let Some(toi) = navmesh.cast_ray(Ray::new(target, Vec3::Y), f32::MAX) {
            target = target + toi * Vec3::Y;
        } else if let Some(toi) = navmesh.cast_ray(Ray::new(target, -Vec3::Y), f32::MAX) {
            target = target - toi * Vec3::Y;
        }

        let mut moveto = MoveTo::new(target, 225.0);
        moveto.start(ctx);
        self.movement.movement = Some(moveto);
        self.movement.last_action = Instant::now();
        self.movement.interrupt_at =
            Instant::now() + rand::random_range(MIN_MOVE_INTERRUPT_TIME..MAX_MOVE_INTERRUPT_TIME);
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

#[derive(Clone, Debug)]
struct MovementStrategy {
    movement: Option<MoveTo>,
    last_action: Instant,
    interrupt_at: Instant,
    idle_until: Instant,
}
