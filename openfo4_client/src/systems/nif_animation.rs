use std::time::Duration;

use game_core_pipeline::entities::Entities;
use game_tracing::trace_span;
use hecs::World;
use openfo4_core::events::{ComponentChanged, EventBus};

use crate::{
    assets::nif::{animation::ControlledValue, animation_controller::AnimationControllerInstance},
    render::RenderObject,
};

pub fn update_nif_animations(
    world: &mut World,
    events: &mut EventBus,
    entities: &Entities,
    real_time: Duration,
) {
    let _span = trace_span!("update_nif_animations").entered();

    let mut material_updates = Vec::new();

    for (_, controller) in world.query::<&AnimationControllerInstance>().iter() {
        for (node, params) in &controller.controller().material {
            let mut material = params.material.clone();

            // FIXME: Don't recreate/reupload the material when it doesn't change this frame.
            for value in &params.values {
                match value {
                    ControlledValue::UOffset(interpolator) => {
                        if let Some(value) = interpolator.sample(real_time) {
                            material.uv_offset.x = value;
                        }
                    }
                    ControlledValue::VOffset(interpolator) => {
                        if let Some(value) = interpolator.sample(real_time) {
                            material.uv_offset.y = value;
                        }
                    }
                    ControlledValue::UScale(interpolator) => {
                        if let Some(value) = interpolator.sample(real_time) {
                            material.uv_scale.x = value;
                        }
                    }
                    ControlledValue::VScale(interpolator) => {
                        if let Some(value) = interpolator.sample(real_time) {
                            material.uv_scale.y = value;
                        }
                    }
                    ControlledValue::Alpha(interpolator) => {
                        if let Some(value) = interpolator.sample(real_time) {
                            material.base_color.a = value;
                        }
                    }
                }
            }

            let entity = *controller.nodes().get(node).unwrap();

            material_updates.push((entity, material));
        }
    }

    for (entity, material) in material_updates {
        if let Ok(mut object) = world.get::<&mut RenderObject>(entity) {
            let material = entities.create_material(material);
            object.material = material;

            events.send(ComponentChanged::<RenderObject>::new(entity));
        }
    }
}
