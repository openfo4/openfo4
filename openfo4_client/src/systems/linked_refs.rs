use hashbrown::HashMap;
use openfo4_core::ecs::Entity;
use openfo4_core::record::RecordId;

#[derive(Clone, Debug, Default)]
pub struct LinkedRefs {
    refs: HashMap<Option<RecordId>, Entity>,
}

impl LinkedRefs {
    pub fn set(&mut self, keyword: Option<RecordId>, entity: Entity) {
        self.refs.insert(keyword, entity);
    }

    pub fn remove(&mut self, keyword: Option<RecordId>) {
        self.refs.remove(&keyword);
    }

    pub fn get(&self, keyword: Option<RecordId>) -> Option<Entity> {
        self.refs.get(&keyword).copied()
    }
}
