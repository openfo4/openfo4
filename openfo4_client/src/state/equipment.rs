use std::collections::VecDeque;

use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_asset::{AssetLoader, Handle};
use openfo4_esp::records::{
    ObjectModificationFunctionType, ObjectModificationPropertyProperty, ObjectModificationValueType,
};
use openfo4_esp::{FormId, RecordData, RecordKind};

use crate::assets::nif_v2::NifScene;
use crate::state::esp::EspLoader;
use crate::state::hardcoded_formids::DAMAGE_TYPE_PHYSICAL;
use crate::systems::equipment::{Equipment, EquipmentKey, equip_item};

#[derive(Clone, Debug)]
pub struct EquipmentItem {
    pub models: Vec<Handle<NifScene>>,
    pub attachment_parents: HashMap<usize, usize>,
    pub resistances: HashMap<FormId, u32>,
    /// Keywords attached to the base item.
    pub base_keywords: HashSet<FormId>,
    /// Keywords after applying attachments.
    pub combined_keywords: HashSet<FormId>,
    pub weight: f32,
    pub value: f32,
}

impl EquipmentItem {
    pub fn attach_scene(&self, world: &mut World, root: Entity) -> Vec<EquipmentKey> {
        // FIXME: We currently remove these components, then put them
        // back at the end to satisfy the borrowing impl of `equip_item`,
        // but this is not good because it may reshuffle the entity archetype data.
        let skeleton = world.remove_one::<SkeletonInstance>(root).unwrap();
        let mut equipment = world.remove_one::<Equipment>(root).unwrap();

        let mut keys = HashMap::new();
        let mut children = HashMap::<_, Vec<_>>::new();
        let mut queue = VecDeque::new();
        queue.extend(0..self.models.len());

        while let Some(index) = queue.pop_front() {
            let scene = &self.models[index];

            let parent_key = if let Some(parent) = self.attachment_parents.get(&index) {
                let Some(key) = keys.get(parent) else {
                    children.entry(parent).or_default().push(index);
                    continue;
                };

                Some(*key)
            } else {
                None
            };

            let key = equip_item(root, scene, &skeleton, world, &mut equipment, parent_key);
            keys.insert(index, key);

            if let Some(children) = children.remove(&index) {
                queue.extend(children);
            }
        }

        world.insert(root, (skeleton, equipment)).unwrap();

        keys.into_values().collect()
    }
}

pub async fn load_equipment_item(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    form: FormId,
    attachments: &[FormId],
    female: bool,
) -> EquipmentItem {
    let mut models = Vec::new();
    let mut base_resistances = HashMap::new();
    let mut base_keywords = HashSet::new();

    let mut attachment_point_parents = HashMap::new();

    let mut base_weight = 0.0;
    let mut base_value = 0.0;

    match esp_loader.lookup(None, form).await.unwrap().data {
        RecordData::Armor(armor) => {
            for model in &*armor.models {
                let addon = esp_loader
                    .lookup(Some(RecordKind::ArmorAddon), model.armor_addon)
                    .await
                    .unwrap();
                let addon = match addon.data {
                    RecordData::ArmorAddon(data) => data,
                    _ => unreachable!(),
                };

                let mut model_path = None;
                if female {
                    if let Some(model) = addon.female_model.model {
                        model_path = Some(model);
                    }
                } else {
                    if let Some(model) = addon.male_model.model {
                        model_path = Some(model);
                    }
                }

                let Some(model_path) = model_path else {
                    continue;
                };

                let scene = loader
                    .load::<NifScene, _>(model_path.as_str().unwrap())
                    .await
                    .unwrap();
                models.push(scene);
            }

            for resistance in armor.resistances.iter().flatten() {
                *base_resistances.entry(resistance.damage_type).or_default() += resistance.value;
            }
        }
        RecordData::Weapon(weapon) => {
            let model_index = models.len();
            if let Some(model) = &weapon.model.model {
                let scene = loader
                    .load::<NifScene, _>(model.as_str().unwrap())
                    .await
                    .unwrap();
                models.push(scene);
            }

            if let Some(attach_parent_slots) = weapon.attach_parent_slots {
                for slot in &attach_parent_slots {
                    attachment_point_parents.insert(*slot, model_index);
                }
            }

            base_weight = weapon.data.weight;
            base_value = weapon.data.value as f32;

            if let Some(keywords) = weapon.keywords {
                base_keywords.extend(keywords);
            }
        }
        _ => unreachable!(),
    }

    let mut resistances = base_resistances.clone();
    let mut attachment_parents = HashMap::new();
    let mut combined_keywords = base_keywords.clone();

    let mut total_weight = base_weight;
    let mut total_value = base_value;

    for attachment in attachments {
        let RecordData::ObjectModification(attachment) =
            esp_loader.lookup(None, *attachment).await.unwrap().data
        else {
            unreachable!()
        };

        let Some(parent) = attachment_point_parents.get(&attachment.data.attach_point) else {
            continue;
        };

        let model_index = if let Some(model) = &attachment.model.model {
            let scene = loader
                .load::<NifScene, _>(model.as_str().unwrap())
                .await
                .unwrap();

            let model_index = models.len();
            models.push(scene);

            attachment_parents.insert(model_index, *parent);

            model_index
        } else {
            // If the attachment does not have a model, but it does have further
            // attachment points we directly attachment the parents to the parent.
            *parent
        };

        for slot in &attachment.data.attach_parent_slots {
            attachment_point_parents.insert(*slot, model_index);
        }

        for property in &attachment.data.properties {
            let params = match property.value_type {
                ObjectModificationValueType::Int => Parameters::Int(property.value1),
                ObjectModificationValueType::Float => {
                    Parameters::Float(f32::from_bits(property.value1))
                }
                ObjectModificationValueType::Bool => Parameters::Bool(property.value1 != 0),
                ObjectModificationValueType::FormIdInt => {
                    Parameters::FormIdInt(FormId(property.value1), property.value2)
                }
                ObjectModificationValueType::FormIdFloat => Parameters::FormIdFloat(
                    FormId(property.value1),
                    f32::from_bits(property.value2),
                ),
                _ => unreachable!(),
            };

            match property.property {
                ObjectModificationPropertyProperty::Keywords => {
                    let keyword = params.to_formid::<0>().unwrap();

                    match property.function_type {
                        ObjectModificationFunctionType::Add => {
                            combined_keywords.insert(keyword);
                        }
                        ObjectModificationFunctionType::Rem => {
                            combined_keywords.remove(&keyword);
                        }
                        _ => unreachable!(),
                    }
                }
                ObjectModificationPropertyProperty::Weight => {
                    let value = params.to_f32::<0>().unwrap();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => total_weight = value,
                        ObjectModificationFunctionType::Add => total_weight += value,
                        ObjectModificationFunctionType::MulAdd => {
                            total_weight += base_weight * value
                        }
                        _ => unreachable!(),
                    }
                }
                ObjectModificationPropertyProperty::Value => {
                    let value = params.to_f32::<0>().unwrap();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => total_value = value,
                        ObjectModificationFunctionType::Add => total_value += value,
                        ObjectModificationFunctionType::MulAdd => total_value += base_value * value,
                        _ => unreachable!(),
                    }
                }
                // "Rating" actually is physical damage resistance.
                ObjectModificationPropertyProperty::Rating => {
                    let value = params.to_u32::<0>().unwrap();

                    let resistance = resistances.entry(DAMAGE_TYPE_PHYSICAL).or_default();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => *resistance = value,
                        ObjectModificationFunctionType::Add => *resistance += value,
                        _ => unreachable!(),
                    }
                }
                ObjectModificationPropertyProperty::DamageTypeValue => {
                    let dmg_type = params.to_formid::<0>().unwrap();
                    let value = params.to_u32::<1>().unwrap();

                    let resistance = resistances.entry(dmg_type).or_default();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => *resistance = value,
                        ObjectModificationFunctionType::Add => *resistance += value,
                        _ => unreachable!(),
                    }
                }
                _ => (),
            }
        }
    }

    EquipmentItem {
        models,
        resistances,
        attachment_parents,
        base_keywords,
        combined_keywords,
        weight: total_weight,
        value: total_value,
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Parameters {
    Int(u32),
    Float(f32),
    Bool(bool),
    FormIdInt(FormId, u32),
    FormIdFloat(FormId, f32),
}

impl Parameters {
    fn to_f32<const N: usize>(&self) -> Option<f32> {
        match N {
            0 => match *self {
                Self::Int(v) => Some(v as f32),
                Self::Float(v) => Some(v),
                _ => None,
            },
            1 => match *self {
                Self::FormIdInt(_, v) => Some(v as f32),
                Self::FormIdFloat(_, v) => Some(v),
                _ => None,
            },
            _ => unreachable!(),
        }
    }

    fn to_u32<const N: usize>(&self) -> Option<u32> {
        match N {
            0 => match *self {
                Self::Int(v) => Some(v),
                Self::Float(v) => Some(v as u32),
                _ => None,
            },
            1 => match *self {
                Self::FormIdInt(_, v) => Some(v),
                Self::FormIdFloat(_, v) => Some(v as u32),
                _ => None,
            },
            _ => unreachable!(),
        }
    }

    fn to_formid<const N: usize>(&self) -> Option<FormId> {
        match N {
            0 => match *self {
                Self::FormIdInt(v, _) => Some(v),
                Self::FormIdFloat(v, _) => Some(v),
                _ => None,
            },
            _ => unreachable!(),
        }
    }
}
