use std::collections::{HashMap, VecDeque};

use game_common::components::Transform;
use game_tracing::trace_span;
use glam::{Vec3, Vec4Swizzles};
use hecs::{CommandBuffer, World};
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_core::hierarchy::Children;

use crate::render::RenderObject;
use crate::shared::SharedState;
use crate::systems::equipment::Equipment;

pub fn mesh_skinning(world: &mut World, shared: &SharedState) {
    let _span = trace_span!("mesh_skinning").entered();

    let mut commands = CommandBuffer::new();
    for (_, (skeleton, equipment, children)) in world
        .query::<(&SkeletonInstance, &mut Equipment, &mut Children)>()
        .iter()
    {
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

        for skin in equipment.items_mut().map(|v| &mut v.skins).flatten() {
            let mut bone_matrices = Vec::new();
            for bone_index in skin.bones.iter() {
                let bind_pose = skeleton.bones_bind_pose.get(bone_index).unwrap();
                let bone = skeleton.bones.get(bone_index).unwrap();

                let transform = *bone_transforms.get(bone).unwrap();
                let mat = transform.compute_matrix() * bind_pose.compute_matrix().inverse();

                assert_ne!(mat.determinant(), 0.0);

                bone_matrices.push(mat);
            }

            let mut positions = skin.mesh.positions().to_vec();
            let mut normals = skin.mesh.normals().to_vec();
            let mut tangents = skin.mesh.tangents().to_vec();
            assert_eq!(positions.len(), skin.mesh_bone_indices.len());
            assert_eq!(positions.len(), skin.mesh_bone_weights.len());

            for ((((pos, normal), tangent), indices), weights) in positions
                .iter_mut()
                .zip(&mut normals)
                .zip(&mut tangents)
                .zip(&skin.mesh_bone_indices)
                .zip(&skin.mesh_bone_weights)
            {
                let skin_matrix = bone_matrices[indices[0] as usize] * weights.x
                    + bone_matrices[indices[1] as usize] * weights.y
                    + bone_matrices[indices[2] as usize] * weights.z
                    + bone_matrices[indices[3] as usize] * weights.w;

                let final_pos = skin_matrix * pos.extend(1.0);
                *pos = final_pos.truncate();

                let (scale, rotation, _) = skin_matrix.to_scale_rotation_translation();

                let final_normal = rotation * *normal;
                *normal = final_normal;

                let final_tangent = rotation * tangent.xyz();
                *tangent = final_tangent.extend(tangent.w);

                assert!(normal.is_finite());
                assert!(tangent.is_finite());
            }

            if let Some(entity) = skin.entity.take() {
                commands.despawn(entity);
                children.0.retain(|v| *v != entity);
            }

            let mut mesh = skin.mesh.clone();
            mesh.set_positions(positions);
            mesh.set_normals(normals);
            mesh.set_tangents(tangents);

            let new_entity = world.reserve_entity();
            commands.insert(
                new_entity,
                (
                    Transform::default(),
                    RenderObject {
                        mesh: shared.entities.create_mesh(mesh),
                        material: skin.material.clone(),
                    },
                ),
            );

            skin.entity = Some(new_entity);
            children.0.push(new_entity);
        }
    }

    commands.run_on(world);
}
