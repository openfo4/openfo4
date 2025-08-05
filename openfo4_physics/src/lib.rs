pub mod components;

use std::cell::Cell;
use std::collections::{HashMap, VecDeque};

use components::{ColliderShape, RigidBody, RigidBodyKind};
use game_common::collections::bimap::BiMap;
use game_common::components::{GlobalTransform, Transform};
use glam::{Quat, Vec3};
use hecs::{Entity, World};
use openfo4_core::hierarchy::Children;
use rapier3d::math::{Isometry, Rotation};
use rapier3d::prelude::{
    CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, DefaultBroadPhase, ImpulseJointSet,
    IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, RigidBodyBuilder,
    RigidBodyHandle, RigidBodySet, RigidBodyType, SharedShape, Vector,
};
use tracing::instrument;

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
            body_children: HashMap::new(),
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

    #[instrument(name = "PhysicsPipeline::update_rigid_bodies", skip_all)]
    fn update_rigid_bodies(&mut self, world: &World) {
        let mut despawned_bodies = self.body_handles.clone();

        for (entity, (global_transform, rigid_body)) in
            world.query::<(&GlobalTransform, &RigidBody)>().iter()
        {
            let translation = Vector::from(global_transform.0.translation);
            let rotation = Rotation::from(global_transform.0.rotation);

            let kind = match rigid_body.kind {
                RigidBodyKind::Fixed => RigidBodyType::Fixed,
                RigidBodyKind::Dynamic => RigidBodyType::Dynamic,
            };
            let linvel = Vector::from(rigid_body.linvel);
            let angvel = Vector::from(rigid_body.angvel);

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

    #[instrument(name = "PhysicsPipeline::update_colliders", skip_all)]
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
                        ColliderBuilder::new(build_shape(&collider.shape, transform.scale));
                    let handle = self.colliders.insert(builder);

                    SpawnedCollider {
                        handle,
                        scale: transform.scale,
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

                state.set_position_wrt_parent(Isometry {
                    translation: Vector::from(translation).into(),
                    rotation: rotation.into(),
                });

                state.set_friction(collider.friction);
                state.set_restitution(collider.restitution);

                self.colliders.set_parent(
                    spawned_collider.handle,
                    Some(collider_parent.body),
                    &mut self.bodies,
                );
            }
        }

        for (entity, collider) in despawned_colliders {
            self.collider_handles.remove(&entity);
            self.colliders
                .remove(collider.handle, &mut self.islands, &mut self.bodies, true);
        }
    }

    fn write_back(&mut self, world: &mut World) {
        for (handle, body) in self.bodies.iter() {
            let entity = *self.body_handles.get_right(&handle).unwrap();

            let mut rigid_body = world.get::<&mut RigidBody>(entity).unwrap();
            rigid_body.linvel = Vec3::from(*body.linvel());
            rigid_body.angvel = Vec3::from(*body.angvel());

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

            let new_translation = Vec3::from(*body.translation());
            let new_rotation = Quat::from(*body.rotation());

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
        ColliderShape::TriMesh(mesh) => {
            let vertices = mesh
                .vertices()
                .iter()
                .map(|vertex| {
                    let point = *vertex * scale;
                    Vector::from(point).into()
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
                SharedShape::convex_hull(&vertices).unwrap()
            }
        }
    }
}
