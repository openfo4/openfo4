use std::f32::consts::PI;
use std::time::Instant;

use game_common::components::{Axis, Color, GlobalTransform, PointLight, Transform};
use game_common::math::Ray;
use game_gizmos::Gizmos;
use glam::{Quat, Vec3};
use openfo4_animation::skeleton::BoneMarker;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::{Collider, ColliderShape};

use crate::cell::CellId;
use crate::components::{Camera, NiTransform, Player, TriggerBox, WorldSpace};
use crate::state::npc::BodyPartBone;
use crate::systems::ai::AiSystem;
use crate::systems::door::{AUTOLOAD_DOOR_RADIUS, Door};

const COLOR: Color = Color::RED;

const HITSCAN_RAYCAST_LINE_HIT_COLOR: Color = Color::GREEN;
const HITSCAN_RAYCAST_LINE_MISS_COLOR: Color = Color::RED;
const HITSCAN_RAYCAST_HIT_COLOR: Color = Color::GREEN;

const AUTO_DOOR_COLOR: Color = Color::GREEN;

const NAVMESH_TRIANGLE_COLOR: Color = Color::BLUE;
const NAVMESH_PATH_COLOR: Color = Color::RED;

#[derive(Copy, Clone, Debug)]
pub struct HitscanRaycast {
    pub ray: Ray,
    pub max_toi: f32,
    pub impact_toi: Option<f32>,
}

/// Entities with this component are despawned when the time is reached.
#[derive(Copy, Clone, Debug)]
pub struct DespawnAt(pub Instant);

pub fn despawn_expired(world: &mut World) {
    let mut commands = CommandBuffer::new();

    let now = Instant::now();
    for (entity, expires) in world.query::<(Entity, &DespawnAt)>().iter() {
        if now >= expires.0 {
            commands.despawn(entity);
        }
    }

    commands.run_on(world);
}

pub fn draw_colliers(world: &World, gizmos: &Gizmos) {
    for (transform, collider, body_part) in world
        .query::<(&GlobalTransform, &Collider, Option<&BodyPartBone>)>()
        .iter()
    {
        let transform = transform.0;

        let color = if body_part.is_some() {
            Color::BLUE
        } else {
            Color::RED
        };

        match &collider.shape {
            ColliderShape::Capsule(shape) => match shape.axis {
                Axis::X => {}
                Axis::Y => {
                    let half_height = shape.half_height * transform.scale.y;
                    let radius = shape.radius * transform.scale.x.midpoint(transform.scale.z);

                    let a = transform.translation + Vec3::new(0.0, half_height, 0.0);
                    let b = transform.translation - Vec3::new(0.0, half_height, 0.0);

                    // Cyclinder
                    for offset in [
                        Vec3::new(radius, 0.0, 0.0),
                        Vec3::new(-radius, 0.0, 0.0),
                        Vec3::new(0.0, 0.0, radius),
                        Vec3::new(0.0, 0.0, -radius),
                    ] {
                        gizmos.line(a + offset, b + offset, color);
                    }

                    gizmos.circle(a, Vec3::Y, radius, color);
                    gizmos.circle(b, Vec3::Y, radius, color);

                    gizmos.arc(a, Quat::from_rotation_x(PI / 2.0), PI, radius, color);
                    gizmos.arc(
                        a,
                        Quat::from_rotation_x(PI / 2.0) * Quat::from_rotation_z(PI / 2.0),
                        PI,
                        radius,
                        color,
                    );

                    gizmos.arc(b, Quat::from_rotation_x(-PI / 2.0), PI, radius, color);
                    gizmos.arc(
                        b,
                        Quat::from_rotation_x(-PI / 2.0) * Quat::from_rotation_z(-PI / 2.0),
                        PI,
                        radius,
                        color,
                    );
                }
                Axis::Z => {}
            },
            ColliderShape::TriMesh(mesh) => {
                for indices in mesh.indices().chunks(3) {
                    let a = transform.transform_point(mesh.vertices()[indices[0] as usize]);
                    let b = transform.transform_point(mesh.vertices()[indices[1] as usize]);
                    let c = transform.transform_point(mesh.vertices()[indices[2] as usize]);

                    gizmos.line(a, b, color);
                    gizmos.line(b, c, color);
                    gizmos.line(c, a, color);
                }
            }
        }
    }
}

pub fn draw_translations(world: &World, gizmos: &Gizmos) {
    for transform in world.query::<&GlobalTransform>().iter() {
        gizmos.line(
            transform.0.translation,
            transform.0.translation + Vec3::X,
            Color::RED,
        );
        gizmos.line(
            transform.0.translation,
            transform.0.translation + Vec3::Y,
            Color::GREEN,
        );
        gizmos.line(
            transform.0.translation,
            transform.0.translation + Vec3::Z,
            Color::BLUE,
        );
    }
}

pub fn draw_cell_boundaries(world: &World, gizmos: &Gizmos) {
    for (transform, _) in world.query::<(&GlobalTransform, &Camera)>().iter() {
        let transform = transform.0;
        let cell = CellId::from(NiTransform::from(transform));

        let min = Transform::from(NiTransform::from_translation(cell.min().extend(0.0)))
            .translation
            .with_y(transform.translation.y - 512.0);
        let max = Transform::from(NiTransform::from_translation(cell.max().extend(0.0)))
            .translation
            .with_y(transform.translation.y + 512.0);

        for (start, end) in [
            (
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(min.x, max.y, min.z),
            ),
            (
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(min.x, max.y, max.z),
            ),
            (
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(max.x, max.y, min.z),
            ),
            (
                Vec3::new(max.x, min.y, max.z),
                Vec3::new(max.x, max.y, max.z),
            ),
        ] {
            gizmos.line(start, end, COLOR);
        }
    }
}

pub fn draw_skeletons(world: &World, gizmos: &Gizmos) {
    for (transform, _, children) in world
        .query::<(&GlobalTransform, &BoneMarker, Option<&Children>)>()
        .iter()
    {
        let Some(children) = children else {
            continue;
        };

        for children in &children.0 {
            let Ok(child_transform) = world.get::<&GlobalTransform>(*children) else {
                continue;
            };

            gizmos.line(
                transform.0.translation,
                child_transform.0.translation,
                COLOR,
            );
        }
    }
}

pub fn draw_light_markers(world: &World, gizmos: &Gizmos) {
    for (transform, light) in world.query::<(&GlobalTransform, &PointLight)>().iter() {
        gizmos.sphere(transform.0.translation, light.radius, light.color);
    }
}

pub fn draw_hitscan_markers(world: &World, gizmos: &Gizmos) {
    for raycast in world.query::<&HitscanRaycast>().iter() {
        let toi = raycast.impact_toi.unwrap_or(raycast.max_toi);

        let color = if raycast.impact_toi.is_some() {
            HITSCAN_RAYCAST_LINE_HIT_COLOR
        } else {
            HITSCAN_RAYCAST_LINE_MISS_COLOR
        };

        gizmos.line(raycast.ray.origin, raycast.ray.point(toi), color);

        if let Some(toi) = raycast.impact_toi {
            raycast.ray.point(toi);

            gizmos.circle(
                raycast.ray.point(toi),
                raycast.ray.direction,
                0.5,
                HITSCAN_RAYCAST_HIT_COLOR,
            );
        }
    }
}

pub fn draw_doors(world: &World, gizmos: &Gizmos) {
    for (transform, door) in world.query::<(&Transform, &Door)>().iter() {
        if !matches!(*door, Door::AutoPortal(_)) {
            continue;
        }

        gizmos.sphere(transform.translation, AUTOLOAD_DOOR_RADIUS, AUTO_DOOR_COLOR);
    }
}

pub fn draw_trigger_boxes(world: &World, gizmos: &Gizmos) {
    for (transform, trigger_box) in world.query::<(&Transform, &TriggerBox)>().iter() {
        let min = transform.translation - trigger_box.half_extents;
        let max = transform.translation + trigger_box.half_extents;

        for (start, end) in [
            (
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(min.x, min.y, max.z),
            ),
            (
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(min.x, max.y, min.z),
            ),
            (
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(max.x, min.y, min.z),
            ),
            (
                Vec3::new(max.x, max.y, max.z),
                Vec3::new(max.x, max.y, min.z),
            ),
            (
                Vec3::new(max.x, max.y, max.z),
                Vec3::new(max.x, min.y, max.z),
            ),
            (
                Vec3::new(max.x, max.y, max.z),
                Vec3::new(min.x, max.y, max.z),
            ),
            (
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(max.x, max.y, min.z),
            ),
            (
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(max.x, min.y, max.z),
            ),
            (
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(max.x, max.y, min.z),
            ),
            (
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(min.x, max.y, max.z),
            ),
            (
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(min.x, max.y, max.z),
            ),
            (
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(max.x, min.y, max.z),
            ),
        ] {
            gizmos.line(start, end, trigger_box.marker_color);
        }
    }
}

pub fn draw_navmesh(world: &World, ai: &AiSystem, gizmos: &Gizmos) {
    let Some(worldspace) = camera_worldspace(world) else {
        return;
    };

    let Some(navmesh) = ai.navmeshes.get(&worldspace) else {
        return;
    };

    for [a, b, c] in navmesh.triangles().iter().map(|v| v.vertices()) {
        gizmos.line(a, b, NAVMESH_TRIANGLE_COLOR);
        gizmos.line(b, c, NAVMESH_TRIANGLE_COLOR);
        gizmos.line(c, a, NAVMESH_TRIANGLE_COLOR);
    }
}

pub fn draw_planned_ai_agent_paths(world: &World, ai: &AiSystem, gizmos: &Gizmos) {
    let Some(worldspace) = camera_worldspace(world) else {
        return;
    };

    let Some(navmesh) = ai.navmeshes.get(&worldspace) else {
        return;
    };

    for (_, (entity, path)) in &ai.paths {
        if world
            .get::<&WorldSpace>(*entity)
            .ok()
            .is_none_or(|ws| *ws != worldspace)
        {
            continue;
        }

        for id in &path.triangles {
            let [a, b, c] = navmesh.triangles()[*id].vertices();

            gizmos.line(a, b, NAVMESH_PATH_COLOR);
            gizmos.line(b, c, NAVMESH_PATH_COLOR);
            gizmos.line(c, a, NAVMESH_PATH_COLOR);
        }

        for &[a, b] in path.waypoints.array_windows::<2>() {
            gizmos.line(a, b, Color::FUCHSIA);
        }
    }
}

fn camera_worldspace(world: &World) -> Option<WorldSpace> {
    // FIXME: We should look at the camera, not the player.
    for (ws, _) in world.query::<(&WorldSpace, &Player)>().iter() {
        return Some(*ws);
    }

    None
}
