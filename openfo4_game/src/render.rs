use std::time::Instant;

use openfo4_common::math::Ray;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_core_pipeline::entities::{MaterialHandle, MeshHandle};

#[derive(Clone, Debug)]
pub struct RenderObject {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
}

/// Entities with this component are despawned when the time is reached.
#[derive(Copy, Clone, Debug)]
pub struct DespawnAt(pub Instant);

#[derive(Copy, Clone, Debug)]
pub struct HitscanRaycast {
    pub ray: Ray,
    pub max_toi: f32,
    pub impact_toi: Option<f32>,
}

pub(crate) fn despawn_expired(world: &mut World) {
    let mut commands = CommandBuffer::new();

    let now = Instant::now();
    for (entity, expires) in world.query::<(Entity, &DespawnAt)>().iter() {
        if now >= expires.0 {
            commands.despawn(entity);
        }
    }

    commands.run_on(world);
}
