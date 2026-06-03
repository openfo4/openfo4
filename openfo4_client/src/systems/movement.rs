use std::f32::consts::PI;

use game_common::components::Transform;
use glam::Vec3;
use openfo4_core::Frozen;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_physics::PhysicsPipeline;
use openfo4_physics::controller::CharacterController;

use crate::assets::nif::NIF_TO_METERS;
use crate::components::{Grounded, Player};
use crate::state::UPS;
use crate::state::hardcoded::KEYWORD_FEMALE;
use crate::state::npc::ObjectKeywords;
use crate::systems::FacingDirection;
use crate::systems::animation_sync::AnimationParameters;
use crate::systems::controller::ControllerShape;

#[derive(Copy, Clone, Debug, Default)]
pub struct MovementSpeed {
    pub speed: f32,
    pub sprinting: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct MovementDirection(Vec3);

impl MovementDirection {
    pub const ZERO: Self = Self(Vec3::ZERO);

    pub fn new_normalized(dir: Vec3) -> Self {
        debug_assert!(dir.is_normalized() || dir == Vec3::ZERO);
        Self(dir)
    }
}

pub fn apply_movement(world: &mut World, pipeline: &mut PhysicsPipeline) {
    let mut buffer = CommandBuffer::new();

    for (
        entity,
        mut transform,
        facing_dir,
        movement_dir,
        speed,
        mut controller,
        collider,
        actor_keywords,
        player,
        frozen,
    ) in world.query_mut::<(
        Entity,
        &mut Transform,
        &FacingDirection,
        &MovementDirection,
        &MovementSpeed,
        Option<&mut CharacterController>,
        &ControllerShape,
        &ObjectKeywords,
        Option<&Player>,
        Option<&Frozen>,
    )>() {
        let direction = movement_dir.0;

        // Remove the Y component from the facing direction since
        // we don't want to move up.
        let mut translation = facing_dir.0 * direction;
        translation.y = 0.0;
        translation = translation.normalize_or_zero();

        // Speed is in units per second.
        // We are on a fixed timestep of 60 updates per second.
        translation *= speed.speed * NIF_TO_METERS / UPS as f32;

        if let Some(controller) = &mut controller {
            if frozen.as_ref().is_none_or(|v| !v.controls) {
                let movement = controller.move_shape(
                    pipeline,
                    *transform * collider.offset,
                    &collider.shape,
                    translation,
                    entity,
                );

                transform.translation += movement.translation;
            }

            if frozen.as_ref().is_none_or(|v| !v.physics) {
                let gravity_movement = controller.apply_gravity(
                    pipeline,
                    *transform * collider.offset,
                    &collider.shape,
                    entity,
                );

                if gravity_movement.grounded {
                    buffer.insert_one(entity, Grounded);
                } else {
                    buffer.remove_one::<Grounded>(entity);
                }

                transform.translation += gravity_movement.translation;
            }
        } else {
            transform.translation += translation;
        }

        // Compute angle between forward (-Z) vector and the movement vector.
        // 0.0 is front, 0.25 is right, 0.5 is back, 0.75 is left.
        let dir_norm = direction.normalize_or_zero();
        let mut dir = (-dir_norm.z).acos();
        // acos will yield PI/2 for a zero vector, but we want to face forward
        // when we don't move.
        // The length is either zero or normalized, so this will multiply with
        // either 0 or 1.
        dir *= dir_norm.length_squared();

        if (dir_norm.x).is_sign_negative() {
            dir = PI + (PI - dir);
        }

        dir /= 2.0 * PI;

        // Both rotation are world-space, but we want entity-space, so
        // we must remove the entity rotation.
        // The result is in the NIF coordinate system.
        let facing_direction = (transform.rotation.conjugate() * facing_dir.0) * -Vec3::Z;

        buffer.insert_one(
            entity,
            AnimationParameters {
                speed: speed.speed,
                direction: dir,
                is_moving: direction != Vec3::ZERO,
                is_sprinting: speed.sprinting,
                is_player: player.is_some(),
                is_female: actor_keywords.contains(KEYWORD_FEMALE),
                // is_jumping: state.jumping,
                is_jumping: false,
                facing_direction,
            },
        );
    }

    buffer.run_on(world);
}
