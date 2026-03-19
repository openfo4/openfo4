use game_common::components::{GlobalTransform, Transform};
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::{Quat, Vec3};
use hecs::{CommandBuffer, Entity, World};
use openfo4_animation::skeleton::SkeletonInstance;

// Distances in CK units.
const CAMERA_DISTANCE_MIN: f32 = 150.0;
const CAMERA_DISTANCE_MAX: f32 = 400.0;

use crate::assets::nif::NIF_TO_METERS;
use crate::components::Camera;

#[derive(Copy, Clone, Debug)]
pub struct CameraOffset {
    /// Rotation applied on top of the actor entity rotation.
    pub rotation: Quat,
    pub entity: Entity,
}

#[derive(Copy, Clone, Debug)]
pub struct PlayerControlled;

#[derive(Copy, Clone, Debug)]
pub enum CameraMode {
    Free,
    First { entity: Entity },
    Third { entity: Entity, distance: f32 },
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

    let camera_entity = skeleton.bones[&bone_index];
    drop(skeleton);

    world
        .insert_one(
            entity,
            CameraOffset {
                rotation: Quat::IDENTITY,
                entity: camera_entity,
            },
        )
        .unwrap();
    world.insert_one(entity, PlayerControlled).unwrap();
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
            WindowEvent::MouseWheel(event) => match &mut camera.mode {
                CameraMode::Third {
                    entity: _,
                    distance,
                } => {
                    *distance =
                        (*distance + event.y * 2.0).clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX);
                }
                _ => (),
            },
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
                                camera.mode = CameraMode::Third {
                                    entity,
                                    distance: CAMERA_DISTANCE_MIN,
                                };
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
        let mut dist = 0.0;
        let parent = match camera.mode {
            CameraMode::First { entity: parent } => parent,
            CameraMode::Third { entity, distance } => {
                dist = distance;
                entity
            }
            _ => continue,
        };

        let offset = world.get::<&CameraOffset>(parent).ok();

        let parent_transform = world.get::<&GlobalTransform>(parent).unwrap();
        let mut transform = parent_transform.0;

        if let Some(offset) = offset {
            transform = world.get::<&GlobalTransform>(offset.entity).unwrap().0;

            transform.rotation *= offset.rotation;
            transform.translation += transform.rotation * Vec3::new(0.0, 0.0, dist * NIF_TO_METERS);
        }

        buffer.insert(entity, (transform, GlobalTransform(transform)));
    }

    buffer.run_on(world);
}
