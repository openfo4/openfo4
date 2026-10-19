pub mod components;
pub mod controller;
pub mod debug;

mod rapier_controller;

use std::cell::Cell;
use std::collections::VecDeque;

use components::{ColliderShape, RigidBody, RigidBodyKind};
use glam::Vec3;
use hashbrown::HashMap;
use openfo4_common::collections::bimap::BiMap;
use openfo4_common::components::{Axis, GlobalTransform, Transform};
use openfo4_common::math::Ray;
use openfo4_core::Frozen;
use openfo4_core::ecs::{Entity, Satisfies, ViewBorrow, World};
use openfo4_core::hierarchy::Children;
use openfo4_gizmos::Gizmos;
use openfo4_tracing::trace_span;
use rapier3d::math::Pose;
use rapier3d::parry::query::{DefaultQueryDispatcher, ShapeCastOptions};
use rapier3d::prelude::{
    CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, DebugRenderMode, DebugRenderPipeline,
    DebugRenderStyle, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager,
    MultibodyJointSet, NarrowPhase, QueryPipeline, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
    RigidBodyType, SharedShape,
};
use tracing::instrument;

use crate::components::{Collider, CollisionLayer};
use crate::debug::GizmoRenderBackend;

const DT: f32 = 1.0 / 60.0;
const MIN_CCD_DT: f32 = DT / 100.0;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

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
    rigid_bodies: HashMap<Entity, RigidBodyState>,
    body_handles: BiMap<Entity, RigidBodyHandle>,
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
            debug: DebugRenderPipeline::new(
                DebugRenderStyle::default(),
                DebugRenderMode::default(),
            ),
            rigid_bodies: HashMap::new(),
        }
    }

    #[instrument(name = "PhysicsPipeline::step", skip_all)]
    pub fn step(&mut self, world: &mut World) {
        self.update_rigid_bodies(world);
        self.update_colliders(world);

        self.pipeline.step(
            GRAVITY,
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

        let view = world.view::<(
            Satisfies<&RigidBody>,
            Satisfies<&Collider>,
            &Transform,
            Option<&Children>,
        )>();

        for (entity, global_transform, rigid_body, frozen) in world
            .query::<(Entity, &GlobalTransform, &RigidBody, Option<&Frozen>)>()
            .iter()
        {
            if frozen.is_some_and(|v| v.physics) {
                continue;
            }

            let translation = global_transform.0.translation;
            let rotation = global_transform.0.rotation;

            let kind = match rigid_body.kind {
                RigidBodyKind::Fixed => RigidBodyType::Fixed,
                RigidBodyKind::Dynamic => RigidBodyType::Dynamic,
            };
            let linvel = rigid_body.linvel;
            let angvel = rigid_body.angvel;

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

            let children = collect_collider_children(entity, &view);
            match self.rigid_bodies.get_mut(&entity) {
                Some(body) => {
                    if body.colliders != children {
                        body.colliders = children;
                        body.has_updated.set(true);
                    }
                }
                None => {
                    self.rigid_bodies.insert(
                        entity,
                        RigidBodyState {
                            handle: *handle,
                            has_updated: Cell::new(true),
                            colliders: children,
                        },
                    );
                }
            }

            despawned_bodies.remove_left(&entity);
        }

        for (entity, handle) in despawned_bodies {
            self.rigid_bodies.remove(&entity);
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

    fn update_colliders(&mut self, world: &World) {
        let _span = trace_span!("PhysicsPipeline::update_colliders").entered();

        let mut despawned_colliders = self.collider_handles.clone();

        for body in self.rigid_bodies.values() {
            for entity in body.colliders.keys() {
                despawned_colliders.remove(entity);
            }

            if !body.has_updated.replace(false) {
                continue;
            }

            for (&entity, &transform_rel_to_parent) in &body.colliders {
                let collider = world.get::<&Collider>(entity).unwrap();
                let transform = world.get::<&GlobalTransform>(entity).unwrap().0;

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
                let scale_to_parent = transform.scale / transform_rel_to_parent.scale;
                let translation = transform_rel_to_parent.translation * scale_to_parent;
                let rotation = transform_rel_to_parent.rotation;

                state.set_friction(collider.friction);
                state.set_restitution(collider.restitution);
                state.set_collision_groups(collider.layer.to_interaction_groups());

                self.colliders.set_parent(
                    spawned_collider.handle,
                    Some(body.handle),
                    &mut self.bodies,
                );

                // We can only set the translation AFTER setting the parent. Setting the
                // parent requires access to all colliders for some reason. (rapiers API
                // is probably just weird...).
                let state = self.colliders.get_mut(spawned_collider.handle).unwrap();
                state.set_position_wrt_parent(Pose {
                    translation,
                    rotation,
                    padding: 0,
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
        let mut view = world.view_mut::<(&mut RigidBody, &mut Transform, &mut GlobalTransform)>();

        for handle in self.islands.active_bodies() {
            let body = self.bodies.get(handle).unwrap();
            let entity = *self.body_handles.get_right(&handle).unwrap();

            let Some((mut rigid_body, mut transform, mut global_transform)) = view.get_mut(entity)
            else {
                continue;
            };

            rigid_body.linvel = body.linvel();
            rigid_body.angvel = body.angvel();

            // We source the position of the rigid body from the `GlobalTransform` component,
            // but we cannot write back directly to the `GlobalTransform` component since
            // it will be overwritten in the next frame.
            // Instead we must write to the `Transform` component, but it may differ from the
            // `GlobalTransform` since the rigid body may itself be a children of another entity.

            // To handle this correctly we compute the delta between the absolute `GlobalTransform`
            // (before the physics tick) and the new absolute position (after the physics tick).
            // The delta describes the changed transform in this physics tick and can be applied
            // to `Transform`.

            let new_translation = body.translation();
            let new_rotation = *body.rotation();

            let delta_translation = new_translation - global_transform.0.translation;
            // Note that `delta_rotation` is applied from left.
            let delta_rotation = new_rotation * global_transform.0.rotation.conjugate();

            transform.translation += delta_translation;
            transform.rotation = (delta_rotation * transform.rotation).normalize();

            // Keep `GlobalTransform` maintained.
            // While this is not strictly necessary because the caller can recompute this
            // after this call, this can be costly and we can already do it very cheaply here.
            global_transform.0.translation = new_translation;
            global_transform.0.rotation = new_rotation;
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
            origin: ray.origin,
            dir: ray.direction,
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
        let origin = Pose {
            translation: origin.translation,
            rotation: origin.rotation,
            padding: 0,
        };

        self.query_pipeline()
            .cast_shape(&origin, direction, &*shape, ShapeCastOptions::default())
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
struct RigidBodyState {
    /// The handle of the [`RigidBody`].
    handle: RigidBodyHandle,
    has_updated: Cell<bool>,
    /// Colliders attached to the rigid body.
    ///
    /// The [`Transform`] value of the collider is relative to this [`RigidBody`].
    colliders: HashMap<Entity, Transform>,
}

#[derive(Clone, Debug)]
struct SpawnedCollider {
    handle: ColliderHandle,
    scale: Vec3,
    layer: CollisionLayer,
}

/// Collects and returns all entities with collider components that are children of `entity`.
/// (This also includes `entity` itself).
///
/// The [`Transform`] value of an entity is the local offset from the root `entity`.
///
/// The branch is interrupted if an entity contains no transform, or if it contains a rigid body
/// component, signaling the start of a new rigid body.
fn collect_collider_children(
    root: Entity,
    view: &ViewBorrow<
        '_,
        (
            Satisfies<&RigidBody>,
            Satisfies<&Collider>,
            &Transform,
            Option<&Children>,
        ),
    >,
) -> HashMap<Entity, Transform> {
    let mut entities = HashMap::new();
    let mut queue = VecDeque::new();

    let Some((_, is_collider, _, children)) = view.get(root) else {
        return entities;
    };

    // The root entity can also be a collider.
    if is_collider {
        entities.insert(root, Transform::IDENTITY);
    }

    if let Some(children) = children {
        for &child in &children.0 {
            queue.push_back((child, Transform::IDENTITY));
        }
    }

    while let Some((entity, parent_transform)) = queue.pop_front() {
        let Some((is_rigid_body, is_collider, transform, children)) = view.get(entity) else {
            continue;
        };

        // If the child itself is a rigid body it should
        // not be considered a child of this rigid body
        // anymore and be detached.
        if is_rigid_body {
            continue;
        }

        let local_transform = parent_transform * *transform;

        if is_collider {
            entities.insert(entity, local_transform);
        }

        if let Some(children) = children {
            for &child in &children.0 {
                queue.push_back((child, local_transform));
            }
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
                .map(|vertex| *vertex * scale)
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
    use glam::Vec3;
    use openfo4_common::components::{Axis, Transform};
    use openfo4_core::ecs::{Satisfies, World};
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

        let view = world.view::<(
            Satisfies<&RigidBody>,
            Satisfies<&Collider>,
            &Transform,
            Option<&Children>,
        )>();

        assert_eq!(
            collect_collider_children(root, &view),
            [
                (e0, Transform::from_translation(Vec3::splat(3.0))),
                (e1, Transform::from_translation(Vec3::splat(1.0)))
            ]
            .into_iter()
            .collect()
        );
    }
}
