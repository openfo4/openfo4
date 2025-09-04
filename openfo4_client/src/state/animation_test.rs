use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Arc, OnceLock};

use game_common::components::{Axis, Transform};
use game_core_pipeline::entities::MaterialHandle;
use game_render::mesh::Mesh;
use glam::{IVec2, Vec3, Vec4};
use hecs::{Entity, World};
use image::imageops::replace;
use openfo4_animation::builder::{Builder, Hint};
use openfo4_animation::driver::AnimationDriver;
use openfo4_animation::skeleton::{BoneMarker, SkeletonInstance};
use openfo4_animation::{AnimationSet, Value};
use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;
use openfo4_hkx::behavior::VariableType;
use openfo4_hkx::{HkObject, HkxFile};
use openfo4_physics::components::{
    Capsule, Collider, ColliderShape, RigidBody, RigidBodyKind, TriMesh,
};

use crate::assets::AssetServer;
use crate::assets::hkx::FileHkxHandle;
use crate::assets::nif_v2::{Instantiate, NIF_TO_METERS, NifScene};
use crate::cell::CellId;
use crate::components::{NiTransform, Player};
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::anim_starter::{ActiveBehaviorGraphs, AnimationEventQueue};
use crate::state::animation::{SkeletonConfig, load_anim};
use crate::systems::PlayerControlState;
use crate::systems::controller::ControllerShape;

pub struct SkinInstance {
    pub mesh: Mesh,
    pub mesh_bone_weights: Vec<Vec4>,
    pub mesh_bone_indices: Vec<[u8; 4]>,
    pub bones: Vec<usize>,
    pub entity: Option<Entity>,
    pub material: MaterialHandle,
}

pub struct SkinInstances(pub Vec<SkinInstance>);

pub fn animation_test(
    loader: &FileLoader,
    shared: &SharedState,
    world: &mut World,
    server: &AssetServer,
    asset_loader: &AssetLoader,
) {
    static ONCE: OnceLock<()> = OnceLock::new();

    let project_file = "actors/turret/turretstanding.hkx";
    let behavior_file = "meshes/actors/turret/behaviors/turretsharedbehavior.hkx";
    let search_paths = ["meshes/actors/turret/animations/standing"];
    let skin_path = "meshes/actors/turret/characterassets/turretstanding.nif";
    let nif_skeleton = "actors/turret/characterassets/turretstandingskeleton.nif";

    let project_file = "actors/character/raiderproject.hkx";
    let behavior_filename = "meshes/actors/character/behaviors/weaponbehavior.hkx";
    // let behavior_filename = "meshes/actors/character/behaviors/mtbehavior.hkx";
    // let behavior_filename = "meshes/actors/character/behaviors/raiderrootbehavior.hkx";
    let behavior_file = behavior_filename;
    let search_paths = [
        // "meshes/actors/character/animations/mt/player",
        // "meshes/actors/character/animations/mt/confident",
        // "meshes/actors/character/animations/mt/neutral",
        // "meshes/actors/character/animations",
        // "meshes/actors/character/animations/furniture/water",
        "meshes/actors/character/animations/weapon/laserpistol",
        "meshes/actors/character/animations/weapon/pistol",
        "meshes/actors/character/animations",
    ];
    // let anim_search_path = "meshes/actors/character/animations/";
    // let skin_path = "meshes/actors/character/characterassets/malebody.nif";
    let skin_path = "meshes/armor/armoredcoat/outfitf.nif";
    let nif_skeleton = "actors/character/characterassets/skeleton.nif";

    let char_path = "meshes/actors/character/characters/raidercharacter.hkx";

    // let raider_root_path = "meshes/actors/character/behaviors/raiderrootbehavior.hkx";

    ONCE.get_or_init(|| {
        let (nif_skeleton, skeleton) =
            futures_lite::future::block_on(crate::state::animation::load(
                server,
                asset_loader,
                SkeletonConfig {
                    nif_skeleton: nif_skeleton,
                    hkx_project: project_file,
                },
            ));

        let mut char_file = loader.open(char_path).unwrap();
        let mut buf = Vec::new();
        char_file.read_to_end(&mut buf).unwrap();
        let char_file = HkxFile::decode(&buf).unwrap();

        let mut behavior_file = loader.open(behavior_file).unwrap();
        let mut buf = Vec::new();
        behavior_file.read_to_end(&mut buf).unwrap();
        let behavior_file = HkxFile::decode(&buf).unwrap();

        let mut props = HashMap::new();
        for obj in char_file.iter() {
            match obj {
                HkObject::CharacterData(data) => {
                    let vals = data.character_property_values.as_ref().unwrap();

                    for ((name, info), data) in data
                        .string_data
                        .character_property_names
                        .iter()
                        .zip(data.character_property_infos.iter())
                        .zip(
                            data.character_property_values
                                .as_ref()
                                .unwrap()
                                .word_variable_values
                                .iter(),
                        )
                    {
                        let value = match info.type_ {
                            VariableType::Invalid => Value::None,
                            VariableType::Bool => Value::Bool(data.value == 0),
                            VariableType::Int16 => Value::I16(data.value as i16),
                            VariableType::Int32 => Value::I32(data.value),
                            VariableType::Real => Value::F32(f32::from_bits(data.value as u32)),
                            VariableType::Pointer => {
                                let ptr = &vals.variant_variable_values[data.value as usize];

                                match char_file.get(ptr.unwrap()) {
                                    Some(HkObject::BoneWeightArray(weights)) => {
                                        Value::BoneWeights(weights.bone_weights.to_vec())
                                    }
                                    _ => todo!(),
                                }
                            }
                            _ => todo!(),
                        };

                        props.insert(name.clone(), value);
                    }
                }
                _ => (),
            }
        }

        let mut builder = Builder::new(behavior_file, props).unwrap();

        let graph = loop {
            match builder.build().unwrap() {
                Ok(graph) => break graph,
                Err(Hint::NeedFile(path)) => {
                    let dir = "meshes/actors/character";
                    let p = format!("{}/{}", dir, path.to_lowercase().replace("\\", "/"));

                    let mut file = loader.open(&p).unwrap();
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).unwrap();
                    let file = HkxFile::decode(&buf).unwrap();

                    builder.register_file(&path, file).unwrap();
                }
            }
        };

        let mut anim_clips = HashMap::new();

        for file in graph.animation_files() {
            if anim_clips.contains_key(&file) {
                continue;
            }

            let path = file.to_lowercase().replace("\\", "/").replace("hkt", "hkx");

            let file_stem = path.split("/").last().unwrap_or(&path);

            let search_paths: Vec<_> = search_paths
                .iter()
                .map(|v| format!("{}/{}", v, file_stem))
                .collect();
            let search_paths: Vec<_> = search_paths.iter().map(|v| v.as_str()).collect();

            // let anim_root = format!(
            //     "{}/{}",
            //     anim_search_path,
            //     path.split("/").last().unwrap_or(&path)
            // );

            let clip = match futures_lite::future::block_on(load_anim(server, &search_paths)) {
                Ok(clip) => clip,
                Err(()) => {
                    tracing::error!("no animation found in paths {}", search_paths.join(", "));
                    continue;
                }
            };

            anim_clips.insert(file, clip);
        }

        let offset = CellId(IVec2::new(2, 7)).min();
        let offset =
            NiTransform::from_translation(offset.extend(0.0) - Vec3::new(1024.0, 1024.0, 0.0));

        let mut bones = HashMap::new();
        let mut bones_bind_pose = HashMap::new();
        let mut root_bones: HashSet<_> = skeleton.bones.iter().map(|b| b.index).collect();

        for (bone, bind_pose) in skeleton
            .bones
            .iter()
            .zip(skeleton.bind_pose_bone_transforms())
        {
            bones.insert(bone.name.to_lowercase(), bone);
            bones_bind_pose.insert(bone.index, bind_pose);

            for index in &bone.children {
                root_bones.remove(index);
            }
        }

        let mut bone_entities = HashMap::new();
        let mut root_bone_entities = Vec::new();

        let root = nif_skeleton.instantiate_with_feedback(
            |node, entity| {
                if let Some(bone) = bones.get(&node.name.to_lowercase()) {
                    bone_entities.insert(bone.index, entity);

                    if root_bones.contains(&bone.index) {
                        root_bone_entities.push(entity);
                    }
                }
            },
            offset.into(),
            world,
        );

        // dbg!(&bones_bind_pose);

        // let mut bone_parent = HashMap::new();
        // for bone in &skeleton.bones {
        //     let mut entities = Vec::new();

        //     for children in &bone.children {
        //         let entity = bones.get(children).unwrap();
        //         entities.push(*entity);

        //         bone_parent.insert(*children, bone.index);
        //     }

        //     let parent = bones.get(&bone.index).unwrap();

        //     if !entities.is_empty() {
        //         world.insert_one(*parent, Children(entities)).unwrap();
        //     }
        // }

        // let mut root_bones = Vec::new();
        // for bone in &skeleton.bones {
        //     if !bone_parent.contains_key(&bone.index) {
        //         let entity = bones.get(&bone.index).unwrap();
        //         root_bones.push(*entity);
        //     }
        // }

        // let mut child_entities = Vec::new();
        // child_entities.extend(root_bones.clone());

        for bone in bones.values() {
            if let Some(entity) = bone_entities.get(&bone.index) {
                world.insert_one(*entity, BoneMarker).unwrap();
                world.insert_one(*entity, bone.transform).unwrap();
            }
        }

        for entity in bone_entities.values() {
            world.insert_one(*entity, BoneMarker).unwrap();
        }

        // let skin_scene = futures_lite::future::block_on(nif_loader.load(skin_path)).unwrap();
        let skin_scene =
            futures_lite::future::block_on(asset_loader.load::<NifScene, _>(skin_path)).unwrap();

        let mut skin_instances = Vec::new();

        let mut skin_bind_poses = HashMap::new();

        for (_, node) in skin_scene.nodes.iter() {
            let Some(object) = &node.renderable else {
                continue;
            };

            let Some(skin) = &node.skin else {
                continue;
            };

            let mut bone_indices = Vec::new();
            for name in &skin.bones {
                // dbg!(&name);
                let bone = skeleton
                    .bones
                    .iter()
                    .find(|b| b.name.to_lowercase() == name.to_lowercase())
                    .unwrap();

                let bone_node = skin_scene
                    .nodes
                    .iter()
                    .find(|(_, node)| &node.name == name)
                    .unwrap();

                // dbg!(&bones_bind_pose[&bone.index]);
                // dbg!(bone.transform);
                // dbg!(bone_node.1.transform);
                skin_bind_poses.insert(bone.index, bone_node.1.transform);

                bone_indices.push(bone.index);
            }

            skin_instances.push(SkinInstance {
                mesh: skin.mesh.clone(),
                mesh_bone_weights: skin.weights.clone(),
                mesh_bone_indices: skin.indices.clone(),
                bones: bone_indices,
                entity: None,
                material: object.material.clone(),
            });
        }

        let char_data = char_file
            .iter()
            .find_map(|v| match v {
                HkObject::CharacterData(data) => Some(data),
                _ => None,
            })
            .unwrap();

        let height = char_data
            .character_controller_setup
            .rigid_body_setup
            .shape_setup
            .capsule_height
            * 1.0
            / NIF_TO_METERS;
        let radius = char_data
            .character_controller_setup
            .rigid_body_setup
            .shape_setup
            .capsule_radius
            * 1.0
            / NIF_TO_METERS;

        let collider_offset = Transform {
            translation: Vec3::new(0.0, 0.0, height / 2.0),
            ..Default::default()
        };
        let collider_shape = ColliderShape::Capsule(Capsule {
            axis: Axis::Z,
            // It seems that the height already includes the radius
            // at the top and bottom hemispheres.
            half_height: (height - radius) / 2.0,
            radius: radius / 2.0,
        });

        let collider = world.spawn((
            collider_offset,
            Collider {
                friction: 1.0,
                restitution: 0.0,
                shape: collider_shape.clone(),
            },
        ));

        world
            .insert(
                root,
                (
                    AnimationSet(Arc::new(anim_clips)),
                    SkeletonInstance {
                        skeleton,
                        bones: bone_entities,
                        bones_bind_pose,
                        root_bones: root_bone_entities,
                    },
                    AnimationDriver::new(Arc::new(graph)),
                    SkinInstances(skin_instances),
                    ActiveBehaviorGraphs(vec![
                        behavior_filename.replace("meshes/", ""),
                        "actors/character/behaviors/raiderrootbehavior.hkx".to_owned(),
                    ]),
                    AnimationEventQueue::default(),
                    RigidBody::FIXED,
                    Player,
                    PlayerControlState::default(),
                    ControllerShape {
                        offset: collider_offset,
                        shape: collider_shape,
                    },
                ),
            )
            .unwrap();

        world.add_children(root, collider);

        crate::systems::camera::attach_camera(world, root);

        // world.spawn((
        //     Transform {
        //         translation: Transform::from(offset).translation,
        //         rotation: Transform::from(offset).rotation,
        //         scale: Transform::from(offset).scale,
        //     },
        //     Children(child_entities),
        // ));
    });
}
