use std::collections::VecDeque;

use game_common::components::Transform;
use game_core_pipeline::entities::ObjectHandle;
use game_tracing::trace_span;
use glam::Vec3;
use hashbrown::HashMap;
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::hierarchy::Children;

use crate::shared::SharedState;
use crate::systems::equipment::Equipment;

pub fn mesh_skinning(world: &World, objects: &HashMap<Entity, ObjectHandle>, shared: &SharedState) {
    let _span = trace_span!("mesh_skinning").entered();

    for (skeleton, equipment) in world.query::<(&SkeletonInstance, &mut Equipment)>().iter() {
        // Compute "global" transform relative the the entity with the SkeletonInstance
        // for each bone.
        let mut bone_transforms = HashMap::new();
        let mut entity_parents = HashMap::new();
        let mut queue = VecDeque::new();
        for e in &skeleton.root_bones {
            queue.push_back(*e);
        }

        while let Some(entity) = queue.pop_front() {
            let mut transform = *world.get::<&Transform>(entity).unwrap();
            assert_ne!(transform.scale, Vec3::ZERO);

            if let Some(parent) = entity_parents.get(&entity) {
                let parent_transform = *bone_transforms.get(parent).unwrap();
                transform = parent_transform * transform;
            }

            bone_transforms.insert(entity, transform);

            if let Ok(children) = world.get::<&Children>(entity) {
                queue.extend(&children.0);
                for child in &children.0 {
                    entity_parents.insert(*child, entity);
                }
            }
        }

        for skin in equipment.items().map(|v| v.skins()).flatten() {
            let Some(handle) = objects.get(&skin.entity) else {
                continue;
            };

            let mut transforms = Vec::new();
            for bone_index in skin.bones.iter() {
                let bind_pose = skeleton.bones_bind_pose.get(bone_index).unwrap();
                let bone = skeleton.bones.get(bone_index).unwrap();

                let transform = *bone_transforms.get(bone).unwrap();
                let mat = transform.compute_matrix() * bind_pose.compute_matrix().inverse();

                // assert_ne!(mat.determinant(), 0.0);

                let transform = Transform::from_matrix(mat);

                transforms.push(transform);
            }

            shared.entities.update_bone_transforms(handle, transforms);
        }
    }
}
