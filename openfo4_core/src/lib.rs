use crate::ecs::{Entity, World};
use crate::hierarchy::Children;

pub mod ecs;
pub mod events;
pub mod hierarchy;
pub mod logger;
pub mod path;
pub mod record;
pub mod transform;

pub trait WorldExt {
    fn add_children(&mut self, parent: Entity, children: Entity);

    fn despawn_recursive(&mut self, entity: Entity);

    /// Returns all entities in the hierarchy starting with `entity`.
    ///
    /// This includes the root, `entity`.
    fn hierarchy_recursive(&self, entity: Entity) -> Vec<Entity>;
}

impl WorldExt for World {
    fn add_children(&mut self, parent: Entity, children: Entity) {
        let entity = self.entity(parent).unwrap();

        if let Some(mut c) = entity.get::<&mut Children>() {
            c.0.push(children);
        } else {
            self.insert_one(parent, Children(vec![children])).unwrap();
        }
    }

    fn despawn_recursive(&mut self, entity: Entity) {
        // Fast path if the entity doesn't exist
        // that doesn't allocate.
        if !self.contains(entity) {
            return;
        }

        let mut queue = vec![entity];
        while let Some(e) = queue.pop() {
            if let Ok(children) = self.get::<&Children>(e) {
                queue.extend(&children.0);
            }

            self.despawn(e).ok();
        }
    }

    fn hierarchy_recursive(&self, entity: Entity) -> Vec<Entity> {
        let mut entities = Vec::new();
        let mut queue = vec![entity];
        while let Some(e) = queue.pop() {
            entities.push(e);

            if let Ok(children) = self.get::<&Children>(e) {
                queue.extend(&children.0);
            }
        }

        entities
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Frozen {
    /// Physics interactions are frozen.
    pub physics: bool,
    /// Controls/Inputs are frozen.
    pub controls: bool,
}

impl Frozen {
    pub const FROZEN: Frozen = Frozen {
        physics: true,
        controls: true,
    };

    /// Whether any behavior is frozen.
    pub fn is_any_frozen(&self) -> bool {
        let Self { physics, controls } = self;
        *physics || *controls
    }

    /// Whether all behavior is frozen.
    pub fn is_all_frozen(&self) -> bool {
        let Self { physics, controls } = self;
        *physics && *controls
    }
}
