use std::time::Duration;

use game_core_pipeline::entities::Entities;
use game_tracing::trace_span;
use openfo4_core::ecs::{CommandBuffer, World};
use openfo4_core::events::EventBus;

use crate::assets::nif::animation::ControlledValue;
use crate::assets::nif::animation_controller::{
    AnimationControllerInstance, SequencePlaybackState,
};
use crate::events::animation::StartNifAnimationSequence;
use crate::render::RenderObject;

pub fn update_nif_animations(
    world: &mut World,
    entities: &Entities,
    real_time: Duration,
    delta: Duration,
) {
    let _span = trace_span!("update_nif_animations").entered();

    let mut material_updates = Vec::new();
    let mut commands = CommandBuffer::new();

    for mut controller in world.query::<&mut AnimationControllerInstance>().iter() {
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

        let mut finished_sequences = Vec::new();

        for (name, state) in &controller.sequences {
            let Some(seq) = controller.controller().sequences.get(name) else {
                continue;
            };

            for (key, interpolator) in &seq.transforms {
                let Some(node) = controller.scene().nodes.get(*key) else {
                    continue;
                };

                let entity = controller.nodes().get(key).unwrap();

                let transform = interpolator.sample(state.time, node.transform);

                commands.insert_one(*entity, transform);
            }

            if let Some(duration) = seq.duration
                && state.time >= duration
            {
                finished_sequences.push(name.to_owned());
            }
        }

        for name in finished_sequences {
            controller.sequences.remove(&name);
        }

        for state in controller.sequences.values_mut() {
            state.time += delta;
        }
    }

    for (entity, material) in material_updates {
        if let Ok(mut object) = world.get::<&mut RenderObject>(entity) {
            let material = entities.create_material(material);
            object.material = material;
        }
    }

    commands.run_on(world);
}

pub fn start_nif_animation_sequences(world: &mut World, events: &EventBus) {
    for event in events.read::<StartNifAnimationSequence>() {
        let Ok(mut controller) = world.get::<&mut AnimationControllerInstance>(event.entity) else {
            continue;
        };

        if !controller.controller().sequences.contains_key(&*event.name) {
            tracing::warn!(
                "entity {:?} has no animation sequence named {:?} ",
                event.entity,
                event.name,
            );

            continue;
        }

        tracing::debug!(
            "start nif animation sequence {:?} on entity {:?}",
            event.name,
            event.entity
        );

        controller.sequences.insert(
            event.name.to_string(),
            SequencePlaybackState {
                time: Duration::ZERO,
            },
        );
    }
}
