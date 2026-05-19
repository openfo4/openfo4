use game_common::components::Transform;
use game_tracing::trace_span;
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::record::RecordId;

use crate::components::{ActorMarker, NiTransform, WorldSpace};
use crate::events::animation::StartNifAnimationSequence;
use crate::systems::ActivationEvent;

/// The radius at which the autoload door is activated.
// FIXME: This looks to work roughly, but need to fit this to
// the radius seen in CK.
pub const AUTOLOAD_DOOR_RADIUS: f32 = 3.0;

const DOOR_OPEN_ANIM: &str = "Open";
const DOOR_CLOSE_ANIM: &str = "Close";

#[derive(Copy, Clone, Debug)]
pub enum Door {
    Normal(DoorState),
    Portal(PortalDoor),
    AutoPortal(PortalDoor),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DoorState {
    Open,
    Closed,
}

#[derive(Copy, Clone, Debug)]
pub struct PortalDoor {
    /// The other door.
    pub target: RecordId,
    pub target_position: NiTransform,
}

pub fn handle_door_activations(
    world: &mut World,
    events: &mut EventBus,
    object_ref_set: &ObjectReferenceSet,
) {
    let _span = trace_span!("handle_door_activations").entered();

    let mut anim_events = Vec::new();

    for event in events.read::<ActivationEvent>() {
        if !world.contains(event.actor) {
            continue;
        }

        let Ok(door) = world.get::<&mut Door>(event.target).map(|v| *v) else {
            continue;
        };

        match door {
            Door::Normal(DoorState::Open) => {
                anim_events.push(StartNifAnimationSequence {
                    entity: event.target,
                    name: DOOR_CLOSE_ANIM.into(),
                });

                // FIXME: This should happen when the animation is done.
                world
                    .insert_one(event.target, Door::Normal(DoorState::Closed))
                    .unwrap();
            }
            Door::Normal(DoorState::Closed) => {
                anim_events.push(StartNifAnimationSequence {
                    entity: event.target,
                    name: DOOR_OPEN_ANIM.into(),
                });

                // FIXME: This should happen when the animation is done.
                world
                    .insert_one(event.target, Door::Normal(DoorState::Open))
                    .unwrap();
            }
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

    events.extend(anim_events)
}

pub fn handle_automatic_doors(world: &mut World, object_ref_set: &ObjectReferenceSet) {
    let _span = trace_span!("handle_automatic_doors").entered();

    let mut transitions = Vec::new();

    for (_, door_transform, door, door_worldspace) in world
        .query::<(Entity, &Transform, &Door, &WorldSpace)>()
        .iter()
    {
        let Door::AutoPortal(door) = *door else {
            continue;
        };

        for (actor, actor_transform, _, actor_worldspace) in world
            .query::<(Entity, &Transform, &ActorMarker, &WorldSpace)>()
            .iter()
        {
            if *door_worldspace != *actor_worldspace {
                continue;
            }

            let distance = door_transform
                .translation
                .distance(actor_transform.translation);
            if distance >= AUTOLOAD_DOOR_RADIUS {
                continue;
            }

            transitions.push((door, actor));
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
