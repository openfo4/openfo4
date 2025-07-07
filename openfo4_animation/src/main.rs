use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use game_common::components::Transform;
use glam::Vec3;
use openfo4_animation::animation::AnimationClip;
use openfo4_animation::driver::AnimationDriver;
use openfo4_animation::skeleton::{Bone, Skeleton};
use openfo4_animation::{build_graph, builder::animation_graph};
use openfo4_hkx::{HkObject, HkxFile};

fn main() {
    // let p = "../../archive/raw/Data/meshes/actors/character/behaviors/raiderrootbehavior.hkx";
    // let p = "../../archive/raw/Data/meshes/actors/alien/behaviors/alienrootbehavior.hkx";
    // let p =
    //     "../../archive/raw/Data/meshes/actors/shared/behaviors/aliensharedcorewrappingbehavior.hkx";
    // let p = "../../archive/raw/Data/meshes/actors/character/behaviors/usespinetrackingfurniturewrappingbehavior.hkx";
    let p = "../../archive/raw/Data/meshes/actors/character/behaviors/weaponbehavior.hkx";
    // let p = "../../archive/raw/Data/meshes/actors/character/behaviors/mtbehavior.hkx";
    //let p = "../../archive/raw/Data/meshes/actors/character/behaviors/dialoguebehavior.hkx";
    // let p = "../../archive/raw/Data/meshes/actors/turret/behaviors/turretstandingrootbehavior.hkx";
    // let p = "../../archive/raw/Data/meshes/actors/turret/behaviors/turretsharedbehavior.hkx";
    let x = std::fs::read(p).unwrap();
    let f = HkxFile::decode(&x[..]).unwrap();

    let root = "../../archive/raw/Data/meshes/actors/turret";

    let root_file = std::fs::read(format!("{}/{}", root, "turretstanding.hkx")).unwrap();
    let root_file = HkxFile::decode(&root_file[..]).unwrap();
    dbg!(&root_file);

    let project_data = root_file
        .iter()
        .find_map(|obj| match obj {
            HkObject::ProjectData(data) => Some(data),
            _ => None,
        })
        .unwrap();

    let character_filename = project_data
        .string_data
        .character_filenames
        .first()
        .unwrap()
        .replace("\\", "/")
        .to_lowercase();

    dbg!(&character_filename);

    let character_file =
        HkxFile::decode(&std::fs::read(format!("{}/{}", root, character_filename)).unwrap())
            .unwrap();

    dbg!(&character_file);

    let character_data = character_file
        .iter()
        .find_map(|obj| match obj {
            HkObject::CharacterData(data) => Some(data),
            _ => None,
        })
        .unwrap();

    dbg!(&character_data.string_data.rig_name);

    let skeleton_filename = character_data
        .string_data
        .rig_name
        .replace("\\", "/")
        .replace(".hkt", ".hkx")
        .to_lowercase();
    dbg!(&skeleton_filename);
    let skeleton_file =
        HkxFile::decode(&std::fs::read(format!("{}/{}", root, skeleton_filename)).unwrap())
            .unwrap();

    let skeleton = load_skeleton(&skeleton_file).unwrap();
    dbg!(&skeleton);

    // let character = todo!();

    build_graph(&f);
    let graph = animation_graph(&f);

    dbg!(&graph);

    std::fs::write(
        "graph_compiled.dot",
        openfo4_animation::debug::draw(&graph.graph(), &[]),
    )
    .unwrap();

    // std::fs::write(
    //     "graph_compiled.dot",
    //     petgraph::dot::Dot::with_attr_getters(&graph.graph(), || String::new())
    //         .to_string()
    //         .as_bytes(),
    // )
    // .unwrap();

    let mut anim_clips = HashMap::new();

    for file in graph.animation_files() {
        let path = file.replace("\\", "/").replace("hkt", "hkx").to_lowercase();
        dbg!(&path);

        let anim_file =
            HkxFile::decode(&std::fs::read(format!("{}/{}", root, path)).unwrap()).unwrap();

        dbg!(anim_clips.len());

        for object in anim_file.iter() {
            match object {
                HkObject::SplineCompressedAnimation(anim) => {
                    let keyframes = anim.decompress();

                    anim_clips.insert(
                        file.clone(),
                        AnimationClip {
                            duration: Duration::from_secs_f32(anim.animation.duration),
                            num_bones: anim.animation.num_of_transform_tracks,
                            keyframes: keyframes
                                .into_iter()
                                .map(|t| Transform {
                                    translation: t.translation,
                                    rotation: t.rotation,
                                    scale: t.scale,
                                })
                                .collect(),
                            num_frames: 0,
                            skeleton_bone_to_animation_bone: Default::default(),
                            annotations: Vec::new(),
                            flags: Default::default(),
                        },
                    );
                }
                _ => (),
            }
        }

        dbg!(anim_clips.len());
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
                            rotation: transform.rotation,
                            scale: Vec3::ONE,
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
