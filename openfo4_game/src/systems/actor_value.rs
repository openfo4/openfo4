use hashbrown::HashMap;
use openfo4_core::record::RecordId;

#[derive(Clone, Debug, Default)]
pub struct ActorValueProperties(HashMap<RecordId, ActorValue>);

impl ActorValueProperties {
    pub fn get(&self, id: RecordId) -> Option<ActorValue> {
        self.0.get(&id).copied()
    }

    pub fn set(&mut self, id: RecordId, value: ActorValue) {
        self.0.insert(id, value);
    }

    /// Removes all stored [`ActorValue`]s.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct ActorValue {
    /// The value set by the game.
    pub value: f32,
    /// The value after applying damage (via `DamageAV`).
    ///
    /// This is value is never greater than `value`.
    pub damaged: f32,
}

impl ActorValue {
    pub const fn new(value: f32) -> Self {
        Self {
            value,
            damaged: value,
        }
    }

    pub fn damaged(&self) -> f32 {
        self.damaged
    }

    /// Returns the damage count that has been applied.
    ///
    /// Not to be confused with the current [`damaged`] value.
    ///
    /// [`damaged`]: Self::damaged
    pub fn damage(&self) -> f32 {
        debug_assert!(self.value >= self.damaged);
        self.value - self.damaged
    }
}
