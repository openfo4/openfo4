use game_common::components::{GlobalTransform, Transform};
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::{Quat, Vec3};
use hecs::{CommandBuffer, Entity, World};
use openfo4_animation::skeleton::SkeletonInstance;

use crate::components::Camera;

#[derive(Copy, Clone, Debug)]
pub struct CameraOffset {
    pub transform: Transform,
    pub rotation: Quat,
}

#[derive(Copy, Clone, Debug)]
pub struct PlayerControlled;

#[derive(Copy, Clone, Debug)]
pub enum CameraMode {
    Free,
    First { entity: Entity },
    Third,
}

pub fn attach_camera(world: &mut World, entity: Entity) {
    let skeleton = world.get::<&SkeletonInstance>(entity).unwrap();

    let bone_index = skeleton
        .skeleton
        .bones
        .iter()
        .find_map(|bone| (bone.name.to_lowercase() == "camera").then_some(bone.index))
        .unwrap();

    let transform = *skeleton.bones_bind_pose.get(&bone_index).unwrap();
    drop(skeleton);

    world
        .insert_one(
            entity,
            CameraOffset {
                transform: Transform {
                    translation: transform.translation,
                    rotation: transform.rotation,
                    scale: Vec3::ONE,
                },
                rotation: Quat::IDENTITY,
            },
        )
        .unwrap();
    world.insert_one(entity, PlayerControlled).unwrap();

    // for (_, camera) in world.query_mut::<&mut Camera>() {
    //     camera.mode = CameraMode::First { parent: entity };
    // }
}

pub fn handle_free_camera(world: &mut World, event: &WindowEvent) {
    let entity = world
        .query_mut::<&PlayerControlled>()
        .into_iter()
        .find_map(|(entity, _)| Some(entity));

    for (_, (transform, camera)) in world.query_mut::<(&mut Transform, &mut Camera)>() {
        // if !matches!(camera.mode, CameraMode::Free) {
        //     continue;
        // }

        match &event {
            WindowEvent::MouseMotion(event) => {
                let yaw = event.delta.x * 0.001;
                let pitch = event.delta.y * 0.001;

                let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
                let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

                transform.rotation = q1 * transform.rotation;
                transform.rotation = transform.rotation * q2;
                transform.rotation = transform.rotation.normalize();
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
                if event.key_code == Some(KeyCode::Tab) && event.state.is_pressed() {
                    match camera.mode {
                        CameraMode::Free => {
                            if let Some(entity) = entity {
                                camera.mode = CameraMode::First { entity };
                            }
                        }
                        _ => camera.mode = CameraMode::Free,
                    }
                }

                transform.translation += transform.rotation * direction;
            }
            _ => (),
        }
    }
}

pub fn update_camera_transform(world: &mut World) {
    let mut buffer = CommandBuffer::new();

    for (entity, camera) in world.query::<&Camera>().iter() {
        let parent = match camera.mode {
            CameraMode::First { entity: parent } => parent,
            _ => continue,
        };

        let offset = world.get::<&CameraOffset>(parent).ok();

        let parent_transform = world.get::<&GlobalTransform>(parent).unwrap();
        let mut transform = parent_transform.0;

        if let Some(offset) = offset {
            transform *= offset.transform;
            transform.rotation *= offset.rotation;
        }

        buffer.insert(entity, (transform, GlobalTransform(transform)));
    }

    buffer.run_on(world);
}
