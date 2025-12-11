use game_common::components::Transform;
use hecs::World;
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_esp::FormId;

use crate::components::{NiTransform, WorldSpace};
use crate::systems::ActivationEvent;

#[derive(Copy, Clone, Debug)]
pub enum Door {
    Portal(PortalDoor),
}

#[derive(Copy, Clone, Debug)]
pub struct PortalDoor {
    /// The other door.
    pub target: FormId,
    pub target_position: NiTransform,
}

pub fn handle_door_activations(
    world: &mut World,
    events: &mut EventBus,
    object_ref_set: &ObjectReferenceSet,
) {
    for event in events.read::<ActivationEvent>() {
        dbg!(&event);
        if !world.contains(event.actor) {
            continue;
        }

        let Ok(door) = world.get::<&Door>(event.target).map(|v| *v) else {
            continue;
        };

        dbg!(&door);

        match door {
            Door::Portal(door) => {
                let Some(target) = object_ref_set.get_by_form(door.target) else {
                    tracing::warn!("unconnected portal door: {:?}", door.target);
                    continue;
                };

                let transform = Transform::from(door.target_position);
                let worldspace = *world.get::<&WorldSpace>(target).unwrap();

                world.insert(event.actor, (transform, worldspace)).unwrap();

                let mut queue = vec![event.actor];
                while let Some(e) = queue.pop() {
                    world.insert_one(e, worldspace).unwrap();
                    if let Ok(children) = world.get::<&Children>(e) {
                        queue.extend(&children.0);
                    }
                }
            }
        }
    }
}
