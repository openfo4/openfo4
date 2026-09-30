use std::sync::Arc;

use game_common::components::Transform;
use game_tasks::{TaskPool, TaskQueue};
use game_tracing::trace_span;
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::record::RecordId;

use crate::components::{ActorMarker, NiTransform, WorldSpace};
use crate::events::animation::StartNifAnimationSequence;
use crate::state::level::loader::{EntityLoader, EntityRef};
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

#[derive(Debug)]
pub struct DoorSystem {
    transitions: TaskQueue<Option<(Entity, EntityRef, NiTransform)>>,
    pool: Arc<TaskPool>,
    loader: Arc<EntityLoader>,
}

impl DoorSystem {
    pub fn new(pool: Arc<TaskPool>, loader: Arc<EntityLoader>) -> Self {
        Self {
            pool,
            loader,
            transitions: TaskQueue::new(),
        }
    }

    pub fn update(&mut self, world: &mut World, events: &mut EventBus) {
        self.handle_door_activations(world, events);
        self.handle_automatic_doors(world);

        while let Some(transition) = self.transitions.pop() {
            let Some((entity, target, target_pos)) = transition else {
                continue;
            };

            tracing::debug!("moving entity to target portal door {:?}", target.entity());

            let transform = Transform::from(target_pos);
            let worldspace = *world.get::<&WorldSpace>(target.entity()).unwrap();

            world.insert(entity, (transform, worldspace)).unwrap();

            for e in world.hierarchy_recursive(entity) {
                world.insert_one(e, worldspace).unwrap();
            }
        }
    }

    fn handle_door_activations(&mut self, world: &mut World, events: &mut EventBus) {
        let _span = trace_span!("DoorSystem::handle_door_activations").entered();

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
                    self.queue_transition(event.actor, door.target, door.target_position);
                }
                Door::AutoPortal(_) => {}
            }
        }
    }

    fn handle_automatic_doors(&mut self, world: &mut World) {
        let _span = trace_span!("DoorSystem::handle_automatic_doors").entered();

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

                self.queue_transition(actor, door.target, door.target_position);
            }
        }
    }

    fn queue_transition(&mut self, actor: Entity, target: RecordId, target_pos: NiTransform) {
        let loader = self.loader.clone();
        self.transitions.push(self.pool.spawn(async move {
            match loader.load(target).await {
                Ok(target) => Some((actor, target, target_pos)),
                Err(()) => {
                    tracing::error!("failed to load portal door target for target door {target}");
                    None
                }
            }
        }));
    }
}
