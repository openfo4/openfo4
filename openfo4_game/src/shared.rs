use openfo4_core_pipeline::entities::Entities;

#[derive(Debug)]
pub struct SharedState {
    pub entities: Entities,
}

impl SharedState {
    pub fn new(entities: Entities) -> Self {
        Self { entities }
    }
}
