use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::OnceLock;

use game_common::components::{Axis, Transform};
use game_tasks::TaskPool;
use glam::{IVec2, Vec3};
use hecs::{Entity, World};
use openfo4_animation::builder::{Builder, Hint};
use openfo4_animation::skeleton::{BoneMarker, Skeleton, SkeletonInstance};
use openfo4_animation::{AnimationGraph, Value};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::record::{ModuleId, RecordId};
use openfo4_core::{Frozen, WorldExt};
use openfo4_esp::FormId;
use openfo4_esp::records::Race;
use openfo4_hkx::behavior::VariableType;
use openfo4_hkx::{HkObject, HkxFile};
use openfo4_physics::components::{Capsule, ColliderShape};

use crate::assets::animation::AnimationSet;
use crate::assets::data_file::DataFile;
use crate::assets::nif_v2::{Instantiate, LoadNifFlags, NIF_TO_METERS, NifScene};
use crate::cell::CellId;
use crate::components::{ActorMarker, NiTransform, Player, WorldSpace};
use crate::loader::fix_path;
use crate::state::anim_starter::AnimationEventQueue;
use crate::state::animation::{self, SkeletonConfig, load_anim};
use crate::state::esp::EspLoader;
use crate::state::hardcoded::HARDCODED_MODULE_ID;
use crate::state::level::Streamer;
use crate::state::npc::{ActorInstanceProps, BodyPartBone, ObjectKeywords, spawn_actor};
use crate::systems::PlayerControlState;
use crate::systems::inventory::{AddItemEventAsync, EquipItemEvent, Inventory};

#[derive(Clone, Debug)]
pub struct SkinInstance {
    pub entity: Entity,
    pub bones: Vec<usize>,
}

pub fn animation_test(
    events: &mut EventBus,
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

    let behavior_filename = "meshes/actors/character/behaviors/raiderrootbehavior.hkx";
    // let behavior_filename = "meshes/actors/character/behaviors/weaponbehavior.hkx";
    let subgraph_filename = "meshes/actors/character/behaviors/weaponbehavior.hkx";
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
        let start_x = std::env::var("OF4_SX")
            .unwrap_or("-13".to_owned())
            .parse()
            .unwrap();
        let start_y = std::env::var("OF4_SX")
            .unwrap_or("13".to_owned())
            .parse()
            .unwrap();

        let offset = CellId(IVec2::new(start_x, start_y)).min();
        for (index, offset) in [
            NiTransform::from_translation(offset.extend(0.0) + Vec3::new(1024.0, 1024.0, 8096.0)),
            // NiTransform::from_translation(offset.extend(0.0) - Vec3::new(2000.0, 2000.0, 0.0)),
        ]
        .iter()
        .enumerate()
        {
            let actor = futures_lite::future::block_on(spawn_actor(
                asset_loader,
                esp_loader,
                RecordId::new(HARDCODED_MODULE_ID, FormId(0x7)),
                &ActorInstanceProps { dead: false },
            ))
            .unwrap();

            let root = actor.clone().instantiate(world, events);
            world
                .insert_one(root, ObjectKeywords::new(actor.keywords))
                .unwrap();
            world.insert_one(root, Frozen::default()).unwrap();

            // let race = futures_lite::future::block_on(
            //     esp_loader.lookup(Some(RecordKind::Race), FormId(0x13746)),
            // )
            // .unwrap();

            // let race = match race.data {
            //     RecordData::Race(race) => race,
            //     _ => todo!(),
            // };

            // let character =
            //     futures_lite::future::block_on(load_character(&race, false, asset_loader)).unwrap();
            // let behavior = futures_lite::future::block_on(load_behavior_data(
            //     &character.root_path,
            //     behavior_filename,
            //     subgraph_filename,
            //     &character,
            //     &search_paths,
            //     asset_loader,
            // ))
            // .unwrap();

            let inventory = Inventory::default();
            let armor = FormId(0x001BBCB5);

            // events.send(AddItemEvent {
            //     actor: root,
            //     item: armor,
            //     attachments: Vec::new(),
            // });
            // events.send(EquipItemEvent {
            //     actor: root,
            //     item: armor,
            // });

            events.send(AddItemEventAsync {
                actor: root,
                item: RecordId::new(HARDCODED_MODULE_ID, FormId(0x9983b)),
                // OMODs
                attachments: vec![
                    RecordId::new(HARDCODED_MODULE_ID, FormId(0x12eced)),
                    RecordId::new(HARDCODED_MODULE_ID, FormId(0x9983d)),
                    RecordId::new(HARDCODED_MODULE_ID, FormId(0x99836)),
                    RecordId::new(HARDCODED_MODULE_ID, FormId(0x1877fa)),
                    RecordId::new(HARDCODED_MODULE_ID, FormId(0x4f21d)),
                ],
                hidden: false,
                count: 1,
                equip: true,
            });
            events.send(AddItemEventAsync {
                actor: root,
                item: RecordId::new(HARDCODED_MODULE_ID, FormId(0x000C1897)),
                attachments: Vec::new(),
                hidden: false,
                count: 100,
                equip: true,
            });
            events.send(EquipItemEvent {
                actor: root,
                item: RecordId::new(HARDCODED_MODULE_ID, FormId(0x9983b)),
            });

            world.insert_one(root, inventory).unwrap();
            world.insert_one(root, Transform::from(*offset)).unwrap();
            world
                .insert_one(
                    root,
                    WorldSpace {
                        record: RecordId::new(HARDCODED_MODULE_ID, FormId(0x3c)),
                        is_interior: false,
                    },
                )
                .unwrap();

            // let equipment = Equipment::default();

            // let collider = world.spawn((
            //     character.controller_offset,
            //     Collider {
            //         friction: 1.0,
            //         restitution: 0.0,
            //         shape: character.controller_shape.clone(),
            //     },
            // ));

            // world
            //     .insert(
            //         root,
            //         (
            //             behavior.clips,
            //             skeleton_instance,
            //             AnimationDriver::new(Arc::new(behavior.graph)),
            //             ActiveBehaviorGraphs(vec![
            //                 subgraph_filename.replace("meshes/", ""),
            //                 "actors/character/behaviors/raiderrootbehavior.hkx".to_owned(),
            //             ]),
            //             RigidBody::FIXED,
            //             ControllerShape {
            //                 offset: character.controller_offset,
            //                 shape: character.controller_shape,
            //             },
            //             equipment,
            //             inventory,
            //         ),
            //     )
            //     .unwrap();

            // world.add_children(root, collider);

            if index == 0 {
                let distance = std::env::var("OF4_DIST")
                    .map(|v| v.parse().unwrap())
                    .unwrap_or(0);

                crate::systems::camera::attach_camera(world, root);
                world
                    .insert(
                        root,
                        (
                            Player,
                            PlayerControlState::default(),
                            AnimationEventQueue::default(),
                            Streamer { distance },
                            ActorMarker,
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

    let nif_skeleton = nif_skeleton.0.path.as_ref().unwrap().as_str().unwrap();
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
                    Some(object) => {
                        tracing::warn!(
                            "unknown character pointer property {:?} in {}; ignoring",
                            object,
                            character_path.to_string_lossy(),
                        );

                        continue;
                    }
                    None => todo!(),
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

    let behavior_filename = fix_path(&format!(
        "{}/{}",
        root_path, character.string_data.behavior_filename
    ));

    Ok(ActorCharacter {
        behavior_filename,
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
    pub behavior_filename: String,
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
            LoadNifFlags::all(),
        );

        world.add_children(root, scene);

        let body_parts = self.build_body_parts(root);

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

        for (bone_index, bone_entity) in &bones {
            // A bone tagged body part must include all recursive
            // children since colliders may be children.
            // We skip children entities that are other bones.
            if let Some(body_part) = body_parts.get(&bone_index) {
                let mut queue = VecDeque::from(vec![*bone_entity]);

                while let Some(entity) = queue.pop_front() {
                    world.insert_one(entity, *body_part).unwrap();

                    let Ok(children) = world.get::<&Children>(entity) else {
                        continue;
                    };

                    for children in &children.0 {
                        if bones.values().any(|e| e == children) {
                            continue;
                        }

                        queue.push_back(*children);
                    }
                }
            }
        }

        SpawnedActor {
            skeleton: SkeletonInstance {
                skeleton: self.skeleton.clone(),
                bones,
                bones_bind_pose,
                root_bones: root_bone_entities,
                culled: HashSet::new(),
            },
        }
    }

    /// Returns the body parts for each bone.
    fn build_body_parts(&self, actor: Entity) -> HashMap<usize, BodyPartBone> {
        let mut body_parts = HashMap::new();
        for body_part in &self.body_parts {
            let mut bones = Vec::new();

            let Some(root_bone) = self
                .skeleton
                .bones
                .iter()
                .find(|bone| bone.name.to_lowercase() == body_part.start_bone.to_lowercase())
            else {
                continue;
            };

            let mut queue = VecDeque::new();
            queue.push_back(root_bone);

            while let Some(bone) = queue.pop_front() {
                bones.push(bone.index);

                for children in &bone.children {
                    let b = &self.skeleton.bones[*children];

                    if !self
                        .body_parts
                        .iter()
                        .any(|v| v.start_bone.to_lowercase() == b.name.to_lowercase())
                    {
                        queue.push_back(b);
                    }
                }
            }

            for bone in bones {
                body_parts.insert(
                    bone,
                    BodyPartBone {
                        actor,
                        id: body_part.id,
                    },
                );
            }
        }

        body_parts
    }
}

#[derive(Clone, Debug)]
pub struct SpawnedActor {
    pub skeleton: SkeletonInstance,
}

pub struct ActorBehaviorData {
    pub graph: AnimationGraph,
    pub clips: AnimationSet,
    pub behavior_files: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum LoadBehaviorGraphError {
    Load(Vec<(String, crate::assets::Error)>),
    Build(openfo4_animation::builder::Error),
}

pub async fn load_behavior_graph(
    loader: &AssetLoader,
    root_path: &str,
    behavior_path: &str,
    subgraph_path: &str,
    character: &ActorCharacter,
) -> Result<(AnimationGraph, Vec<String>), LoadBehaviorGraphError> {
    let mut behavior_path = fix_path(behavior_path);
    if !behavior_path.starts_with("meshes") {
        behavior_path = format!("meshes/{}", behavior_path);
    }

    let mut subgraph_path = fix_path(subgraph_path);
    if !subgraph_path.starts_with("meshes/") {
        subgraph_path = format!("meshes/{}", subgraph_path);
    }

    let subgraph_root = extract_behavior_root_path(&subgraph_path);

    let behavior_file = match loader.load::<DataFile<HkxFile>, _>(&behavior_path).await {
        Ok(f) => f,
        Err(err) => {
            return Err(LoadBehaviorGraphError::Load(vec![(behavior_path, err)]));
        }
    };

    let mut builder = Builder::new((**behavior_file).clone(), character.props.clone())
        .map_err(LoadBehaviorGraphError::Build)?;

    let mut behavior_files = Vec::new();

    let graph = loop {
        match builder.build().map_err(LoadBehaviorGraphError::Build)? {
            Ok(graph) => break graph,
            Err(Hint::NeedFile(path)) => {
                let mut file_loaded = false;
                let mut errors = Vec::new();

                for p in [
                    format!("{}/{}", root_path, path.to_lowercase().replace("\\", "/")),
                    // Loading from the subgraph directory can also happen.
                    format!(
                        "{}/{}",
                        subgraph_root,
                        path.to_lowercase().replace("\\", "/")
                    ),
                    // The game seems to also load from this directory for all races.
                    // This doesn't seem to be referenced anywhere, so it might be hardcoded?
                    format!(
                        "meshes/actors/shared/{}",
                        path.to_lowercase().replace("\\", "/")
                    ),
                ] {
                    match loader.load::<DataFile<HkxFile>, _>(&p).await {
                        Ok(file) => {
                            behavior_files.push(p);
                            builder
                                .register_file(&path, (**file).clone())
                                .map_err(LoadBehaviorGraphError::Build)?;

                            file_loaded = true;
                            break;
                        }
                        Err(err) => {
                            errors.push((p, err));
                        }
                    }
                }

                if !file_loaded {
                    return Err(LoadBehaviorGraphError::Load(errors));
                }
            }
            Err(Hint::NeedSwapGenerator) => {
                let file = match loader.load::<DataFile<HkxFile>, _>(&subgraph_path).await {
                    Ok(f) => f,
                    Err(err) => {
                        return Err(LoadBehaviorGraphError::Load(vec![(subgraph_path, err)]));
                    }
                };

                builder
                    .register_swap_generator((**file).clone())
                    .map_err(LoadBehaviorGraphError::Build)?;
            }
        }
    };

    behavior_files.push(behavior_path);
    behavior_files.push(subgraph_path);
    for file in &mut behavior_files {
        *file = file
            .strip_prefix("/")
            .unwrap_or(file)
            .strip_prefix("meshes/")
            .unwrap_or(file)
            .to_owned();
    }

    Ok((graph, behavior_files))
}

pub async fn load_behavior_data(
    root_path: &str,
    behavior_path: &str,
    subgraph_path: &str,
    character: &ActorCharacter,
    search_paths: &[&str],
    loader: &AssetLoader,
    pool: &TaskPool,
) -> Result<ActorBehaviorData, ()> {
    let (graph, behavior_files) =
        load_behavior_graph(loader, root_path, behavior_path, subgraph_path, character)
            .await
            .unwrap();

    let animation_files = graph.animation_files();
    let mut tasks = HashMap::with_capacity(animation_files.len());

    for file in animation_files {
        if tasks.contains_key(&file) {
            continue;
        }

        let path = file.to_lowercase().replace("\\", "/").replace("hkt", "hkx");
        let file_stem = path.split("/").last().unwrap_or(&path);
        let search_paths: Vec<_> = search_paths
            .iter()
            .map(|v| format!("meshes/{}/{}", fix_path(v), file_stem))
            .collect();

        let loader = loader.clone();
        let task = pool.spawn(async move {
            let search_paths: Vec<_> = search_paths.iter().map(|v| v.as_str()).collect();
            match load_anim(&loader, &search_paths).await {
                Ok(clip) => Some(clip),
                Err(err) => {
                    tracing::warn!(
                        "failed to load animation {} in searched paths {}",
                        path,
                        search_paths.join(", ")
                    );
                    None
                }
            }
        });

        tasks.insert(file, task);
    }

    let mut clips = HashMap::new();
    for (file, task) in tasks {
        if let Some(clip) = task.await {
            clips.insert(file, clip);
        }
    }

    Ok(ActorBehaviorData {
        graph,
        clips: AnimationSet(clips),
        behavior_files,
    })
}

#[derive(Clone, Debug)]
pub struct BodyPartBoneProps {
    pub id: u32,
    pub start_bone: String,
}

fn extract_behavior_root_path(file_path: &str) -> &str {
    let mut path = file_path;

    // Remove file part (x.hkx).
    if let Some(index) = path.rfind('/') {
        path = &path[..index];
    }

    // Remove behaviors section if present.
    if path.ends_with("behaviors") {
        if let Some(index) = path.rfind('/') {
            path = &path[..index];
        }
    }

    path
}
