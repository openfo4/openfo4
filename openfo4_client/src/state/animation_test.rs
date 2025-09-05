use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use game_common::components::{Axis, Transform};
use game_core_pipeline::entities::MaterialHandle;
use game_render::mesh::Mesh;
use glam::{IVec2, Vec3, Vec4};
use hecs::{Entity, World};
use image::imageops::replace;
use openfo4_animation::builder::{Builder, Hint};
use openfo4_animation::driver::AnimationDriver;
use openfo4_animation::skeleton::{BoneMarker, Skeleton, SkeletonInstance};
use openfo4_animation::{AnimationGraph, AnimationSet, Value};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::WorldExt;
use openfo4_esp::records::Race;
use openfo4_esp::{FormId, RecordData, RecordKind};
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
use crate::state::animation::{self, SkeletonConfig, load_anim};
use crate::state::esp::EspLoader;
use crate::systems::PlayerControlState;
use crate::systems::controller::ControllerShape;
use crate::systems::equipment::equip_item;

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
    esp_loader: &EspLoader,
) {
    static ONCE: OnceLock<()> = OnceLock::new();

    let project_file = "actors/turret/turretstanding.hkx";
    let behavior_file = "meshes/actors/turret/behaviors/turretsharedbehavior.hkx";
    let search_paths = ["meshes/actors/turret/animations/standing"];
    let skin_path = "meshes/actors/turret/characterassets/turretstanding.nif";
    let nif_skeleton = "actors/turret/characterassets/turretstandingskeleton.nif";

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
        let race = futures_lite::future::block_on(
            esp_loader.lookup(Some(RecordKind::Race), FormId(0x13746)),
        )
        .unwrap();

        let race = match race.data {
            RecordData::Race(race) => race,
            _ => todo!(),
        };

        let character =
            futures_lite::future::block_on(load_character(&race, false, server, asset_loader))
                .unwrap();
        let behavior = futures_lite::future::block_on(load_behavior_data(
            behavior_filename,
            &character,
            &search_paths,
            server,
        ))
        .unwrap();

        let offset = CellId(IVec2::new(2, 7)).min();
        let offset =
            NiTransform::from_translation(offset.extend(0.0) - Vec3::new(1024.0, 1024.0, 0.0));

        let mut bones = HashMap::new();
        let mut bones_bind_pose = HashMap::new();
        let mut root_bones: HashSet<_> = character.skeleton.bones.iter().map(|b| b.index).collect();

        for (bone, bind_pose) in character
            .skeleton
            .bones
            .iter()
            .zip(character.skeleton.bind_pose_bone_transforms())
        {
            bones.insert(bone.name.to_lowercase(), bone);
            bones_bind_pose.insert(bone.index, bind_pose);

            for index in &bone.children {
                root_bones.remove(index);
            }
        }

        let mut bone_entities = HashMap::new();
        let mut root_bone_entities = Vec::new();

        let root = world.spawn((Transform::from(offset),));

        let mut queue = VecDeque::new();

        for index in &root_bones {
            queue.push_back((root, *index));
        }

        while let Some((parent, index)) = queue.pop_front() {
            let bone = &character.skeleton.bones[index];

            let entity = world.spawn((BoneMarker, bone.transform));

            if let Some((_, node)) = character
                .scene
                .nodes
                .iter()
                .find(|(_, node)| node.name.to_lowercase() == bone.name.to_lowercase())
            {
                let mut builder = node.build();
                world.insert(entity, builder.build()).unwrap();
            }

            world.add_children(parent, entity);

            for children in &bone.children {
                queue.push_back((entity, *children));
            }

            bone_entities.insert(bone.index, entity);

            if root_bones.contains(&bone.index) {
                root_bone_entities.push(entity);
            }
        }

        // let root = character.scene.instantiate_with_feedback(
        //     |node, entity| {
        //         if let Some(bone) = bones.get(&node.name.to_lowercase()) {
        //             bone_entities.insert(bone.index, entity);

        //             if root_bones.contains(&bone.index) {
        //                 root_bone_entities.push(entity);
        //             }
        //         }
        //     },
        //     offset.into(),
        //     world,
        // );

        // for bone in bones.values() {
        //     if let Some(entity) = bone_entities.get(&bone.index) {
        //         world.insert_one(*entity, BoneMarker).unwrap();
        //         world.insert_one(*entity, bone.transform).unwrap();
        //     }
        // }

        // for entity in bone_entities.values() {
        //     world.insert_one(*entity, BoneMarker).unwrap();
        // }

        let skeleton_instance = SkeletonInstance {
            skeleton: character.skeleton,
            bones: bone_entities,
            bones_bind_pose,
            root_bones: root_bone_entities,
        };

        let skin_instances = futures_lite::future::block_on(equip_item(
            skin_path,
            asset_loader,
            &skeleton_instance,
            world,
        ));
        futures_lite::future::block_on(equip_item(
            "meshes/weapons/laserweapons/laserpistol.nif",
            asset_loader,
            &skeleton_instance,
            world,
        ));

        let collider = world.spawn((
            character.controller_offset,
            Collider {
                friction: 1.0,
                restitution: 0.0,
                shape: character.controller_shape.clone(),
            },
        ));

        world
            .insert(
                root,
                (
                    behavior.clips,
                    skeleton_instance,
                    AnimationDriver::new(Arc::new(behavior.graph)),
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
                        offset: character.controller_offset,
                        shape: character.controller_shape,
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

#[derive(Clone, Debug)]
pub enum Error {
    NoNifSkeleton,
    NoHkxProject,
    NoProjectData,
    NoCharacterFilenames,
}

async fn load_character(
    race: &Race,
    female: bool,
    server: &AssetServer,
    loader: &AssetLoader,
) -> Result<ActorCharacter, Error> {
    let nif_skeleton;
    let hkx_project;

    if female {
        nif_skeleton = race
            .female_skeletal_model
            .as_ref()
            .ok_or(Error::NoNifSkeleton)?;

        hkx_project = race
            .behavior_graph
            .as_ref()
            .map(|g| g.0.female.as_ref())
            .flatten()
            .ok_or(Error::NoHkxProject)?;
    } else {
        nif_skeleton = race
            .male_skeletal_model
            .as_ref()
            .ok_or(Error::NoNifSkeleton)?;

        hkx_project = race
            .behavior_graph
            .as_ref()
            .map(|g| g.0.male.as_ref())
            .flatten()
            .ok_or(Error::NoHkxProject)?;
    };

    let nif_skeleton = nif_skeleton.0.path.as_str().unwrap();
    let hkx_project = "actors/character/raiderproject.hkx";

    let (nif_skeleton, skeleton) = animation::load(
        server,
        loader,
        SkeletonConfig {
            nif_skeleton,
            hkx_project,
        },
    )
    .await;

    let project_path = PathBuf::from("/meshes/").join(hkx_project);
    let project = server.load::<HkxFile>(&project_path).await.unwrap();
    let project = project
        .iter()
        .find_map(|obj| match obj {
            HkObject::ProjectData(data) => Some(data),
            _ => None,
        })
        .ok_or(Error::NoProjectData)?;

    let character_filename = project
        .string_data
        .character_filenames
        .get(0)
        .ok_or(Error::NoCharacterFilenames)?;

    let character_path = project_path.parent().unwrap().join(character_filename);
    let character_file = server.load::<HkxFile>(&character_path).await.unwrap();
    let character = character_file
        .iter()
        .find_map(|obj| match obj {
            HkObject::CharacterData(data) => Some(data),
            _ => None,
        })
        .unwrap();

    let mut props = HashMap::new();

    let vals = character.character_property_values.as_ref().unwrap();

    for ((name, info), data) in character
        .string_data
        .character_property_names
        .iter()
        .zip(character.character_property_infos.iter())
        .zip(
            character
                .character_property_values
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

                match character_file.get(ptr.unwrap()) {
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

    let controller = &character.character_controller_setup.rigid_body_setup;

    let height = controller.shape_setup.capsule_height / NIF_TO_METERS;
    let radius = controller.shape_setup.capsule_radius / NIF_TO_METERS;

    let controller_offset = Transform {
        translation: Vec3::new(0.0, 0.0, height / 2.0),
        ..Default::default()
    };

    let controller_shape = ColliderShape::Capsule(Capsule {
        axis: Axis::Z,
        // It seems that the height already includes the radius
        // at the top and bottom hemispheres.
        half_height: (height - radius) / 2.0,
        radius: radius / 2.0,
    });

    Ok(ActorCharacter {
        scene: nif_skeleton,
        skeleton,
        controller_offset,
        controller_shape,
        props,
    })
}

#[derive(Clone, Debug)]
struct ActorCharacter {
    scene: Handle<NifScene>,
    skeleton: Skeleton,
    controller_offset: Transform,
    controller_shape: ColliderShape,
    props: HashMap<String, Value>,
}

struct ActorBehaviorData {
    graph: AnimationGraph,
    clips: AnimationSet,
}

async fn load_behavior_data(
    behavior_path: &str,
    character: &ActorCharacter,
    search_paths: &[&str],
    server: &AssetServer,
) -> Result<ActorBehaviorData, ()> {
    let behavior_file = server.load::<HkxFile>(behavior_path).await.unwrap();

    let mut builder = Builder::new((*behavior_file).clone(), character.props.clone()).unwrap();

    let graph = loop {
        match builder.build().unwrap() {
            Ok(graph) => break graph,
            Err(Hint::NeedFile(path)) => {
                let dir = "meshes/actors/character";
                let p = format!("{}/{}", dir, path.to_lowercase().replace("\\", "/"));

                let file = server.load::<HkxFile>(&p).await.unwrap();
                builder.register_file(&path, (*file).clone()).unwrap();
            }
        }
    };

    let mut clips = HashMap::new();
    for file in graph.animation_files() {
        if clips.contains_key(&file) {
            continue;
        }

        let path = file.to_lowercase().replace("\\", "/").replace("hkt", "hkx");
        let file_stem = path.split("/").last().unwrap_or(&path);
        let search_paths: Vec<_> = search_paths
            .iter()
            .map(|v| format!("{}/{}", v, file_stem))
            .collect();
        let search_paths: Vec<_> = search_paths.iter().map(|v| v.as_str()).collect();

        let clip = match load_anim(server, &search_paths).await {
            Ok(clip) => clip,
            Err(err) => {
                tracing::error!(
                    "failed to load animation {} in searched paths {}",
                    path,
                    search_paths.join(", ")
                );
                continue;
            }
        };

        clips.insert(file, clip);
    }

    Ok(ActorBehaviorData {
        graph,
        clips: AnimationSet(Arc::new(clips)),
    })
}
