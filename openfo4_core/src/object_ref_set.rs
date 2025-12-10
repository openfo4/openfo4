use std::collections::HashMap;

use hecs::Entity;
use openfo4_esp::FormId;

#[derive(Clone, Debug, Default)]
pub struct ObjectReferenceSet(HashMap<FormId, Entity>);

impl ObjectReferenceSet {
    pub fn insert(&mut self, id: FormId, entity: Entity) {
        self.0.insert(id, entity);
    }

    pub fn get(&self, id: FormId) -> Option<Entity> {
        self.0.get(&id).copied()
    }
}
