use openfo4_common::components::Transform;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_tracing::trace_span;

use crate::components::{Root, TriggerBox};
use crate::events::trigger::{TriggerEnter, TriggerLeave};

pub fn update_trigger_box_contacts(world: &mut World, events: &mut EventBus) {
    let _span = trace_span!("update_trigger_box_contacts").entered();

    for (zone, tbox_transform, mut tbox) in world
        .query::<(Entity, &Transform, &mut TriggerBox)>()
        .iter()
    {
        for (entity, _, transform) in world.query::<(Entity, &Root, &Transform)>().iter() {
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
