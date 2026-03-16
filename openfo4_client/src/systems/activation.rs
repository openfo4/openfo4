use std::time::{Duration, Instant};

use game_common::components::{GlobalTransform, Transform};
use game_common::math::Ray;
use game_input::keyboard::KeyCode;
use game_window::events::WindowEvent;
use glam::Vec3;
use hecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::{Collider, CollisionLayer};
use openfo4_physics::{PhysicsPipeline, QueryFilter};

use crate::assets::nif::NIF_TO_METERS;
use crate::components::{Camera, LookingAtTarget, Player};
use crate::render::physics_debug::{DespawnAt, HitscanRaycast};
use crate::systems::ActivationEvent;
use crate::systems::camera::{CameraMode, CameraOffset};

const ACTIVATION_REACH: f32 = 10.0;

pub fn handle_activation_inputs(
    world: &mut World,
    event: &WindowEvent,
    events: &mut EventBus,
    physics_pipeline: &PhysicsPipeline,
) {
    match event {
        WindowEvent::KeyboardInput(event) => {
            if event.key_code != Some(KeyCode::F) || !event.state.is_pressed() {
                return;
            }
        }
        _ => return,
    }

    // // Disable activation in free cam mode.
    // for (_, camera) in world.query_mut::<&Camera>() {
    //     if matches!(camera.mode, CameraMode::Free) {
    //         return;
    //     }
    // }

    // Get the camera transform.
    // let mut camera_transform = Transform::default();
    let Some(camera_transform) = world
        .query::<(&Transform, &Camera)>()
        .iter()
        .map(|(_, (v, _))| *v)
        .next()
    else {
        return;
    };

    // for (_, camera) in world.query::<&Camera>().iter() {
    //     let mut dist = 0.0;
    //     let parent = match camera.mode {
    //         CameraMode::First { entity: parent } => parent,
    //         CameraMode::Third { entity, distance } => {
    //             dist = distance;
    //             entity
    //         }
    //         _ => continue,
    //     };

    //     let offset = world.get::<&CameraOffset>(parent).ok();

    //     let parent_transform = world.get::<&GlobalTransform>(parent).unwrap();
    //     let mut transform = parent_transform.0;

    //     if let Some(offset) = offset {
    //         transform = world.get::<&GlobalTransform>(offset.entity).unwrap().0;

    //         transform.translation += transform.rotation * Vec3::new(0.0, 0.0, dist * NIF_TO_METERS);
    //         transform.rotation = offset.rotation;
    //     }

    //     camera_transform = transform;
    // }

    let Some((player, (&transform, _))) = world
        .query_mut::<(&Transform, &Player)>()
        .into_iter()
        .next()
    else {
        return;
    };

    // We must exclude the actor itself, otherwise we're just gonna
    // hit ourselves immediately.
    let mut exclude_colliders = Vec::new();
    let mut queue = vec![player];
    while let Some(entity) = queue.pop() {
        if world.entity(entity).unwrap().has::<Collider>() {
            exclude_colliders.push(entity);
        }

        if let Some(children) = world.entity(entity).unwrap().get::<&Children>() {
            queue.extend_from_slice(&children.0);
        }
    }

    let ray = Ray {
        origin: camera_transform.translation,
        direction: camera_transform.rotation * -Vec3::Z,
    };

    let Some(target) = physics_pipeline.cast_ray(
        ray,
        ACTIVATION_REACH,
        QueryFilter {
            exclude_colliders: &exclude_colliders,
            layers: &[CollisionLayer::STATIC],
        },
    ) else {
        return;
    };

    // Compute whether the target is reachable from the player actor.
    // if ray.point(target.toi).distance(transform.translation) > ACTIVATION_REACH {
    //     return;
    // }

    world.spawn((
        HitscanRaycast {
            ray,
            max_toi: ACTIVATION_REACH,
            impact_toi: Some(target.toi),
        },
        DespawnAt(Instant::now() + Duration::from_secs(5)),
    ));

    events.send(ActivationEvent {
        actor: player,
        target: target.entity,
    });
}

pub fn update_player_looking_at_target(world: &mut World, physics_pipeline: &PhysicsPipeline) {
    let Some(camera_transform) = world
        .query::<(&Transform, &Camera)>()
        .iter()
        .map(|(_, (v, _))| *v)
        .next()
    else {
        return;
    };

    let Some((player, _)) = world.query_mut::<&Player>().into_iter().next() else {
        return;
    };

    // We must exclude the actor itself, otherwise we're just gonna
    // hit ourselves immediately.
    let mut exclude_colliders = Vec::new();
    let mut queue = vec![player];
    while let Some(entity) = queue.pop() {
        if world.entity(entity).unwrap().has::<Collider>() {
            exclude_colliders.push(entity);
        }

        if let Some(children) = world.entity(entity).unwrap().get::<&Children>() {
            queue.extend_from_slice(&children.0);
        }
    }

    let ray = Ray {
        origin: camera_transform.translation,
        direction: camera_transform.rotation * -Vec3::Z,
    };

    let Some(target) = physics_pipeline.cast_ray(
        ray,
        ACTIVATION_REACH,
        QueryFilter {
            exclude_colliders: &exclude_colliders,
            layers: &[CollisionLayer::STATIC],
        },
    ) else {
        world.remove_one::<LookingAtTarget>(player).ok();
        return;
    };

    world
        .insert_one(player, LookingAtTarget(target.entity))
        .unwrap();
}
