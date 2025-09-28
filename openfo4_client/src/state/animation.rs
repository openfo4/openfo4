use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

use game_common::components::Transform;
use openfo4_animation::animation::AnimationClip;
use openfo4_animation::skeleton::{Bone, Skeleton};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_hkx::{HkObject, HkxFile};

use crate::assets::Error;
use crate::assets::animation::FileAnimationClip;
use crate::assets::data_file::DataFile;
use crate::assets::nif_v2::NifScene;
use crate::loader::fix_path;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkeletonConfig<'a> {
    /// Path to the nif file of the skeleton.
    pub nif_skeleton: &'a str,
    /// Path to the project hkx file.
    pub hkx_project: &'a str,
}

pub async fn load(
    loader: &AssetLoader,
    config: SkeletonConfig<'_>,
) -> (Handle<NifScene>, Skeleton) {
    let nif_skeleton = format!("meshes/{}", fix_path(config.nif_skeleton));
    let project_path = format!("meshes/{}", fix_path(config.hkx_project));

    let nif_skeleton = loader.load::<NifScene, _>(&nif_skeleton).await.unwrap();

    let mut root = project_path.clone();
    if let Some(position) = root.rfind("/") {
        root.truncate(position);
    }

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

    let character_file = loader
        .load::<DataFile<HkxFile>, _>(&format!("{}/{}", root, hkt_to_hkx(character_filename)))
        .await
        .unwrap();

    let character_data = character_file
        .iter()
        .find_map(|object| match object {
            HkObject::CharacterData(data) => Some(data),
            _ => None,
        })
        .unwrap();

    let skeleton_filename = hkt_to_hkx(&character_data.string_data.rig_name);

    let skeleton_file = loader
        .load::<DataFile<HkxFile>, _>(&format!("{}/{}", root, skeleton_filename))
        .await
        .unwrap();

    let skeleton = load_skeleton(&skeleton_file, &nif_skeleton).unwrap();
    let skeleton = merge_skeletons(&nif_skeleton, &skeleton);

    (nif_skeleton, skeleton)
}

pub async fn load_anim(loader: &AssetLoader, search_paths: &[&str]) -> Result<AnimationClip, ()> {
    for path in search_paths {
        match loader.load::<FileAnimationClip, _>(*path).await {
            Ok(clip) => return Ok(clip.0.clone()),
            Err(Error::NotFound) => continue,
            Err(err) => {
                tracing::error!("failed to load animation clip: {}", err);
            }
        }
    }

    Err(())
}

fn hkt_to_hkx(path: &str) -> Cow<'_, str> {
    if let Some(path) = path.to_lowercase().strip_suffix(".hkt") {
        let mut path = path.to_owned();
        path.push_str(".hkx");
        Cow::Owned(path)
    } else {
        Cow::Borrowed(path)
    }
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
