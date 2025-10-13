use std::f32::consts::PI;

use game_common::components::{Axis, Color, GlobalTransform, PointLight, Transform};
use game_gizmos::Gizmos;
use glam::{Quat, Vec3};
use hecs::World;
use openfo4_animation::skeleton::BoneMarker;
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::{Collider, ColliderShape};

use crate::cell::CellId;
use crate::components::{Camera, NiTransform};
use crate::state::npc::BodyPartBone;

const COLOR: Color = Color::RED;

pub fn draw_colliers(world: &World, gizmos: &Gizmos) {
    for (_, (transform, collider, body_part)) in world
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
    for (_, transform) in world.query::<&GlobalTransform>().iter() {
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
    for (_, (transform, _)) in world.query::<(&GlobalTransform, &Camera)>().iter() {
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
    for (_, (transform, bone, children)) in world
        .query::<(&GlobalTransform, &BoneMarker, Option<&Children>)>()
        .iter()
    {
        let Some(children) = children else {
            continue;
        };

        for children in &children.0 {
            let child_transform = world.get::<&GlobalTransform>(*children).unwrap();

            gizmos.line(
                transform.0.translation,
                child_transform.0.translation,
                COLOR,
            );
        }
    }
}

pub fn draw_light_markers(world: &World, gizmos: &Gizmos) {
    for (_, (transform, light)) in world.query::<(&GlobalTransform, &PointLight)>().iter() {
        gizmos.sphere(transform.0.translation, light.radius, light.color);
    }
}
