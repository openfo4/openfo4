use game_common::components::Transform;
use game_core_pipeline::entities::ObjectHandle;
use game_tracing::trace_span;
use hashbrown::HashMap;
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_core::ecs::{Entity, World};

use crate::shared::SharedState;
use crate::systems::equipment::Equipment;

pub fn mesh_skinning(world: &World, objects: &HashMap<Entity, ObjectHandle>, shared: &SharedState) {
    let _span = trace_span!("mesh_skinning").entered();

    for (skeleton, equipment) in world.query::<(&SkeletonInstance, &mut Equipment)>().iter() {
        let mut bone_transforms = HashMap::with_capacity(skeleton.skeleton.bones.len());
        for bone in &skeleton.skeleton.bones {
            let bind_pose = skeleton.bones_bind_pose.get(&bone.index).unwrap();
            let transform = skeleton.transforms.global(bone.index);

            let mat = transform.compute_matrix() * bind_pose.compute_matrix().inverse();
            let transform = Transform::from_matrix(mat);
            bone_transforms.insert(bone.index, transform);
        }

        for skin in equipment.items().map(|v| v.skins()).flatten() {
            let Some(handle) = objects.get(&skin.entity) else {
                continue;
            };

            let mut transforms = Vec::with_capacity(skin.bones.len());
            for bone in &skin.bones {
                let transform = *bone_transforms.get(bone).unwrap();
                transforms.push(transform);
            }

            shared.entities.update_bone_transforms(handle, transforms);
        }
    }
}
