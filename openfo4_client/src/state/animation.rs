use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use game_common::components::Transform;
use openfo4_animation::animation::{AnimationClip, AnimationClipFlags, Annotation};
use openfo4_animation::skeleton::{Bone, Skeleton};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_hkx::{HkObject, HkxFile};

use crate::assets::AssetServer;
use crate::assets::nif_v2::NifScene;
use crate::loader::{FileLoader, fix_path};

pub struct AnimationLoader {
    file_loader: Arc<FileLoader>,
}

impl AnimationLoader {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkeletonConfig<'a> {
    /// Path to the nif file of the skeleton.
    pub nif_skeleton: &'a str,
    /// Path to the project hkx file.
    pub hkx_project: &'a str,
}

pub async fn load(
    server: &AssetServer,
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

    let root_file = server.load::<HkxFile>(&project_path).await.unwrap();

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

    let character_file = server
        .load::<HkxFile>(&format!("{}/{}", root, hkt_to_hkx(character_filename)))
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

    let skeleton_file = server
        .load::<HkxFile>(&format!("{}/{}", root, skeleton_filename))
        .await
        .unwrap();

    let skeleton = load_skeleton(&skeleton_file).unwrap();
    let skeleton = merge_skeletons(&nif_skeleton, &skeleton);

    (nif_skeleton, skeleton)
}

pub async fn load_anim(server: &AssetServer, search_paths: &[&str]) -> Result<AnimationClip, ()> {
    let mut clip = AnimationClip {
        duration: Duration::ZERO,
        num_bones: 0,
        num_frames: 0,
        keyframes: Vec::new(),
        skeleton_bone_to_animation_bone: HashMap::new(),
        annotations: Vec::new(),
        flags: AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING,
    };

    let mut file = None;
    for path in search_paths {
        match server.load::<HkxFile>(path).await {
            Ok(f) => {
                file = Some(f);
                break;
            }
            Err(err) => {
                tracing::error!("{}", err);
            }
        };
    }

    let Some(file) = file else {
        return Err(());
    };

    for object in file.iter() {
        match object {
            HkObject::SplineCompressedAnimation(anim) => {
                let keyframes = anim.decompress();

                clip.duration = Duration::from_secs_f32(anim.animation.duration);
                clip.num_bones = anim.animation.num_of_transform_tracks;
                clip.num_frames = anim.num_frames;
                clip.keyframes = keyframes
                    .into_iter()
                    .map(|t| Transform {
                        translation: t.translation,
                        rotation: t.rotation,
                        scale: t.scale,
                    })
                    .collect();

                for track in &*anim.animation.annotation_tracks {
                    for annotation in &*track.annotations {
                        let time = Duration::from_secs_f32(annotation.time);

                        let annotation = if annotation.text.starts_with("Sync") {
                            Annotation::Sync(time, annotation.text.clone())
                        } else {
                            continue;
                        };

                        clip.annotations.push(annotation);
                    }
                }
            }
            HkObject::AnimationBinding(binding) => {
                for (index, bone) in binding.transform_track_to_bone_indices.iter().enumerate() {
                    clip.skeleton_bone_to_animation_bone
                        .insert(*bone as usize, index);
                }
            }
            _ => (),
        }
    }

    Ok(clip)
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

fn load_skeleton(file: &HkxFile) -> Option<Skeleton> {
    for object in file.iter() {
        match object {
            HkObject::Skeleton(data) if data.name == "Root" => {
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
    // queue.extend(nif.nodes.root_keys());

    let root = nif
        .nodes
        .iter()
        .find_map(|(key, node)| (node.name == "Root").then_some(key))
        .unwrap();
    queue.push_back(root);

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
