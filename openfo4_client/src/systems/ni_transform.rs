use game_common::components::Transform;
use openfo4_core::ecs::{CommandBuffer, Entity, World};

use crate::cell::CellId;
use crate::components::{NiTransform, Root};

#[tracing::instrument(skip_all)]
pub fn compute_transform_from_ni_transform(world: &mut World) {
    let mut commands = CommandBuffer::new();
    for (entity, ni_transform) in world.query_mut::<(Entity, &NiTransform)>() {
        commands.insert_one(entity, Transform::from(*ni_transform));
    }
    commands.run_on(world);
}

#[tracing::instrument(skip_all)]
pub fn compute_cell_ids_from_ni_transform(world: &mut World) {
    let mut commands = CommandBuffer::new();
    for (entity, transform, _) in world.query_mut::<(Entity, &Transform, &Root)>() {
        let ni_transform = NiTransform::from(*transform);
        commands.insert_one(entity, CellId::from(ni_transform));
    }
    commands.run_on(world);
}
