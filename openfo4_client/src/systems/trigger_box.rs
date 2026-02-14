use game_common::components::Transform;
use game_tracing::trace_span;
use hecs::World;
use openfo4_core::events::EventBus;

use crate::components::{Root, TriggerBox};
use crate::events::trigger::{TriggerEnter, TriggerLeave};

pub fn update_trigger_box_contacts(world: &mut World, events: &mut EventBus) {
    let _span = trace_span!("update_trigger_box_contacts").entered();

    for (zone, (tbox_transform, tbox)) in world.query::<(&Transform, &mut TriggerBox)>().iter() {
        for (entity, (_, transform)) in world.query::<(&Root, &Transform)>().iter() {
            let dist = (transform.translation - tbox_transform.translation).abs();
            let is_inside = dist.cmplt(tbox.half_extents.abs()).all();

            if is_inside && tbox.entities_inside.insert(entity) {
                events.send(TriggerEnter { zone, entity });
            // FIXME: What about entities that despawn here?
            // they are obviously not visited here.
            } else if !is_inside && tbox.entities_inside.remove(&entity) {
                events.send(TriggerLeave { zone, entity });
            }
        }
    }
}
