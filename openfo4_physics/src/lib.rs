pub mod components;
pub mod controller;
pub mod debug;

mod convert;

use std::cell::Cell;
use std::collections::{HashMap, VecDeque};

use components::{ColliderShape, RigidBody, RigidBodyKind};
use game_common::collections::bimap::BiMap;
use game_common::components::{Axis, GlobalTransform, Transform};
use game_common::math::Ray;
use game_gizmos::Gizmos;
use game_tracing::trace_span;
use glam::Vec3;
use hecs::{Entity, World};
use openfo4_core::Frozen;
use openfo4_core::hierarchy::Children;
use rapier3d::math::Isometry;
use rapier3d::parry::query::{DefaultQueryDispatcher, ShapeCastOptions};
use rapier3d::prelude::{
    CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, DebugRenderMode, DebugRenderPipeline,
    DebugRenderStyle, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager,
    MultibodyJointSet, NarrowPhase, QueryPipeline, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
    RigidBodyType, SharedShape, Vector,
};
use tracing::instrument;

use crate::components::CollisionLayer;
use crate::convert::{quat, rotation, vec3, vector};
use crate::debug::GizmoRenderBackend;

const DT: f32 = 1.0 / 60.0;
const MIN_CCD_DT: f32 = DT / 100.0;
const GRAVITY: Vector<f32> = Vector::new(0.0, -9.81, 0.0);

pub struct PhysicsPipeline {
    // Rapier stuff
    pipeline: rapier3d::pipeline::PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    // Our stuff
    body_handles: BiMap<Entity, RigidBodyHandle>,
    /// Maps [`RigidBody`] entities to a list of [`Collider`] entities that are attached to the
    /// [`RigidBody`]. The [`Transform`] represents the offset from the [`Collider`] to the
    /// [`RigidBody`].
    body_children: HashMap<Entity, BodyChildren>,
    collider_handles: HashMap<Entity, SpawnedCollider>,
    collider_handle_entities: HashMap<ColliderHandle, Entity>,
    debug: DebugRenderPipeline,
}

impl PhysicsPipeline {
    pub fn new() -> Self {
        let integration_parameters = IntegrationParameters {
            dt: DT,
            min_ccd_dt: MIN_CCD_DT,
            ..Default::default()
        };

        Self {
            pipeline: rapier3d::pipeline::PhysicsPipeline::new(),
            integration_parameters,
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            body_handles: BiMap::new(),
            collider_handles: HashMap::new(),
            collider_handle_entities: HashMap::new(),
            body_children: HashMap::new(),
            debug: DebugRenderPipeline::new(
                DebugRenderStyle::default(),
                DebugRenderMode::default(),
            ),
        }
    }

    #[instrument(name = "PhysicsPipeline::step", skip_all)]
    pub fn step(&mut self, world: &mut World) {
        self.update_rigid_bodies(world);
        self.update_colliders(world);

        self.pipeline.step(
            &GRAVITY,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );

        self.write_back(world);
    }

    pub fn render_debug(&mut self, gizmos: &Gizmos) {
        self.debug.render(
            &mut GizmoRenderBackend(gizmos),
            &self.bodies,
            &self.colliders,
            &self.impulse_joints,
            &self.multibody_joints,
            &self.narrow_phase,
        );
    }

    #[instrument(name = "PhysicsPipeline::update_rigid_bodies", skip_all)]
    fn update_rigid_bodies(&mut self, world: &World) {
        let mut despawned_bodies = self.body_handles.clone();

        for (entity, (global_transform, rigid_body, frozen)) in world
            .query::<(&GlobalTransform, &RigidBody, Option<&Frozen>)>()
            .iter()
        {
            if frozen.is_some_and(|v| v.physics) {
                continue;
            }

            let translation = vector(global_transform.0.translation);
            let rotation = rotation(global_transform.0.rotation);

            let kind = match rigid_body.kind {
                RigidBodyKind::Fixed => RigidBodyType::Fixed,
                RigidBodyKind::Dynamic => RigidBodyType::Dynamic,
            };
            let linvel = vector(rigid_body.linvel);
            let angvel = vector(rigid_body.angvel);

            if !self.body_handles.contains_left(&entity) {
                let builder = RigidBodyBuilder::new(kind);
                let handle = self.bodies.insert(builder);
                self.body_handles.insert(entity, handle);
            }

            let handle = self.body_handles.get_left(&entity).unwrap();
            let body = self.bodies.get_mut(*handle).unwrap();

            body.set_translation(translation, true);
            body.set_rotation(rotation, true);
            body.set_linvel(linvel, true);
            body.set_angvel(angvel, true);
            body.set_body_type(kind, true);

            let children = collect_collider_children(entity, world);
            match self.body_children.get_mut(&entity) {
                Some(c) => {
                    if c.entities != children {
                        c.entities = children;
                        c.has_updated.set(true);
                    }
                }
                None => {
                    self.body_children.insert(
                        entity,
                        BodyChildren {
                            has_updated: Cell::new(true),
                            entities: children,
                        },
                    );
                }
            }

            despawned_bodies.remove_left(&entity);
        }

        for (entity, handle) in despawned_bodies {
            self.body_children.remove(&entity);
            self.body_handles.remove_left(&entity);
            self.bodies.remove(
                handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                // Don't remove attached colliders, they are removed in the next
                // stage that updates all entities with colliders.
                false,
            );
        }
    }

    // #[instrument(name = "PhysicsPipeline::update_colliders", skip_all)]
    fn update_colliders(&mut self, world: &World) {
        let mut despawned_colliders = self.collider_handles.clone();

        for children in self.body_children.values() {
            for entity in children.entities.keys() {
                despawned_colliders.remove(entity);
            }

            if !children.has_updated.get() {
                continue;
            }

            children.has_updated.set(false);

            for &entity in children.entities.keys() {
                let collider = world.get::<&components::Collider>(entity).unwrap();
                let transform = world.get::<&GlobalTransform>(entity).unwrap().0;

                // If a `Collider` is attached to a `RigidBody` entity,
                // the transform of the `Collider` from the `RigidBody` is used.
                // Otherwise the `Collider` will be attached to first parent
                // that has a `RigidBody`, with the offset applied from the `Collider`s
                // transform.
                // `Collider` without any `RigidBody` parents are ignored.
                let Some(collider_parent) = self.get_collider_parent(entity) else {
                    continue;
                };

                let spawned_collider = self.collider_handles.entry(entity).or_insert_with(|| {
                    let builder =
                        ColliderBuilder::new(build_shape(&collider.shape, transform.scale))
                            .collision_groups(collider.layer.to_interaction_groups());

                    let handle = self.colliders.insert(builder);

                    debug_assert!(!self.collider_handle_entities.contains_key(&handle));
                    self.collider_handle_entities.insert(handle, entity);

                    SpawnedCollider {
                        handle,
                        scale: transform.scale,
                        layer: collider.layer,
                    }
                });

                let state = self.colliders.get_mut(spawned_collider.handle).unwrap();

                // TODO: Handle updated collider shape.

                // The physics engine doesn't allow arbitrary scaling of collision
                // meshes. This means we need to recreate the shape if the scale changes.
                if spawned_collider.scale != transform.scale {
                    let shape = build_shape(&collider.shape, transform.scale);
                    state.set_shape(shape);
                    spawned_collider.scale = transform.scale;
                }

                // Scale of the collider relative to the scale the parent expects us.
                // We need to offset the collider in the coordinate of the parent,
                // which we get when we scale from the collider to the parent sees us.
                let scale_to_parent = transform.scale / collider_parent.transform.scale;
                let translation = collider_parent.transform.translation * scale_to_parent;
                let rotation = collider_parent.transform.rotation;

                state.set_friction(collider.friction);
                state.set_restitution(collider.restitution);
                state.set_collision_groups(collider.layer.to_interaction_groups());

                self.colliders.set_parent(
                    spawned_collider.handle,
                    Some(collider_parent.body),
                    &mut self.bodies,
                );

                // We can only set the translation AFTER setting the parent. Setting the
                // parent requires access to all colliders for some reason. (rapiers API
                // is probably just weird...).
                let state = self.colliders.get_mut(spawned_collider.handle).unwrap();
                state.set_position_wrt_parent(Isometry {
                    translation: vector(translation).into(),
                    rotation: convert::rotation(rotation),
                });
            }
        }

        for (entity, collider) in despawned_colliders {
            self.collider_handles.remove(&entity);
            self.collider_handle_entities.remove(&collider.handle);
            self.colliders
                .remove(collider.handle, &mut self.islands, &mut self.bodies, true);
        }
    }

    fn write_back(&mut self, world: &mut World) {
        for (handle, body) in self.bodies.iter() {
            let entity = *self.body_handles.get_right(&handle).unwrap();

            let mut rigid_body = world.get::<&mut RigidBody>(entity).unwrap();
            rigid_body.linvel = vec3(*body.linvel());
            rigid_body.angvel = vec3(*body.angvel());

            // We source the position of the rigid body from the `GlobalTransform` component,
            // but we cannot write back directly to the `GlobalTransform` component since
            // it will be overwritten in the next frame.
            // Instead we must write to the `Transform` component, but it may differ from the
            // `GlobalTransform` since the rigid body may itself be a children of another entity.

            // To handle this correctly we compute the delta between the absolute `GlobalTransform`
            // (before the physics tick) and the new absolute position (after the physics tick).
            // The delta describes the changed transform in this physics tick and can be applied
            // to `Transform`.

            let mut transform = world.get::<&mut Transform>(entity).unwrap();
            let global_transform = world.get::<&GlobalTransform>(entity).unwrap();

            let new_translation = vec3(*body.translation());
            let new_rotation = quat(*body.rotation());

            let delta_translation = new_translation - global_transform.0.translation;
            // Note that `delta_rotation` is applied from left.
            let delta_rotation = new_rotation * global_transform.0.rotation.conjugate();

            transform.translation += delta_translation;
            transform.rotation = (delta_rotation * transform.rotation).normalize();
        }
    }

    fn get_collider_parent(&self, entity: Entity) -> Option<ColliderParent> {
        // Select a rigid body for the collider. If the entity has a rigid body
        // it is preferred, otherwise we select the rigid body from the parent
        // entity.
        match self.body_handles.get_left(&entity) {
            Some(handle) => Some(ColliderParent {
                body: *handle,
                transform: Transform::IDENTITY,
            }),
            None => {
                for (parent, children) in self.body_children.iter() {
                    if let Some(transform) = children.entities.get(&entity).copied() {
                        let parent_handle = self.body_handles.get_left(parent).unwrap();
                        return Some(ColliderParent {
                            body: *parent_handle,
                            transform,
                        });
                    }
                }

                None
            }
        }
    }

    fn query_pipeline(&self) -> QueryPipeline<'_> {
        self.broad_phase.as_query_pipeline(
            &DefaultQueryDispatcher,
            &self.bodies,
            &self.colliders,
            rapier3d::pipeline::QueryFilter::default(),
        )
    }

    pub fn cast_ray(&self, ray: Ray, max_toi: f32, filter: QueryFilter<'_>) -> Option<QueryHit> {
        let _span = trace_span!("PhysicsPipeline::cast_ray").entered();

        let ray = rapier3d::geometry::Ray {
            origin: vector(ray.origin).into(),
            dir: vector(ray.direction).into(),
        };

        let predicate = |handle: ColliderHandle, _: &rapier3d::prelude::Collider| -> bool {
            let entity = self.collider_handle_entities.get(&handle).unwrap();
            let layer = self.collider_handles.get(entity).unwrap().layer;

            filter.layers.contains(&layer) && !filter.exclude_colliders.contains(entity)
        };

        self.query_pipeline()
            .with_filter(rapier3d::pipeline::QueryFilter {
                predicate: Some(&predicate),
                ..Default::default()
            })
            .cast_ray(&ray, max_toi, true)
            .map(|(collider, toi)| {
                let entity = *self.collider_handle_entities.get(&collider).unwrap();

                QueryHit { entity, toi }
            })
    }

    pub fn cast_shape(
        &self,
        origin: Transform,
        direction: Vec3,
        shape: &ColliderShape,
    ) -> Option<QueryHit> {
        let shape = build_shape(shape, origin.scale);
        let origin = Isometry {
            translation: vector(origin.translation).into(),
            rotation: rotation(origin.rotation),
        };

        self.query_pipeline()
            .cast_shape(
                &origin,
                &vector(direction).into(),
                &*shape,
                ShapeCastOptions::default(),
            )
            .map(|(collider, hit)| {
                let entity = *self.collider_handle_entities.get(&collider).unwrap();

                QueryHit {
                    entity,
                    toi: hit.time_of_impact,
                }
            })
    }
}

#[derive(Clone, Debug)]
struct BodyChildren {
    has_updated: Cell<bool>,
    entities: HashMap<Entity, Transform>,
}

#[derive(Clone, Debug)]
struct SpawnedCollider {
    handle: ColliderHandle,
    scale: Vec3,
    layer: CollisionLayer,
}

/// A reference to a rigid body that a collider should attach to.
#[derive(Copy, Clone, Debug)]
struct ColliderParent {
    /// The handle of the rigid body.
    body: RigidBodyHandle,
    /// The transform of the collider relative to the rigid body.
    transform: Transform,
}

/// Collects and returns all entities with collider components that are children of `entity`.
/// (This also includes `entity` itself).
///
/// The [`Transform`] value of an entity is the local offset from the root `entity`.
///
/// The branch is interrupted if an entity contains no transform, or if it contains a rigid body
/// component, signaling the start of a new rigid body.
fn collect_collider_children(entity: Entity, world: &World) -> HashMap<Entity, Transform> {
    let mut entities = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back((entity, Transform::default()));

    // The root entity can also be a collider.
    if world
        .satisfies::<&components::Collider>(entity)
        .unwrap_or_default()
    {
        entities.insert(entity, Transform::default());
    }

    while let Some((entity, parent_transform)) = queue.pop_front() {
        let Ok(children) = world.get::<&Children>(entity) else {
            continue;
        };

        for &children in &children.0 {
            let Ok(child_transform) = world.get::<&Transform>(children) else {
                continue;
            };

            // If the child itself is a rigid body it should
            // not be considered a child of this rigid body
            // anymore and be detached.
            if world.satisfies::<&RigidBody>(children).unwrap_or_default() {
                continue;
            }

            let transform = parent_transform * *child_transform;

            if world
                .satisfies::<&components::Collider>(children)
                .unwrap_or_default()
            {
                entities.insert(children, transform);
            }

            queue.push_back((children, transform));
        }
    }

    entities
}

fn build_shape(shape: &ColliderShape, scale: Vec3) -> SharedShape {
    match shape {
        ColliderShape::Capsule(shape) => {
            // Capsules can only be scaled in two directions:
            // The height direction and the radius which includes both
            // other directions uniformly.
            // If the radius scale factors are not the same we will take
            // the middle (midpoint) value.
            match shape.axis {
                Axis::X => {
                    let half_height = shape.half_height * scale.x;
                    let radius = shape.radius * scale.y.midpoint(scale.z);
                    SharedShape::capsule_x(half_height, radius)
                }
                Axis::Y => {
                    let half_height = shape.half_height * scale.y;
                    let radius = shape.radius * scale.x.midpoint(scale.z);
                    SharedShape::capsule_y(half_height, radius)
                }
                Axis::Z => {
                    let half_height = shape.half_height * scale.z;
                    let radius = shape.radius * scale.x.midpoint(scale.y);
                    SharedShape::capsule_z(half_height, radius)
                }
            }
        }
        ColliderShape::TriMesh(mesh) => {
            let vertices = mesh
                .vertices()
                .iter()
                .map(|vertex| {
                    let point = *vertex * scale;
                    vector(point).into()
                })
                .collect();
            let indices = mesh
                .indices()
                .chunks(3)
                .map(|indices| indices.try_into().unwrap())
                .collect::<Vec<[u32; 3]>>();

            if !mesh.is_convex() {
                SharedShape::trimesh(vertices, indices).unwrap()
            } else {
                // FIXME: Could use convex_mesh here, but it fails
                // for some convex meshes (e.g. a simple cube)?
                match SharedShape::convex_hull(&vertices) {
                    Some(shape) => shape,
                    None => SharedShape::trimesh(vertices, indices).unwrap(),
                }
            }
        }
    }
}

/// A resulting hit of a [`cast_ray`] or [`cast_shape`].
///
/// [`cast_ray`]: PhysicsPipeline::cast_ray
/// [`cast_shape`]: PhysicsPipeline::cast_shape
#[derive(Copy, Clone, Debug)]
pub struct QueryHit {
    /// The entity with the [`Collider`] attached that was hit.
    pub entity: Entity,
    /// The distance of the entity.
    pub toi: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct QueryFilter<'a> {
    /// Only collide with the given list of layers.
    pub layers: &'a [CollisionLayer],
    /// Exclude all matching entities with a collider.
    pub exclude_colliders: &'a [Entity],
}

#[cfg(test)]
mod tests {
    use game_common::components::{Axis, Transform};
    use glam::Vec3;
    use hecs::World;
    use openfo4_core::hierarchy::Children;

    use crate::collect_collider_children;
    use crate::components::{Capsule, Collider, ColliderShape, CollisionLayer, RigidBody};

    fn create_dummy_collider() -> Collider {
        Collider {
            friction: 1.0,
            restitution: 0.0,
            shape: ColliderShape::Capsule(Capsule {
                axis: Axis::X,
                half_height: 1.0,
                radius: 1.0,
            }),
            layer: CollisionLayer::STATIC,
        }
    }

    #[test]
    fn test_collect_collider_children() {
        let mut world = World::new();
        let e0 = world.spawn((
            Transform::from_translation(Vec3::splat(2.0)),
            create_dummy_collider(),
        ));
        let e1 = world.spawn((
            Transform::from_translation(Vec3::splat(1.0)),
            Children(vec![e0]),
            create_dummy_collider(),
        ));
        let root = world.spawn((Transform::default(), RigidBody::DYNAMIC, Children(vec![e1])));

        assert_eq!(
            collect_collider_children(root, &world),
            [
                (e0, Transform::from_translation(Vec3::splat(3.0))),
                (e1, Transform::from_translation(Vec3::splat(1.0)))
            ]
            .into_iter()
            .collect()
        );
    }
}
