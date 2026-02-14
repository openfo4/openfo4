use hecs::Entity;

/// An entity has entered a trigger zone.
#[derive(Copy, Clone, Debug)]
pub struct TriggerEnter {
    /// The trigger zone that was entered.
    pub zone: Entity,
    /// The entity that has entered the zone.
    pub entity: Entity,
}

/// An entity has left a trigger zone.
#[derive(Copy, Clone, Debug)]
pub struct TriggerLeave {
    /// The trigger zone that was left.
    pub zone: Entity,
    /// The entity that has left the zone.
    pub entity: Entity,
}
