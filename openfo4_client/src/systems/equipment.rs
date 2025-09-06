use std::collections::VecDeque;

use game_common::components::Transform;
use hashbrown::HashMap;
use hecs::World;
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;

use crate::assets::nif_v2::NifScene;
use crate::state::animation_test::SkinInstance;

pub async fn equip_item(
    skin_path: &str,
    loader: &AssetLoader,
    skeleton: &SkeletonInstance,
    world: &mut World,
) -> Vec<SkinInstance> {
    let scene = loader.load::<NifScene, _>(skin_path).await.unwrap();

    let mut skin_bind_pose = HashMap::new();
    let mut instances = Vec::new();

    for (key, node) in scene.nodes.iter() {
        match &node.skin {
            Some(skin) => {
                let Some(object) = &node.renderable else {
                    continue;
                };

                let mut bone_indices = Vec::new();
                for name in &skin.bones {
                    let bone = skeleton
                        .skeleton
                        .bones
                        .iter()
                        .find(|b| b.name.to_lowercase() == name.to_lowercase())
                        .unwrap();

                    let bone_node = scene
                        .nodes
                        .iter()
                        .find(|(_, node)| &node.name == name)
                        .unwrap();

                    skin_bind_pose.insert(bone.index, bone_node.1.transform);
                    bone_indices.push(bone.index);
                }

                instances.push(SkinInstance {
                    mesh: skin.mesh.clone(),
                    mesh_bone_weights: skin.weights.clone(),
                    mesh_bone_indices: skin.indices.clone(),
                    bones: bone_indices,
                    entity: None,
                    material: object.material.clone(),
                });
            }
            None => {
                // If the node matches the name of a bone we will attach it there.
                // This includes all children of the node, unless they match the
                // name of another bone, in which case they will get attached there
                // instead.
                let Some(bone) = skeleton
                    .skeleton
                    .bones
                    .iter()
                    .find(|b| b.name.to_lowercase() == node.name.to_lowercase())
                else {
                    continue;
                };

                let bone_entity = *skeleton.bones.get(&bone.index).unwrap();

                let mut queue = VecDeque::new();
                queue.push_back((bone_entity, key));

                while let Some((parent, key)) = queue.pop_front() {
                    let node = scene.nodes.get(key).unwrap();
                    let entity = world.spawn(node.build().build());

                    // If the node is root, i.e. matches the bone name the transform
                    // of that node should be overwritten by the bone transform.
                    if parent == bone_entity {
                        world.insert_one(entity, Transform::default()).unwrap();
                    }

                    world.add_children(parent, entity);

                    for (child_key, child_node) in scene.nodes.children(key).into_iter().flatten() {
                        // Skip the node if it matches the name of another bone.
                        if skeleton
                            .skeleton
                            .bones
                            .iter()
                            .any(|b| b.name.to_lowercase() == child_node.name.to_lowercase())
                        {
                            continue;
                        }

                        queue.push_back((entity, child_key));
                    }
                }
            }
        }
    }

    instances
}
