use hecs::Entity;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Children(pub Vec<Entity>);
