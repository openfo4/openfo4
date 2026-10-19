use glam::Vec3;
use openfo4_common::components::Transform;
use openfo4_core::ecs::Entity;
use openfo4_tracing::trace_span;
use rapier3d::math::{Pose, Vector};
use rapier3d::parry::query::DefaultQueryDispatcher;
use rapier3d::prelude::QueryFilter;

use crate::components::ColliderShape;
use crate::rapier_controller::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use crate::{GRAVITY, PhysicsPipeline, build_shape};

#[derive(Clone, Debug)]
pub struct CharacterController {
    controller: KinematicCharacterController,
    gravity_velcotiy: Vec3,
}

impl CharacterController {
    pub fn new() -> Self {
        Self {
            controller: KinematicCharacterController {
                offset: CharacterLength::Relative(0.01),
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Relative(0.25),
                    min_width: CharacterLength::Relative(0.1),
                    include_dynamic_bodies: false,
                }),
                ..Default::default()
            },
            gravity_velcotiy: Vector::default(),
        }
    }

    #[must_use]
    pub fn move_shape(
        &mut self,
        pipeline: &mut PhysicsPipeline,
        transform: Transform,
        shape: &ColliderShape,
        direction: Vec3,
        exclude_body: Entity,
    ) -> Movement {
        let _span = trace_span!("CharacterController::move_shape").entered();

        let shape = build_shape(shape, transform.scale);

        let position = Pose {
            translation: transform.translation,
            rotation: transform.rotation,
            padding: 0,
        };

        let mut events = Vec::new();

        let mut filter = QueryFilter::new();
        if let Some(handle) = pipeline.body_handles.get_left(&exclude_body) {
            filter.exclude_rigid_body = Some(*handle);
        }

        let mut query_pl = pipeline.broad_phase.as_query_pipeline_mut(
            &DefaultQueryDispatcher,
            &mut pipeline.bodies,
            &mut pipeline.colliders,
            filter,
        );

        let movement = self.controller.move_shape(
            pipeline.integration_parameters.dt,
            &query_pl.as_ref(),
            &*shape,
            &position,
            direction,
            |event| events.push(event),
        );

        self.controller.solve_character_collision_impulses(
            pipeline.integration_parameters.dt,
            &mut query_pl,
            &*shape,
            1.0,
            &events,
        );

        if movement.grounded {
            self.gravity_velcotiy = Vector::default();
        }

        Movement {
            translation: movement.translation,
            grounded: movement.grounded,
        }
    }

    pub fn apply_gravity(
        &mut self,
        pipeline: &mut PhysicsPipeline,
        transform: Transform,
        shape: &ColliderShape,
        exclude_body: Entity,
    ) -> Movement {
        self.gravity_velcotiy += GRAVITY * pipeline.integration_parameters.dt;
        let delta = self.gravity_velcotiy * pipeline.integration_parameters.dt;

        self.move_shape(pipeline, transform, shape, delta, exclude_body)
    }
}

#[derive(Clone, Debug)]
pub struct Movement {
    pub translation: Vec3,
    pub grounded: bool,
}
