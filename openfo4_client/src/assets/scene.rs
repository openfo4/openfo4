use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use game_common::components::{GlobalTransform, Transform};
use hecs::{Entity, World};
use openfo4_core::hierarchy::Children;
use openfo4_physics::components::Collider;

use crate::render::RenderObject;
use crate::state::asset::NifHandle;

static KNOWN_COMPONENTS: LazyLock<HashSet<TypeId>> = LazyLock::new(|| {
    [
        TypeId::of::<Transform>(),
        TypeId::of::<GlobalTransform>(),
        TypeId::of::<RenderObject>(),
        TypeId::of::<Children>(),
        TypeId::of::<NifHandle>(),
        TypeId::of::<Collider>(),
    ]
    .into_iter()
    .collect()
});

#[derive(Clone)]
pub struct Scene {
    world: Arc<World>,
}

impl Scene {
    pub fn new(world: World) -> Self {
        Self {
            world: Arc::new(world),
        }
    }

    pub fn instantiate(&self, world: &mut World, root: Entity) {
        let mut scene_to_world = HashMap::new();

        let mut non_root_entities = HashSet::new();

        for entity in self.world.iter() {
            let world_entity = world.reserve_entity();
            scene_to_world.insert(entity.entity(), world_entity);
        }

        for entity in self.world.iter() {
            let world_entity = *scene_to_world.get(&entity.entity()).unwrap();

            if cfg!(debug_assertions) {
                for id in entity.component_types() {
                    assert!(KNOWN_COMPONENTS.contains(&id));
                }
            }

            if let Some(children) = entity.get::<&Children>() {
                for child in &children.0 {
                    non_root_entities.insert(*child);
                }

                let children = children
                    .0
                    .iter()
                    .map(|e| *scene_to_world.get(e).unwrap())
                    .collect();
                world.insert_one(world_entity, Children(children)).unwrap();
            }

            if let Some(transform) = entity.get::<&Transform>() {
                world.insert_one(world_entity, *transform).unwrap();
            }

            if let Some(object) = entity.get::<&RenderObject>() {
                world.insert_one(world_entity, (*object).clone()).unwrap();
            }

            if let Some(handle) = entity.get::<&NifHandle>() {
                world.insert_one(world_entity, (*handle).clone()).unwrap();
            }

            if let Some(collider) = entity.get::<&Collider>() {
                world.insert_one(world_entity, (*collider).clone()).unwrap();
            }
        }

        let root_entities = scene_to_world
            .into_iter()
            .filter(|(scene_id, _)| !non_root_entities.contains(&scene_id))
            .map(|(_, world_id)| world_id)
            .collect();

        world.insert_one(root, Children(root_entities)).unwrap();
    }
}
