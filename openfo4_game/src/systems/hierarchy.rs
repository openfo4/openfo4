use std::collections::VecDeque;

use hashbrown::{HashMap, HashSet};
use nohash_hasher::BuildNoHashHasher;
use openfo4_common::components::{GlobalTransform, Transform};
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_core::hierarchy::Children;
use openfo4_tracing::trace_span;

#[derive(Clone, Debug, Default)]
pub struct TransformHierarchyCache {
    parents: HashMap<Entity, Entity, BuildNoHashHasher<Entity>>,
    children:
        HashMap<Entity, HashSet<Entity, BuildNoHashHasher<Entity>>, BuildNoHashHasher<Entity>>,
    /// Contains the computed global transform values.
    ///
    /// The computed global transforms are defined by the chain of transforms. We cannot use the
    /// [`GlobalTransform`] component to extract the current computed transforms, they may have
    /// been overwritten by other systems.
    transforms: HashMap<Entity, Transform, BuildNoHashHasher<Entity>>,
}

pub fn compute_global_transform(world: &mut World, cache: &mut TransformHierarchyCache) {
    let _span = trace_span!("compute_global_transform").entered();

    let mut recompute_transform_entities = HashSet::new();

    // Removing the Children component causes all children to become
    // "deatched" from the previous parent and their transforms will change.
    for entity in world.component_removed::<Children>() {
        let Some(children) = cache.children.remove(&entity) else {
            continue;
        };

        for child in children {
            cache.parents.remove(&child);
            recompute_transform_entities.insert(child);
        }
    }

    let view = world.view::<(&Transform, Option<&Children>)>();

    for entity in world
        .component_changed::<Children>()
        .chain(world.component_added::<Children>())
    {
        let Some((_, Some(children))) = view.get(entity) else {
            continue;
        };

        let mut prev_children = cache.children.remove(&entity).unwrap_or_default();

        for child in &children.0 {
            // Children is new.
            if !prev_children.remove(child) {
                cache.parents.insert(*child, entity);

                recompute_transform_entities.insert(*child);
            }
        }

        // Children is no longer present.
        for child in prev_children {
            cache.parents.remove(&child);
            recompute_transform_entities.insert(child);
        }

        cache
            .children
            .insert(entity, children.0.iter().copied().collect());
    }

    for entity in world
        .component_added::<Transform>()
        .chain(world.component_changed::<Transform>())
    {
        recompute_transform_entities.insert(entity);
    }

    for entity in world.component_removed::<Transform>() {
        cache.transforms.remove(&entity);
        recompute_transform_entities.remove(&entity);
    }

    let mut queue = VecDeque::new();
    let mut q = VecDeque::new();
    'out: for &entity in &recompute_transform_entities {
        // The parent will already recursively queue children.
        // We skip all non-root entities here (of our "recompute transform" set)
        // and let each root entity queue all children.
        // This allows us to construct the queue ordered (i.e. parents before children).
        let mut next = entity;
        while let Some(parent) = cache.parents.get(&next) {
            if recompute_transform_entities.contains(parent) {
                continue 'out;
            }

            next = *parent;
        }

        q.clear();

        q.push_back(entity);
        while let Some(entity) = q.pop_front() {
            queue.push_back(entity);

            let Some((transform, children)) = view.get(entity) else {
                continue;
            };

            cache.transforms.insert(entity, *transform);

            if let Some(children) = children {
                q.extend(&children.0);
            }
        }
    }

    if cfg!(debug_assertions) {
        validate_transform_cache_hierarchy(world, cache);
    }

    let mut new_transforms = Vec::new();
    while let Some(entity) = queue.pop_front() {
        let transform = cache.transforms.get(&entity).unwrap();

        let Some(parent) = cache.parents.get(&entity) else {
            new_transforms.push(entity);
            continue;
        };

        let parent_transform = cache.transforms.get(parent).unwrap();
        let local_transform = *parent_transform * *transform;

        cache.transforms.insert(entity, local_transform);
        new_transforms.push(entity);
    }

    drop(view);

    let mut view = world.view::<&mut GlobalTransform>();
    let mut buffer = CommandBuffer::new();

    for entity in new_transforms {
        let transform = cache.transforms[&entity];

        // Instead of just calling `world.insert(..)` for each entity we attempt
        // to update the existing `GlobalTransform` value. `world.insert(..)`
        // will reshuffle archetypes which is costly, event if the entity already
        // containts `GlobalTransform`.

        if let Some(mut dst) = view.get_mut(entity) {
            dst.0 = transform;
            continue;
        }

        buffer.insert_one(entity, GlobalTransform(transform));
    }

    drop(view);
    buffer.run_on(world);
}

fn validate_transform_cache_hierarchy(world: &World, cache: &TransformHierarchyCache) {
    let _span = trace_span!("validate_transform_cache_hierarchy").entered();

    for (entity, _, children) in world
        .query::<(Entity, &Transform, Option<&Children>)>()
        .iter()
    {
        assert!(cache.transforms.contains_key(&entity));

        if let Some(children) = &children {
            for child in &children.0 {
                assert_eq!(cache.parents.get(child).copied(), Some(entity));
            }

            let children = children.0.iter().copied().collect();
            assert_eq!(cache.children.get(&entity).cloned(), Some(children));
        } else {
            assert!(!cache.children.contains_key(&entity));
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;
    use openfo4_common::components::{GlobalTransform, Transform};
    use openfo4_core::ecs::World;
    use openfo4_core::hierarchy::Children;

    use crate::systems::hierarchy::TransformHierarchyCache;

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

        let mut cache = TransformHierarchyCache::default();
        compute_global_transform(&mut world, &mut cache);

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

        let mut cache = TransformHierarchyCache::default();
        compute_global_transform(&mut world, &mut cache);

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

    #[test]
    fn test_compute_global_transform_insert_children() {
        let mut world = World::new();

        let transform = Transform::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let child0 = world.spawn_one(transform);
        let root = world.spawn((transform, Children(vec![child0])));

        let mut cache = TransformHierarchyCache::default();
        compute_global_transform(&mut world, &mut cache);

        for (index, entity) in [root, child0].into_iter().enumerate() {
            assert_eq!(
                *world.get::<&GlobalTransform>(entity).unwrap(),
                GlobalTransform(Transform::from_translation(Vec3::new(
                    (index + 1) as f32,
                    0.0,
                    0.0
                )))
            );
        }

        world.clear_trackers();

        let child1 = world.spawn_one(transform);
        world.insert_one(child0, Children(vec![child1])).unwrap();

        compute_global_transform(&mut world, &mut cache);

        for (index, entity) in [root, child0, child1].into_iter().enumerate() {
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
