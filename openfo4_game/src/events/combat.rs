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

/// Event sent when the body part of an actor reaches zero health and becomes crippled.
#[derive(Copy, Clone, Debug)]
pub struct BodyPartCrippleEvent {
    /// The actor whose body part has become crippled.
    pub actor: Entity,
    /// The body part that has become crippled.
    pub part: u32,
}
