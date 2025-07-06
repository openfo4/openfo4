use std::collections::{HashMap, VecDeque};

use game_common::components::{GlobalTransform, Transform};
use hecs::World;
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;

#[tracing::instrument(skip_all)]
pub fn compute_global_transform(world: &mut World) {
    let mut transforms = HashMap::new();
    let mut parents = HashMap::new();

    for (entity, (transform, children)) in world.query_mut::<(&Transform, Option<&Children>)>() {
        transforms.insert(entity, *transform);

        if let Some(children) = children {
            for children in &children.0 {
                parents.insert(*children, entity);
            }
        }
    }

    // Queue all entities that have no parent entity.
    let mut queue = VecDeque::new();
    for entity in transforms.keys() {
        if !parents.contains_key(entity) {
            queue.push_back(*entity);
        }
    }

    while let Some(entity) = queue.pop_front() {
        if let Some(parent) = parents.get(&entity) {
            let parent_transform = *transforms.get(parent).unwrap();
            let self_transform = *transforms.get(&entity).unwrap();

            let mut transform = parent_transform * self_transform;
            transform.rotation = transform.rotation.normalize();
            transforms.insert(entity, transform);
        }

        // We have computed the transform of the entity and
        // can compute the transforms of all children.
        if let Ok(children) = world.get::<&Children>(entity) {
            for children in &children.0 {
                queue.push_back(*children);
            }
        }
    }

    for (entity, transform) in transforms {
        // We know the entity is valid, so this unwrap cannot panic.
        world
            .insert_one(entity, GlobalTransform(transform))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use game_common::components::{GlobalTransform, Transform};
    use glam::Vec3;
    use hecs::World;
    use openfo4_core::hierarchy::Children;

    use super::compute_global_transform;

    #[test]
    fn test_compute_global_transform_basic() {
        let mut world = World::new();
        let root = world.spawn((Transform {
            translation: Vec3::new(1.0, 0.0, 0.0),
            ..Default::default()
        },));

        let child = world.spawn((Transform {
            translation: Vec3::new(2.0, 0.0, 0.0),
            ..Default::default()
        },));

        world.insert_one(root, Children(vec![child])).unwrap();

        compute_global_transform(&mut world);

        assert_eq!(
            *world.get::<&GlobalTransform>(root).unwrap(),
            GlobalTransform(Transform {
                translation: Vec3::new(1.0, 0.0, 0.0),
                ..Default::default()
            })
        );
        assert_eq!(
            *world.get::<&GlobalTransform>(child).unwrap(),
            GlobalTransform(Transform {
                translation: Vec3::new(3.0, 0.0, 0.0),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_compute_global_transform_deep() {
        let mut world = World::new();

        let transform = Transform::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let child0 = world.spawn((transform,));
        let child1 = world.spawn((transform, Children(vec![child0])));
        let child2 = world.spawn((transform, Children(vec![child1])));
        let child3 = world.spawn((transform, Children(vec![child2])));
        let root = world.spawn((transform, Children(vec![child3])));

        compute_global_transform(&mut world);

        for (index, entity) in [root, child3, child2, child1, child0]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                *world.get::<&GlobalTransform>(entity).unwrap(),
                GlobalTransform(Transform::from_translation(Vec3::new(
                    (index + 1) as f32,
                    0.0,
                    0.0
                )))
            );
        }
    }
}
