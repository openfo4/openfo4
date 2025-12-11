use game_common::collections::bimap::BiMap;
use hecs::Entity;
use openfo4_esp::FormId;

#[derive(Clone, Debug, Default)]
pub struct ObjectReferenceSet(BiMap<FormId, Entity>);

impl ObjectReferenceSet {
    pub fn insert(&mut self, id: FormId, entity: Entity) {
        self.0.insert(id, entity);
    }

    pub fn get_by_form(&self, id: FormId) -> Option<Entity> {
        self.0.get_left(&id).copied()
    }

    pub fn get_by_entity(&self, entity: Entity) -> Option<FormId> {
        self.0.get_right(&entity).copied()
    }
}
