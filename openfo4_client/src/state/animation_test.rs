use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::ptr::eq;
use std::sync::{Arc, OnceLock};

use game_common::components::{Axis, Transform};
use game_core_pipeline::entities::MaterialHandle;
use game_render::mesh::Mesh;
use glam::{IVec2, Vec3, Vec4};
use hecs::{Entity, World};
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
use openfo4_physics::components::{Capsule, Collider, ColliderShape, RigidBody};

use crate::assets::data_file::DataFile;
use crate::assets::nif_v2::{Instantiate, NIF_TO_METERS, NifScene};
use crate::cell::CellId;
use crate::components::{NiTransform, Player};
use crate::loader::{FileLoader, fix_path};
use crate::shared::SharedState;
use crate::state::anim_starter::{ActiveBehaviorGraphs, AnimationEventQueue};
use crate::state::animation::{self, SkeletonConfig, load_anim};
use crate::state::esp::EspLoader;
use crate::systems::PlayerControlState;
use crate::systems::controller::ControllerShape;
use crate::systems::equipment::{Equipment, equip_item};

#[derive(Clone, Debug)]
pub struct SkinInstance {
    pub mesh: Mesh,
    pub mesh_bone_weights: Vec<Vec4>,
    pub mesh_bone_indices: Vec<[u8; 4]>,
    pub bones: Vec<usize>,
    pub entity: Option<Entity>,
    pub material: MaterialHandle,
}

pub fn animation_test(
    loader: &FileLoader,
    shared: &SharedState,
    world: &mut World,
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
        "actors/character/animations/weapon/laserpistol",
        "actors/character/animations/weapon/pistol",
        "actors/character/animations",
    ];
    // let anim_search_path = "meshes/actors/character/animations/";
    // let skin_path = "meshes/actors/character/characterassets/malebody.nif";
    let skin_path = "meshes/armor/armoredcoat/outfitf.nif";
    let nif_skeleton = "actors/character/characterassets/skeleton.nif";

    let char_path = "meshes/actors/character/characters/raidercharacter.hkx";

    // let raider_root_path = "meshes/actors/character/behaviors/raiderrootbehavior.hkx";

    ONCE.get_or_init(|| {
        let offset = CellId(IVec2::new(2, 7)).min();
        for (index, offset) in [
            NiTransform::from_translation(offset.extend(0.0) - Vec3::new(1024.0, 1024.0, 0.0)),
            // NiTransform::from_translation(offset.extend(0.0) - Vec3::new(2000.0, 2000.0, 0.0)),
        ]
        .iter()
        .enumerate()
        {
            let race = futures_lite::future::block_on(
                esp_loader.lookup(Some(RecordKind::Race), FormId(0x13746)),
            )
            .unwrap();

            let race = match race.data {
                RecordData::Race(race) => race,
                _ => todo!(),
            };

            let character =
                futures_lite::future::block_on(load_character(&race, false, asset_loader)).unwrap();
            let behavior = futures_lite::future::block_on(load_behavior_data(
                &character.root_path,
                behavior_filename,
                &character,
                &search_paths,
                asset_loader,
            ))
            .unwrap();

            let root = world.spawn((Transform::from(*offset),));
            let skeleton_instance = character.instantiate(root, world).skeleton;

            let mut equipment = Equipment::default();

            for model_path in ["meshes/armor/armoredcoat/glovesf.nif"] {
                let scene =
                    futures_lite::future::block_on(asset_loader.load::<NifScene, _>(model_path))
                        .unwrap();

                equip_item(
                    root,
                    &scene,
                    &skeleton_instance,
                    world,
                    &mut equipment,
                    None,
                );
            }

            let base =
                futures_lite::future::block_on(asset_loader.load::<NifScene, _>(
                    "meshes/weapons/laserweapons/laserweapondummyreceiver.nif",
                ))
                .unwrap();
            let base = equip_item(root, &base, &skeleton_instance, world, &mut equipment, None);
            let receiver = futures_lite::future::block_on(
                asset_loader.load::<NifScene, _>("meshes/weapons/laserweapons/laserreceiver.nif"),
            )
            .unwrap();
            let receiver = equip_item(
                root,
                &receiver,
                &skeleton_instance,
                world,
                &mut equipment,
                Some(base),
            );

            let barrel = futures_lite::future::block_on(
                asset_loader
                    .load::<NifScene, _>("meshes/weapons/laserweapons/laserpistolbarrel.nif"),
            )
            .unwrap();
            let barrel = equip_item(
                root,
                &barrel,
                &skeleton_instance,
                world,
                &mut equipment,
                Some(receiver),
            );

            let grip = futures_lite::future::block_on(
                asset_loader.load::<NifScene, _>("meshes/weapons/laserweapons/laserpistolgrip.nif"),
            )
            .unwrap();
            let grip = equip_item(
                root,
                &grip,
                &skeleton_instance,
                world,
                &mut equipment,
                Some(receiver),
            );

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
                        ActiveBehaviorGraphs(vec![
                            behavior_filename.replace("meshes/", ""),
                            "actors/character/behaviors/raiderrootbehavior.hkx".to_owned(),
                        ]),
                        RigidBody::FIXED,
                        ControllerShape {
                            offset: character.controller_offset,
                            shape: character.controller_shape,
                        },
                        equipment,
                    ),
                )
                .unwrap();

            world.add_children(root, collider);

            if index == 0 {
                crate::systems::camera::attach_camera(world, root);
                world
                    .insert(
                        root,
                        (
                            Player,
                            PlayerControlState::default(),
                            AnimationEventQueue::default(),
                        ),
                    )
                    .unwrap();
            }
        }

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

pub async fn load_character(
    race: &Race,
    female: bool,
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
    let hkx_project = hkx_project
        .0
        .model
        .as_ref()
        .ok_or(Error::NoHkxProject)?
        .as_str()
        .ok_or(Error::NoHkxProject)?;
    let hkx_project = fix_path(hkx_project);

    let (nif_skeleton, skeleton) = animation::load(
        loader,
        SkeletonConfig {
            nif_skeleton,
            hkx_project: &hkx_project,
        },
    )
    .await;

    let project_path = PathBuf::from("/meshes/").join(hkx_project);
    let project = loader
        .load::<DataFile<HkxFile>, _>(project_path.to_str().unwrap())
        .await
        .unwrap();
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

    let root_path = project_path.parent().unwrap().to_str().unwrap().to_owned();

    let character_path = project_path.parent().unwrap().join(character_filename);
    let character_file = loader
        .load::<DataFile<HkxFile>, _>(character_path.to_str().unwrap())
        .await
        .unwrap();
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
        root_path: root_path,
        body_parts: Vec::new(),
    })
}

#[derive(Clone, Debug)]
pub struct ActorCharacter {
    pub scene: Handle<NifScene>,
    pub skeleton: Skeleton,
    pub controller_offset: Transform,
    pub controller_shape: ColliderShape,
    pub props: HashMap<String, Value>,
    pub root_path: String,
    pub body_parts: Vec<BodyPartBoneProps>,
}

impl ActorCharacter {
    pub fn instantiate(&self, root: Entity, world: &mut World) -> SpawnedActor {
        // Because the scene may have nodes with extra components we
        // will first spawn the scene and then "overlay" the HKX skeleton.
        let mut scene_nodes = HashMap::new();
        let scene = self.scene.instantiate_with_feedback(
            |node, entity| {
                scene_nodes.insert(node.name.to_lowercase(), entity);
            },
            Transform::default(),
            world,
        );

        world.add_children(root, scene);

        let mut ready_bones = Vec::new();
        for bone in &self.skeleton.bones {
            if scene_nodes.contains_key(&bone.name.to_lowercase()) {
                ready_bones.push(bone);
            }
        }

        let mut bones = HashMap::new();

        while let Some(bone) = ready_bones.pop() {
            let entity = *scene_nodes.get(&bone.name.to_lowercase()).unwrap();
            bones.insert(bone.index, entity);

            // Both transforms SHOULD be the same, however if they do
            // not we will choose the one from the hkx skeleton.
            world.insert(entity, (BoneMarker, bone.transform)).unwrap();

            for index in &bone.children {
                let child_bone = &self.skeleton.bones[*index];
                if scene_nodes.contains_key(&child_bone.name.to_lowercase()) {
                    continue;
                }

                let child_entity = world.reserve_entity();
                world.add_children(entity, child_entity);
                scene_nodes.insert(child_bone.name.to_lowercase(), child_entity);
                ready_bones.push(child_bone);
            }
        }

        let mut bones_bind_pose = HashMap::new();
        let mut root_bones: HashSet<_> = self.skeleton.bones.iter().map(|b| b.index).collect();
        for (bone, transform) in self
            .skeleton
            .bones
            .iter()
            .zip(self.skeleton.bind_pose_bone_transforms())
        {
            bones_bind_pose.insert(bone.index, transform);

            for index in &bone.children {
                root_bones.remove(index);
            }
        }

        let root_bone_entities = root_bones
            .into_iter()
            .map(|index| *bones.get(&index).unwrap())
            .collect();

        let mut body_parts = HashMap::new();
        for body_part in &self.body_parts {
            let mut bones = Vec::new();

            let Some(mut bone) = self
                .skeleton
                .bones
                .iter()
                .find(|bone| bone.name.to_lowercase() == body_part.end_bone.to_lowercase())
            else {
                continue;
            };

            loop {
                bones.push(bone.index);

                let Some(b) = self
                    .skeleton
                    .bones
                    .iter()
                    .find(|b| b.children.contains(&bone.index))
                else {
                    break;
                };

                bone = b;
            }

            body_parts.insert(body_part.id, bones);
        }

        SpawnedActor {
            skeleton: SkeletonInstance {
                skeleton: self.skeleton.clone(),
                bones,
                bones_bind_pose,
                root_bones: root_bone_entities,
                culled: HashSet::new(),
            },
            body_parts,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpawnedActor {
    pub skeleton: SkeletonInstance,
    pub body_parts: HashMap<u32, Vec<usize>>,
}

pub struct ActorBehaviorData {
    pub graph: AnimationGraph,
    pub clips: AnimationSet,
}

pub async fn load_behavior_data(
    root_path: &str,
    behavior_path: &str,
    character: &ActorCharacter,
    search_paths: &[&str],
    loader: &AssetLoader,
) -> Result<ActorBehaviorData, ()> {
    let mut behavior_path = fix_path(behavior_path);
    if !behavior_path.starts_with("meshes") {
        behavior_path = format!("meshes/{}", behavior_path)
    }

    let behavior_file = loader
        .load::<DataFile<HkxFile>, _>(&behavior_path)
        .await
        .unwrap();

    let mut builder = Builder::new((**behavior_file).clone(), character.props.clone()).unwrap();

    let graph = loop {
        match builder.build().unwrap() {
            Ok(graph) => break graph,
            Err(Hint::NeedFile(path)) => {
                let p = format!("{}/{}", root_path, path.to_lowercase().replace("\\", "/"));

                let file = match loader.load::<DataFile<HkxFile>, _>(&p).await {
                    Ok(f) => f,
                    Err(err) => {
                        // The game seems to also load from this directory for all races.
                        // This doesn't seem to be referenced anywhere, so it might be hardcoded?
                        let shared_path = "meshes/actors/shared";
                        let p =
                            format!("{}/{}", shared_path, path.to_lowercase().replace("\\", "/"));

                        loader.load::<DataFile<HkxFile>, _>(&p).await.unwrap()
                    }
                };

                builder.register_file(&path, (**file).clone()).unwrap();
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
            .map(|v| format!("meshes/{}/{}", v, file_stem))
            .collect();
        let search_paths: Vec<_> = search_paths.iter().map(|v| v.as_str()).collect();

        let clip = match load_anim(loader, &search_paths).await {
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

#[derive(Clone, Debug)]
pub struct BodyPartBoneProps {
    pub id: u32,
    pub start_bone: String,
    pub end_bone: String,
}
