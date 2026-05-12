use openfo4_core::ecs::Entity;

/// Event sent when an actor reaches an damaged value of `0.0`.
#[derive(Copy, Clone, Debug)]
pub struct ReachedZeroHealthEvent {
    /// The entity that has reached zero health.
    pub actor: Entity,
    /// The attacker that dealth the final damage.
    pub aggressor: Option<Entity>,
}

#[derive(Copy, Clone, Debug)]
pub struct BleedoutStartEvent {
    pub actor: Entity,
}
