use game_common::components::Transform;
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::{Quat, Vec3};
use openfo4_core::Frozen;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_physics::components::ColliderShape;
use slab::Slab;

use crate::components::{Camera, MovementState, Player};
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::AnimationEventQueue;
use crate::state::npc::ActorMovementSpeeds;
use crate::systems::camera::{CameraMode, CameraOffset, PlayerControlled};
use crate::systems::movement::{MovementDirection, MovementSpeed};
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
            WindowEvent::KeyboardInput(event) => match event.key_code {
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
            },
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

pub fn apply_player_movements(world: &mut World) {
    let mut buffer = CommandBuffer::new();

    for (entity, state, speeds, frozen, _) in world.query_mut::<(
        Entity,
        &PlayerControlState,
        &ActorMovementSpeeds,
        Option<&Frozen>,
        &Player,
    )>() {
        if frozen.is_some_and(|v| v.controls) {
            buffer.insert_one(entity, MovementDirection::ZERO);
            continue;
        }

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

        direction = direction.normalize_or_zero();

        let f_forward = f32::from(direction.z < 0.0) * direction.z * direction.z;
        let f_backward = f32::from(direction.z > 0.0) * direction.z * direction.z;
        let f_left = f32::from(direction.x < 0.0) * direction.x * direction.x;
        let f_right = f32::from(direction.x > 0.0) * direction.x * direction.x;

        let mut speed = 0.0;
        if state.sneaking {
            // FIXME: What to use here, all speeds don't seem to match the game speeds?
            speed += f_forward * speeds.sneak.forward.run;
            speed += f_backward * speeds.sneak.backward.run;
            speed += f_left * speeds.sneak.left.run;
            speed += f_right * speeds.sneak.right.run;
        } else if state.sprinting {
            speed += f_forward * speeds.default.forward.sprint;
            speed += f_backward * speeds.default.backward.sprint;
            speed += f_left * speeds.default.left.sprint;
            speed += f_right * speeds.default.right.sprint;
        } else if state.walking {
            speed += f_forward * speeds.default.forward.walk;
            speed += f_backward * speeds.default.backward.walk;
            speed += f_left * speeds.default.left.walk;
            speed += f_right * speeds.default.right.walk;
        } else {
            speed += f_forward * speeds.default.forward.run;
            speed += f_backward * speeds.default.backward.run;
            speed += f_left * speeds.default.left.run;
            speed += f_right * speeds.default.right.run;
        }

        buffer.insert(
            entity,
            (
                MovementDirection::new_normalized(direction),
                MovementSpeed { speed },
                MovementState {
                    is_moving: direction != Vec3::ZERO,
                    is_sneaking: state.sneaking,
                    is_sprinting: state.sprinting,
                },
            ),
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
