use game_common::components::Transform;
use game_tracing::trace_span;
use glam::Vec3;
use hecs::Entity;
use rapier3d::control::{CharacterLength, KinematicCharacterController};
use rapier3d::math::Isometry;
use rapier3d::parry::query::DefaultQueryDispatcher;
use rapier3d::prelude::QueryFilter;

use crate::components::ColliderShape;
use crate::convert::{rotation, vec3, vector};
use crate::{GRAVITY, PhysicsPipeline, build_shape};

#[derive(Clone, Debug)]
pub struct CharacterController {
    controller: KinematicCharacterController,
}

impl CharacterController {
    pub fn new() -> Self {
        Self {
            controller: KinematicCharacterController {
                offset: CharacterLength::Relative(0.01),
                ..Default::default()
            },
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

        let position = Isometry {
            translation: vector(transform.translation).into(),
            rotation: rotation(transform.rotation),
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
            vector(direction),
            |event| events.push(event),
        );

        self.controller.solve_character_collision_impulses(
            pipeline.integration_parameters.dt,
            &mut query_pl,
            &*shape,
            1.0,
            &events,
        );

        Movement {
            translation: vec3(movement.translation),
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
        self.move_shape(
            pipeline,
            transform,
            shape,
            vec3(GRAVITY * pipeline.integration_parameters.dt),
            exclude_body,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Movement {
    pub translation: Vec3,
    pub grounded: bool,
}
