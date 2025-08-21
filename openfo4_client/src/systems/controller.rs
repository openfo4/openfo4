use std::f32::consts::PI;

use game_common::components::Transform;
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::{Quat, Vec3};
use hecs::World;
use openfo4_animation::Value;
use openfo4_animation::driver::AnimationDriver;
use openfo4_physics::PhysicsPipeline;
use openfo4_physics::components::ColliderShape;
use openfo4_physics::controller::CharacterController;

use crate::assets::nif_v2::NIF_TO_METERS;
use crate::components::Camera;
use crate::state::UPS;
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::AnimationEventQueue;
use crate::systems::PlayerControlState;
use crate::systems::camera::{CameraMode, CameraOffset, PlayerControlled};

pub fn handle_inputs(world: &mut World, actions: &Actions, event: &WindowEvent) {
    for (_, camera) in world.query_mut::<&Camera>() {
        if matches!(camera.mode, CameraMode::Free) {
            return;
        }
    }

    for (_, (transform, offset, _, queue, state, driver)) in world.query_mut::<(
        &mut Transform,
        &mut CameraOffset,
        &PlayerControlled,
        &mut AnimationEventQueue,
        &mut PlayerControlState,
        &mut AnimationDriver,
    )>() {
        match event {
            WindowEvent::MouseMotion(event) => {
                let yaw = event.delta.x * 0.001;
                let pitch = event.delta.y * 0.001;

                let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
                let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

                offset.rotation = q1 * offset.rotation;
                offset.rotation = offset.rotation * q2;
                offset.rotation = offset.rotation.normalize();

                transform.rotation = (q1 * transform.rotation).normalize();
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

pub fn apply_player_movements(world: &mut World, pipeline: &mut PhysicsPipeline) {
    for (entity, (transform, offset, state, controller, collider, driver)) in world.query_mut::<(
        &mut Transform,
        &CameraOffset,
        &mut PlayerControlState,
        Option<&mut CharacterController>,
        &ControllerShape,
        Option<&mut AnimationDriver>,
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
        let mut translation = offset.rotation * direction;
        translation.y = 0.0;
        translation = translation.normalize_or_zero();

        // let speed = 92.55;
        let speed = 500.0;

        // Speed is in units per second.
        // We are on a fixed timestep of 60 updates per second.
        translation *= speed * NIF_TO_METERS / UPS as f32;

        if let Some(controller) = controller {
            let movement = controller.move_shape(
                pipeline,
                *transform * collider.offset,
                &collider.shape,
                translation,
                entity,
            );

            transform.translation += movement.translation;

            let gravity_movement = controller.apply_gravity(
                pipeline,
                *transform * collider.offset,
                &collider.shape,
                entity,
            );

            transform.translation += gravity_movement.translation;
        } else {
            transform.translation += translation;
        }

        if let Some(driver) = driver {
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

            if let Some(value) = driver.get_variable_mut("Direction") {
                *value = Value::F32(dir);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControllerShape {
    pub offset: Transform,
    pub shape: ColliderShape,
}
