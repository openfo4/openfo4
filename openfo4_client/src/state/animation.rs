use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use game_common::components::Transform;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use hashbrown::HashSet;
use hecs::{CommandBuffer, World};
use openfo4_animation::driver::AnimationDriver;
use openfo4_animation::skeleton::{Bone, Skeleton};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::path::DataPath;
use openfo4_esp::records::{SubgraphPerspective, SubgraphRole};
use openfo4_hkx::{HkObject, HkxFile};

use crate::assets::Error;
use crate::assets::animation::FileAnimationClip;
use crate::assets::data_file::DataFile;
use crate::assets::nif_v2::{LoadNifFlags, NifKey, NifScene};
use crate::state::anim_starter::ActiveBehaviorGraphs;
use crate::state::animation_test::load_behavior_data;
use crate::state::npc::{ActorAnimationGraphData, ObjectKeywords};
use crate::systems::inventory::{Inventory, ItemData};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkeletonConfig<'a> {
    /// Path to the nif file of the skeleton.
    pub nif_skeleton: &'a DataPath,
    /// Path to the project hkx file.
    pub hkx_project: &'a DataPath,
}

pub async fn load(
    loader: &AssetLoader,
    config: SkeletonConfig<'_>,
) -> (Handle<NifScene>, Skeleton) {
    let mut nif_skeleton = config.nif_skeleton.to_owned();
    nif_skeleton.push_front("meshes");

    let mut project_path = config.hkx_project.to_owned();
    project_path.push_front("meshes");

    let nif_skeleton = loader
        .load::<NifScene, _>(&NifKey {
            path: nif_skeleton.clone(),
            flags: LoadNifFlags::all(),
        })
        .await
        .unwrap();

    let root = project_path.parent().unwrap();

    let root_file = loader
        .load::<DataFile<HkxFile>, _>(&project_path)
        .await
        .unwrap();

    let project_data = root_file
        .iter()
        .find_map(|object| match object {
            HkObject::ProjectData(data) => Some(data),
            _ => None,
        })
        .unwrap();

    let character_filename = project_data
        .string_data
        .character_filenames
        .first()
        .unwrap();

    let mut character_path = root.to_owned();
    character_path.push(&**character_filename);
    character_path.set_extension("hkx");

    let character_file = loader
        .load::<DataFile<HkxFile>, _>(&character_path)
        .await
        .unwrap();

    let character_data = character_file
        .iter()
        .find_map(|object| match object {
            HkObject::CharacterData(data) => Some(data),
            _ => None,
        })
        .unwrap();

    let skeleton_filename = &character_data.string_data.rig_name;

    let mut skeleton_path = root.to_owned();
    skeleton_path.push(&**skeleton_filename);
    skeleton_path.set_extension("hkx");

    let skeleton_file = loader
        .load::<DataFile<HkxFile>, _>(&skeleton_path)
        .await
        .unwrap();

    let skeleton = load_skeleton(&skeleton_file, &nif_skeleton).unwrap();
    let skeleton = merge_skeletons(&nif_skeleton, &skeleton);

    (nif_skeleton, skeleton)
}

pub async fn load_anim(
    loader: &AssetLoader,
    search_paths: &[&DataPath],
) -> Result<Handle<FileAnimationClip>, ()> {
    for path in search_paths {
        match loader.load::<FileAnimationClip, _>(*path).await {
            Ok(clip) => return Ok(clip),
            Err(Error::NotFound) => continue,
            Err(err) => {
                tracing::error!("failed to load animation clip: {}", err);
            }
        }
    }

    Err(())
}

fn load_skeleton(file: &HkxFile, nif: &NifScene) -> Option<Skeleton> {
    for object in file.iter() {
        match object {
            HkObject::Skeleton(data)
                // The skeleton will attach to the nif scene where hkx skeleton
                // has the same name as the nif node. This is "Root" for most skeletons.
                // There is a theoretical concern if the hkx file contains multiple
                // matching skeletons, but this is never seen in the game files, so we will
                // not care for now.
                if nif
                    .nodes
                    .values()
                    .any(|node| node.name.to_lowercase() == data.name.to_lowercase()) =>
            {
                let mut bones = Vec::new();
                for (index, bone) in data.bones.iter().enumerate() {
                    let transform = &data.reference_pose[index];

                    bones.push(Bone {
                        index,
                        name: bone.name.clone(),
                        children: Vec::new(),
                        transform: Transform {
                            translation: transform.translation.truncate(),
                            rotation: transform.rotation.normalize(),
                            scale: transform.scale.truncate(),
                            ..Default::default()
                        },
                    });
                }

                for (index, parent) in data.parent_indices.iter().enumerate() {
                    if *parent == u16::MAX {
                        continue;
                    }

                    bones[*parent as usize].children.push(index);
                }

                return Some(Skeleton { bones });
            }
            _ => (),
        }
    }

    None
}

fn merge_skeletons(nif: &NifScene, hkx: &Skeleton) -> Skeleton {
    // Create the union of bones of the NIF and HKX skeleton.
    // Bones with the same names refer to the same bone in both files.
    // Note that we must keep the indcies of the HKX bones intact
    // because they are used to map animations tracks.

    let mut skeleton = hkx.clone();
    for bone in &mut skeleton.bones {
        bone.name = bone.name.to_lowercase();
    }

    let mut bones: HashMap<game_core::hierarchy::Key, usize> = HashMap::new();

    let mut queue = VecDeque::new();
    queue.extend(nif.nodes.root_keys());

    while let Some(key) = queue.pop_front() {
        let node = nif.nodes.get(key).unwrap();

        let name = node.name.to_lowercase();

        let index = match skeleton.bones.iter().find(|b| b.name == name) {
            Some(bone) => bone.index,
            None => {
                let index = skeleton.bones.len();
                skeleton.bones.push(Bone {
                    index,
                    name,
                    children: Vec::new(),
                    transform: node.transform,
                });

                if let Some(parent) = nif.nodes.parent_key(key) {
                    let parent_bone_index = *bones.get(&parent).unwrap();
                    let parent_bone = &mut skeleton.bones[parent_bone_index];

                    parent_bone.children.push(index);
                }

                index
            }
        };

        bones.insert(key, index);

        if let Some(children) = nif.nodes.children(key) {
            for (children, _) in children {
                queue.push_back(children);
            }
        }
    }

    skeleton
}

pub fn update_actor_animation_graphs(world: &mut World, loader: &AssetLoader, pool: &TaskPool) {
    let _span = trace_span!("update_actor_animation_graphs").entered();

    let mut buffer = CommandBuffer::new();

    for (entity, (graphs, keywords, inventory, active_graph)) in world.query_mut::<(
        &ActorAnimationGraphData,
        &ObjectKeywords,
        &Inventory,
        Option<&ActiveAnimationGraph>,
    )>() {
        let mut active_index = None;
        let mut max_actor_keywords = 0;
        let mut max_target_keywords = 0;

        let is_weapon_drawn = inventory.items().any(|(_, item)| match &item.data {
            ItemData::Weapon(weapon) => weapon.drawn,
            _ => false,
        });
        let weapon_keywords: HashSet<_> = inventory
            .items()
            .filter(|(_, item)| match &item.data {
                ItemData::Weapon(weapon) => weapon.drawn,
                _ => false,
            })
            .map(|(_, v)| v.combined_keywords.iter())
            .flatten()
            .copied()
            .collect();

        for (index, graph) in graphs.subgraphs.iter().enumerate() {
            let expected_role = if is_weapon_drawn {
                SubgraphRole::Weapon
            } else {
                SubgraphRole::Mt
            };

            if graph.flags.role != expected_role {
                continue;
            }

            if !matches!(graph.flags.perspective, SubgraphPerspective::Third) {
                continue;
            }

            if !graph.actor_keywords.iter().all(|kw| keywords.contains(*kw)) {
                continue;
            }

            if !graph
                .target_keywords
                .iter()
                .all(|kw| weapon_keywords.contains(kw))
            {
                continue;
            }

            if max_actor_keywords > graph.actor_keywords.len() {
                continue;
            }

            if max_target_keywords > graph.target_keywords.len() {
                continue;
            }

            active_index = Some(index);
            max_actor_keywords = graph.actor_keywords.len();
            max_target_keywords = graph.target_keywords.len();
        }

        let Some(active_index) = active_index else {
            continue;
        };

        if active_graph.is_some_and(|v| v.0 == active_index) {
            continue;
        }

        let graph = &graphs.subgraphs[active_index];

        let search_paths = graph
            .search_paths
            .iter()
            .map(|v| v.as_ref())
            .collect::<Vec<_>>();

        let behavior = futures_lite::future::block_on(load_behavior_data(
            &graphs.root_path,
            &graphs.root_behavior_path,
            &graph.behavior,
            &graphs.character,
            &search_paths,
            loader,
            pool,
        ))
        .unwrap();

        buffer.insert(
            entity,
            (
                ActiveAnimationGraph(active_index),
                AnimationDriver::new(Arc::new(behavior.graph)),
                behavior.clips,
                ActiveBehaviorGraphs(behavior.behavior_files.clone()),
            ),
        );
    }

    buffer.run_on(world);
}

#[derive(Copy, Clone, Debug)]
struct ActiveAnimationGraph(usize);

#[derive(Clone, Debug)]
pub struct AnimationPreloadCache {}
