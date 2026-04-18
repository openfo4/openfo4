use game_common::collections::bimap::BiMap;

use crate::ecs::Entity;
use crate::record::RecordId;

#[derive(Clone, Debug, Default)]
pub struct ObjectReferenceSet(BiMap<RecordId, Entity>);

impl ObjectReferenceSet {
    pub fn insert(&mut self, id: RecordId, entity: Entity) {
        self.0.insert(id, entity);
    }

    pub fn get_by_form(&self, id: RecordId) -> Option<Entity> {
        self.0.get_left(&id).copied()
    }

    pub fn get_by_entity(&self, entity: Entity) -> Option<RecordId> {
        self.0.get_right(&entity).copied()
    }
}
