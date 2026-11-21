use std::collections::{HashMap, VecDeque};

use glam::Vec3;
use openfo4_animation::Value;
use openfo4_asset::AssetLoader;
use openfo4_common::components::{Axis, Transform};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::hierarchy::Children;
use openfo4_core::path::DataPathBuf;
use openfo4_esp::records::Race;
use openfo4_hkx::behavior::VariableType;
use openfo4_hkx::{HkObject, HkxFile};
use openfo4_physics::components::{Capsule, ColliderShape};

use crate::assets::data_file::DataFile;
use crate::assets::nif::NIF_TO_METERS;
use crate::state::actor::skeleton::{ActorSkeleton, SkeletonConfig};
use crate::state::npc::BodyPartBone;

#[derive(Clone, Debug)]
pub struct SkinInstance {
    pub entity: Entity,
    pub bones: Vec<usize>,
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

    let nif_skeleton = DataPathBuf::new(nif_skeleton.0.path.as_ref().unwrap());
    let hkx_project = DataPathBuf::new(hkx_project.0.model.as_ref().ok_or(Error::NoHkxProject)?);

    let skeleton = ActorSkeleton::load(
        loader,
        SkeletonConfig {
            nif_skeleton: &nif_skeleton,
            hkx_project: &hkx_project,
        },
    )
    .await;

    let mut project_path = hkx_project.to_owned();
    project_path.push_front("meshes");

    let project = loader
        .load::<DataFile<HkxFile>, _>(&project_path)
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

    let root_path = project_path.parent().unwrap();

    let mut character_path = project_path.parent().unwrap().to_owned();
    character_path.push(&**character_filename);

    let character_file = loader
        .load::<DataFile<HkxFile>, _>(&character_path)
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
                            character_path,
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

    let behavior_filename = root_path.join(&*character.string_data.behavior_filename);

    Ok(ActorCharacter {
        behavior_filename,
        skeleton,
        controller_offset,
        controller_shape,
        props,
        root_path: root_path.to_owned(),
        body_parts: Vec::new(),
    })
}

#[derive(Clone, Debug)]
pub struct ActorCharacter {
    pub behavior_filename: DataPathBuf,
    pub controller_offset: Transform,
    pub controller_shape: ColliderShape,
    pub props: HashMap<String, Value>,
    pub root_path: DataPathBuf,
    pub body_parts: Vec<BodyPartBoneProps>,
    pub skeleton: ActorSkeleton,
}

impl ActorCharacter {
    pub fn instantiate(&self, root: Entity, world: &mut World) {
        let skeleton = self.skeleton.instantiate(root, world);

        let body_parts = self.build_body_parts(root);

        for (bone_index, bone_entity) in &skeleton.bones {
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
                        if skeleton.bones.values().any(|e| e == children) {
                            continue;
                        }

                        queue.push_back(*children);
                    }
                }
            }
        }

        world.insert_one(root, skeleton).unwrap();
    }

    /// Returns the body parts for each bone.
    fn build_body_parts(&self, actor: Entity) -> HashMap<usize, BodyPartBone> {
        let mut body_parts = HashMap::new();
        for body_part in &self.body_parts {
            let mut bones = Vec::new();

            let Some(root_bone) = self
                .skeleton
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
                    let b = &self.skeleton.skeleton.bones[*children];

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
pub struct BodyPartBoneProps {
    pub id: u32,
    pub start_bone: String,
}
