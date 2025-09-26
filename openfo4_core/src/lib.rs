use hecs::{Entity, World};

use crate::hierarchy::Children;

pub mod events;
pub mod hierarchy;
pub mod path;
pub mod transform;

pub trait WorldExt {
    fn add_children(&mut self, parent: Entity, children: Entity);
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
}
