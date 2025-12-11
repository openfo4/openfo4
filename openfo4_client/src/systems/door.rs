use game_common::components::Transform;
use game_tracing::trace_span;
use hecs::World;
use openfo4_core::WorldExt;
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_esp::FormId;

use crate::components::{ActorMarker, NiTransform, WorldSpace};
use crate::systems::ActivationEvent;

/// The radius at which the autoload door is activated.
// FIXME: This looks to work roughly, but need to fit this to
// the radius seen in CK.
pub const AUTOLOAD_DOOR_RADIUS: f32 = 3.0;

#[derive(Copy, Clone, Debug)]
pub enum Door {
    Portal(PortalDoor),
    AutoPortal(PortalDoor),
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
    let _span = trace_span!("handle_door_activations").entered();

    for event in events.read::<ActivationEvent>() {
        if !world.contains(event.actor) {
            continue;
        }

        let Ok(door) = world.get::<&Door>(event.target).map(|v| *v) else {
            continue;
        };

        match door {
            Door::Portal(door) => {
                let Some(target) = object_ref_set.get_by_form(door.target) else {
                    tracing::warn!("unconnected portal door: {:?}", door.target);
                    continue;
                };

                let transform = Transform::from(door.target_position);
                let worldspace = *world.get::<&WorldSpace>(target).unwrap();

                world.insert(event.actor, (transform, worldspace)).unwrap();

                for e in world.hierarchy_recursive(event.actor) {
                    world.insert_one(e, worldspace).unwrap();
                }
            }
            Door::AutoPortal(_) => {}
        }
    }
}

pub fn handle_automatic_doors(world: &mut World, object_ref_set: &ObjectReferenceSet) {
    let _span = trace_span!("handle_automatic_doors").entered();

    let mut transitions = Vec::new();

    for (_, (door_transform, door)) in world.query::<(&Transform, &Door)>().iter() {
        let Door::AutoPortal(door) = door else {
            continue;
        };

        for (actor, (actor_transform, _)) in world.query::<(&Transform, &ActorMarker)>().iter() {
            let distance = door_transform
                .translation
                .distance(actor_transform.translation);
            if distance >= AUTOLOAD_DOOR_RADIUS {
                continue;
            }

            transitions.push((*door, actor));
        }
    }

    for (door, actor) in transitions {
        let Some(target) = object_ref_set.get_by_form(door.target) else {
            tracing::warn!("unconnected portal door: {:?}", door.target);
            continue;
        };

        let transform = Transform::from(door.target_position);
        let worldspace = *world.get::<&WorldSpace>(target).unwrap();

        world.insert(actor, (transform, worldspace)).unwrap();

        for e in world.hierarchy_recursive(actor) {
            world.insert_one(e, worldspace).unwrap();
        }
    }
}
