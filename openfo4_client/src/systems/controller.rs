use std::f32::consts::PI;

use game_common::components::Transform;
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::{Quat, Vec3};
use openfo4_core::Frozen;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_physics::PhysicsPipeline;
use openfo4_physics::components::ColliderShape;
use openfo4_physics::controller::CharacterController;
use slab::Slab;

use crate::assets::nif::NIF_TO_METERS;
use crate::components::{Camera, Player};
use crate::state::UPS;
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::AnimationEventQueue;
use crate::state::hardcoded::KEYWORD_FEMALE;
use crate::state::npc::ObjectKeywords;
use crate::systems::animation_sync::AnimationParameters;
use crate::systems::camera::{CameraMode, CameraOffset, PlayerControlled};
use crate::systems::{FacingDirection, GunFireEvent, PlayerControlState};

pub fn handle_inputs(
    world: &mut World,
    actions: &Actions,
    event: &WindowEvent,
    events: &mut EventBus,
) {
    for camera in world.query_mut::<&Camera>() {
        if matches!(camera.mode, CameraMode::Free) {
            return;
        }
    }

    for (entity, mut transform, mut offset, _, mut queue, mut state) in world.query_mut::<(
        Entity,
        &mut Transform,
        &mut CameraOffset,
        &PlayerControlled,
        &mut AnimationEventQueue,
        &mut PlayerControlState,
    )>() {
        match event {
            WindowEvent::MouseMotion(event) => {
                let yaw = event.delta.x * 0.001;
                let pitch = event.delta.y * 0.001;

                let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
                let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

                transform.rotation = (q1 * transform.rotation).normalize();
                offset.rotation = (offset.rotation * q2).normalize();
            }
            WindowEvent::KeyboardInput(event) => {
                let old_state = state.clone();

                match event.key_code {
                    Some(KeyCode::W) => {
                        state.moving_front = event.state.is_pressed();
                    }
                    Some(KeyCode::S) => {
                        state.moving_back = event.state.is_pressed();
                    }
                    Some(KeyCode::A) => {
                        state.moving_left = event.state.is_pressed();
                    }
                    Some(KeyCode::D) => {
                        state.moving_right = event.state.is_pressed();
                    }
                    Some(KeyCode::LControl) => {
                        if !event.state.is_pressed() {
                            continue;
                        }

                        if let Some(form) = actions.0.get(&Action::Sneak) {
                            queue.0.push_back(*form);
                        }

                        state.sneaking ^= true;
                    }
                    Some(KeyCode::LShift) => {
                        let action = if state.sprinting {
                            Action::SprintStop
                        } else {
                            Action::SprintStart
                        };

                        if let Some(form) = actions.0.get(&action) {
                            queue.0.push_back(*form);
                        }

                        state.sprinting ^= true;
                    }
                    Some(KeyCode::Space) => {
                        if !event.state.is_pressed() {
                            continue;
                        }

                        let action = if state.jumping {
                            state.jumping = false;
                            Action::Land
                        } else {
                            state.jumping = true;
                            Action::Jump
                        };

                        if let Some(form) = actions.0.get(&action) {
                            dbg!(form);
                            queue.0.push_back(*form);
                        }
                    }
                    Some(KeyCode::Key1) if event.state.is_pressed() => {
                        state.walking ^= true;
                    }
                    Some(KeyCode::Key2) if event.state.is_pressed() => {
                        events.send(GunFireEvent { actor: entity });
                    }
                    _ => continue,
                }

                let started_moving = !old_state.moving_front
                    && !old_state.moving_back
                    && !old_state.moving_left
                    && !old_state.moving_right
                    && (state.moving_front
                        || state.moving_back
                        || state.moving_left
                        || state.moving_right);
                let stopped_moving = !state.moving_front
                    && !state.moving_back
                    && !state.moving_left
                    && !state.moving_right
                    && (old_state.moving_front
                        || old_state.moving_back
                        || old_state.moving_left
                        || old_state.moving_right);

                if started_moving {
                    if let Some(form) = actions.0.get(&Action::MoveStart) {
                        queue.0.push_back(*form);
                    }
                }

                if stopped_moving {
                    if let Some(form) = actions.0.get(&Action::MoveStop) {
                        queue.0.push_back(*form);
                    }
                }
            }
            _ => (),
        }
    }
}

pub fn update_facing_direction(world: &mut World) {
    for (_, mut facing_dir, transform, offset) in world
        .query::<(Entity, &mut FacingDirection, &Transform, &CameraOffset)>()
        .iter()
    {
        facing_dir.0 = transform.rotation * Quat::IDENTITY.to_nif() * offset.rotation;
    }
}

pub fn apply_player_movements(world: &mut World, pipeline: &mut PhysicsPipeline) {
    let mut buffer = CommandBuffer::new();

    for (
        entity,
        mut transform,
        facing_dir,
        state,
        mut controller,
        collider,
        actor_keywords,
        player,
        frozen,
    ) in world.query_mut::<(
        Entity,
        &mut Transform,
        &FacingDirection,
        &mut PlayerControlState,
        Option<&mut CharacterController>,
        &ControllerShape,
        &ObjectKeywords,
        Option<&Player>,
        Option<&Frozen>,
    )>() {
        let mut direction = Vec3::ZERO;

        if state.moving_front {
            direction -= Vec3::Z;
        }
        if state.moving_back {
            direction += Vec3::Z;
        }
        if state.moving_left {
            direction -= Vec3::X;
        }
        if state.moving_right {
            direction += Vec3::X;
        }

        // Remove the Y component from the facing direction since
        // we don't want to move up.
        let mut translation = facing_dir.0 * direction;
        translation.y = 0.0;
        translation = translation.normalize_or_zero();

        let speed = if state.sprinting {
            500.0
        } else {
            if state.walking { 92.55 } else { 372.0 }
        };

        // Speed is in units per second.
        // We are on a fixed timestep of 60 updates per second.
        translation *= speed * NIF_TO_METERS / UPS as f32;

        if let Some(controller) = &mut controller {
            if frozen.as_ref().is_none_or(|v| !v.controls) {
                let movement = controller.move_shape(
                    pipeline,
                    *transform * collider.offset,
                    &collider.shape,
                    translation,
                    entity,
                );

                transform.translation += movement.translation;
            }

            if frozen.as_ref().is_none_or(|v| !v.physics) {
                let gravity_movement = controller.apply_gravity(
                    pipeline,
                    *transform * collider.offset,
                    &collider.shape,
                    entity,
                );

                transform.translation += gravity_movement.translation;
            }
        } else {
            transform.translation += translation;
        }

        // Compute angle between forward (-Z) vector and the movement vector.
        // 0.0 is front, 0.25 is right, 0.5 is back, 0.75 is left.
        let dir_norm = direction.normalize_or_zero();
        let mut dir = (-dir_norm.z).acos();
        // acos will yield PI/2 for a zero vector, but we want to face forward
        // when we don't move.
        // The length is either zero or normalized, so this will multiply with
        // either 0 or 1.
        dir *= dir_norm.length_squared();

        if (dir_norm.x).is_sign_negative() {
            dir = PI + (PI - dir);
        }

        dir /= 2.0 * PI;

        // Both rotation are world-space, but we want entity-space, so
        // we must remove the entity rotation.
        // The result is in the NIF coordinate system.
        let facing_direction = (transform.rotation.conjugate() * facing_dir.0) * -Vec3::Z;

        buffer.insert_one(
            entity,
            AnimationParameters {
                speed: speed,
                direction: dir,
                is_moving: state.moving_front
                    || state.moving_back
                    || state.moving_left
                    || state.moving_right,
                is_sprinting: state.sprinting,
                is_player: player.is_some(),
                is_female: actor_keywords.contains(KEYWORD_FEMALE),
                is_jumping: state.jumping,
                facing_direction,
            },
        );
    }

    buffer.run_on(world);
}

#[derive(Clone, Debug)]
pub struct ControllerShape {
    pub offset: Transform,
    pub shape: ColliderShape,
}

/// Defines which inputs are possible for this controller.
#[derive(Clone, Debug, Default)]
pub struct InputControllerMask {
    pub layers: Slab<InputControllerLayer>,
}

impl InputControllerMask {
    /// Returns whether activate controls are enabled.
    pub fn activate(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.activate)
    }

    /// Returns whether camera switch controls are enabled.
    pub fn camera_switch(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.camera_switch)
    }

    /// Returns whether fast travel controls are enabled.
    pub fn fast_travel(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.fast_travel)
    }

    /// Returns whether favorites controls are enabled.
    pub fn favorites(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.favorites)
    }

    /// Returns whether fighting controls are enabled.
    pub fn fighting(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.fighting)
    }

    /// Returns whether journal controls are enabled.
    pub fn journal(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.journal)
    }

    /// Returns whether jumping controls are enabled.
    pub fn jumping(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.jumping)
    }

    /// Returns whether looking controls are enabled.
    pub fn looking(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.looking)
    }

    /// Returns whether menu controls are enabled.
    pub fn menu(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.menu)
    }

    /// Returns whether movement controls are enabled.
    pub fn movement(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.movement)
    }

    /// Returns whether sneaking controls are enabled.
    pub fn sneaking(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.sneaking)
    }

    /// Returns whether VATS controls are enabled.
    pub fn vats(&self) -> bool {
        self.layers.iter().all(|(_, l)| l.vats)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct InputControllerLayer {
    pub activate: bool,
    pub camera_switch: bool,
    pub fast_travel: bool,
    pub favorites: bool,
    pub fighting: bool,
    pub journal: bool,
    pub jumping: bool,
    pub looking: bool,
    pub menu: bool,
    pub movement: bool,
    pub running: bool,
    pub sneaking: bool,
    pub sprinting: bool,
    pub vats: bool,
    pub z_key: bool,
}

impl Default for InputControllerLayer {
    fn default() -> Self {
        Self {
            activate: true,
            camera_switch: true,
            fast_travel: true,
            favorites: true,
            fighting: true,
            journal: true,
            jumping: true,
            looking: true,
            menu: true,
            movement: true,
            running: true,
            sneaking: true,
            sprinting: true,
            vats: true,
            z_key: true,
        }
    }
}
