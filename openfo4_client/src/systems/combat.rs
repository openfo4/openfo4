use game_tracing::trace_span;
use openfo4_core::ecs::{CommandBuffer, World};
use openfo4_core::events::EventBus;

use crate::components::{ActorProperties, ActorProtection, DeadActor, Player};
use crate::events::combat::{BleedoutStartEvent, ReachedZeroHealthEvent};
use crate::systems::DeathStartEvent;

pub fn handle_zero_health_events(world: &mut World, events: &mut EventBus) {
    let _span = trace_span!("handle_zero_health_events").entered();

    let mut death_events = Vec::new();
    let mut bleedout_events = Vec::new();
    let mut commands = CommandBuffer::new();

    for event in events.read::<ReachedZeroHealthEvent>() {
        let Ok(props) = world.get::<&ActorProperties>(event.actor) else {
            continue;
        };

        let kill_actor = match props.protection {
            ActorProtection::None => true,
            // Kill actor only if the aggressor is the player.
            ActorProtection::Protected => event.aggressor.is_some_and(|aggressor| {
                world
                    .entity(aggressor)
                    .is_ok_and(|aggressor| aggressor.has::<Player>())
            }),
            ActorProtection::Essential => false,
        };

        if kill_actor {
            death_events.push(DeathStartEvent {
                actor: event.actor,
                killer: event.aggressor,
            });

            commands.insert_one(event.actor, DeadActor);
        } else {
            bleedout_events.push(BleedoutStartEvent { actor: event.actor });
        }
    }

    events.extend(death_events);
    events.extend(bleedout_events);

    commands.run_on(world);
}
