use game_common::components::Transform;
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::{Quat, Vec3};
use hecs::World;

use crate::components::Camera;
use crate::systems::camera::{CameraMode, CameraOffset, PlayerControlled};

pub fn handle_inputs(world: &mut World, event: &WindowEvent) {
    for (_, camera) in world.query_mut::<&Camera>() {
        if matches!(camera.mode, CameraMode::Free) {
            return;
        }
    }

    for (_, (transform, offset, _)) in
        world.query_mut::<(&mut Transform, &mut CameraOffset, &PlayerControlled)>()
    {
        match event {
            WindowEvent::MouseMotion(event) => {
                let yaw = event.delta.x * 0.001;
                let pitch = event.delta.y * 0.001;

                let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
                let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

                offset.rotation = q1 * offset.rotation;
                offset.rotation = offset.rotation * q2;
                offset.rotation = offset.rotation.normalize();
            }
            WindowEvent::KeyboardInput(event) => {
                let mut direction = Vec3::ZERO;
                if event.key_code == Some(KeyCode::W) {
                    direction -= Vec3::Z;
                }
                if event.key_code == Some(KeyCode::S) {
                    direction += Vec3::Z;
                }
                if event.key_code == Some(KeyCode::A) {
                    direction -= Vec3::X;
                }
                if event.key_code == Some(KeyCode::D) {
                    direction += Vec3::X;
                }

                // Remove the Y component from the facing direction since
                // we don't want to move up.
                direction = offset.rotation * direction;
                direction.y = 0.0;
                direction = direction.normalize_or_zero();

                transform.translation += direction * 0.2;
            }
            _ => (),
        }
    }
}
