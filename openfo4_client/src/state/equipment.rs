use std::collections::VecDeque;
use std::time::Duration;

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
use crate::systems::inventory::{AimProps, ItemData, ItemKind, WeaponItemData};

#[derive(Clone, Debug)]
pub struct EquipmentItem {
    pub resistances: HashMap<FormId, u32>,
    /// Keywords attached to the base item.
    pub base_keywords: HashSet<FormId>,
    /// Keywords after applying attachments.
    pub combined_keywords: HashSet<FormId>,
    pub weight: f32,
    pub value: f32,
    pub kind: ItemKind,
    pub data: ItemData,
}

#[derive(Clone, Debug)]
pub struct EquipmentItemModels {
    models: Vec<Handle<NifScene>>,
    attachment_parents: HashMap<usize, usize>,
}

impl EquipmentItemModels {
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

#[derive(Clone, Debug, Default)]
struct EquipmentProperties {
    resistances: HashMap<FormId, u32>,
    keywords: HashSet<FormId>,
    weight: f32,
    value: f32,
    weapon_capacity: u32,
    weapon_damage: HashMap<FormId, u32>,
    weapon_ammo_damage: u32,
    weapon_cone_of_fire_min_angle: f32,
    weapon_cone_of_fire_max_angle: f32,
    weapon_cone_of_fire_increase_per_shot: f32,
    weapon_cone_of_fire_decrease_per_second: f32,
    weapon_cone_of_fire_decrease_delay_milliseconds: u32,
}

pub async fn load_equipment_item(
    esp_loader: &EspLoader,
    form: FormId,
    attachments: &[FormId],
) -> EquipmentItem {
    let mut base_props = EquipmentProperties::default();

    let kind = match esp_loader.lookup(None, form).await.unwrap().data {
        RecordData::Armor(armor) => {
            base_props.weight = armor.data.weight;
            base_props.value = armor.data.value as f32;

            for resistance in armor.resistances.iter().flatten() {
                *base_props
                    .resistances
                    .entry(resistance.damage_type)
                    .or_default() += resistance.value;
            }

            if let Some(keywords) = armor.keywords {
                base_props.keywords.extend(keywords);
            }

            ItemKind::Armor
        }
        RecordData::Weapon(weapon) => {
            base_props.weight = weapon.data.weight;
            base_props.value = weapon.data.value as f32;

            if let Some(keywords) = weapon.keywords {
                base_props.keywords.extend(keywords);
            }

            base_props.weapon_capacity = u32::from(weapon.data.capacity);

            // Physical damage is hardcoded in the data field.
            // Other damage types are set in the damage_types field instead.
            if weapon.data.base_damage != 0 {
                base_props
                    .weapon_damage
                    .insert(DAMAGE_TYPE_PHYSICAL, u32::from(weapon.data.base_damage));
            }

            // FIXME: What happens when this field contains physical damage?
            if let Some(damage_types) = weapon.damage_types {
                for damage_type in &*damage_types {
                    base_props
                        .weapon_damage
                        .insert(damage_type.typ, damage_type.amount);
                }
            }

            if !weapon.data.ammo.is_null() {
                match esp_loader
                    .lookup(Some(RecordKind::Ammo), weapon.data.ammo)
                    .await
                    .unwrap()
                    .data
                {
                    RecordData::Ammo(ammo) => {
                        // The ammo damage is added to the final damage at the end. We must keep
                        // the base_weapon_damage as is to not break attachment values.
                        // FIXME: Also this seems to be a float? Why?
                        base_props.weapon_ammo_damage = ammo.projectile_data.damage as u32;
                    }
                    _ => unreachable!(),
                }
            }

            if let Some(form) = weapon.aim_model {
                match esp_loader
                    .lookup(Some(RecordKind::AimModel), form)
                    .await
                    .unwrap()
                    .data
                {
                    RecordData::AimModel(model) => {
                        base_props.weapon_cone_of_fire_min_angle =
                            model.data.cone_of_fire_min_angle;
                        base_props.weapon_cone_of_fire_max_angle =
                            model.data.cone_of_fire_max_angle;
                        base_props.weapon_cone_of_fire_increase_per_shot =
                            model.data.cone_of_fire_increase_per_shot;
                        base_props.weapon_cone_of_fire_decrease_per_second =
                            model.data.cone_of_fire_decrease_per_second;
                        base_props.weapon_cone_of_fire_decrease_delay_milliseconds =
                            model.data.cone_of_fire_decrease_delay_ms;
                    }
                    _ => unreachable!(),
                }
            }

            ItemKind::Weapon
        }
        _ => unreachable!(),
    };

    let mut combined_props = base_props.clone();

    for attachment in attachments {
        let RecordData::ObjectModification(attachment) =
            esp_loader.lookup(None, *attachment).await.unwrap().data
        else {
            unreachable!()
        };

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

            enum ModificationClass<'base, 'combined> {
                ScalarF32(&'base f32, &'combined mut f32),
                ScalarU32(&'base u32, &'combined mut u32),
            }

            let class = match property.property {
                ObjectModificationPropertyProperty::Keywords => {
                    let keyword = params.to_formid::<0>().unwrap();

                    match property.function_type {
                        ObjectModificationFunctionType::Add => {
                            combined_props.keywords.insert(keyword);
                        }
                        ObjectModificationFunctionType::Rem => {
                            combined_props.keywords.remove(&keyword);
                        }
                        _ => unreachable!(),
                    }

                    continue;
                }
                ObjectModificationPropertyProperty::Weight => {
                    ModificationClass::ScalarF32(&base_props.weight, &mut combined_props.weight)
                }
                ObjectModificationPropertyProperty::Value => {
                    ModificationClass::ScalarF32(&base_props.value, &mut combined_props.value)
                }
                // "Rating" actually is physical damage resistance.
                ObjectModificationPropertyProperty::Rating => {
                    let value = params.to_u32::<0>().unwrap();

                    let resistance = combined_props
                        .resistances
                        .entry(DAMAGE_TYPE_PHYSICAL)
                        .or_default();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => *resistance = value,
                        ObjectModificationFunctionType::Add => *resistance += value,
                        _ => unreachable!(),
                    }

                    continue;
                }
                ObjectModificationPropertyProperty::DamageTypeValue => {
                    let dmg_type = params.to_formid::<0>().unwrap();
                    let value = params.to_u32::<1>().unwrap();

                    let resistance = combined_props.resistances.entry(dmg_type).or_default();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => *resistance = value,
                        ObjectModificationFunctionType::Add => *resistance += value,
                        _ => unreachable!(),
                    }

                    continue;
                }
                ObjectModificationPropertyProperty::AimModelMinConeDegrees => {
                    ModificationClass::ScalarF32(
                        &base_props.weapon_cone_of_fire_min_angle,
                        &mut combined_props.weapon_cone_of_fire_min_angle,
                    )
                }
                ObjectModificationPropertyProperty::AimModelMaxConeDegrees => {
                    ModificationClass::ScalarF32(
                        &base_props.weapon_cone_of_fire_max_angle,
                        &mut combined_props.weapon_cone_of_fire_max_angle,
                    )
                }
                ObjectModificationPropertyProperty::AimModelConeIncreasePerShot => {
                    ModificationClass::ScalarF32(
                        &base_props.weapon_cone_of_fire_increase_per_shot,
                        &mut combined_props.weapon_cone_of_fire_increase_per_shot,
                    )
                }
                ObjectModificationPropertyProperty::AimModelConeDecreasePerSec => {
                    ModificationClass::ScalarF32(
                        &base_props.weapon_cone_of_fire_decrease_per_second,
                        &mut combined_props.weapon_cone_of_fire_decrease_per_second,
                    )
                }
                ObjectModificationPropertyProperty::AimModelConeDecreaseDelayMs => {
                    ModificationClass::ScalarU32(
                        &base_props.weapon_cone_of_fire_decrease_delay_milliseconds,
                        &mut combined_props.weapon_cone_of_fire_decrease_delay_milliseconds,
                    )
                }
                _ => continue,
            };

            match class {
                ModificationClass::ScalarF32(base, combined) => {
                    let value = params.to_f32::<0>().unwrap();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => *combined = value,
                        ObjectModificationFunctionType::Add => *combined += value,
                        ObjectModificationFunctionType::MulAdd => *combined += *base * value,
                        _ => unreachable!(),
                    }
                }
                ModificationClass::ScalarU32(base, combined) => {
                    let value = params.to_u32::<0>().unwrap();

                    match property.function_type {
                        ObjectModificationFunctionType::Set => *combined = value,
                        ObjectModificationFunctionType::Add => *combined += value,
                        ObjectModificationFunctionType::MulAdd => *combined += *base * value,
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    let data = match kind {
        ItemKind::Armor => ItemData::Armor,
        ItemKind::Weapon => {
            let phys_dmg = combined_props
                .weapon_damage
                .entry(DAMAGE_TYPE_PHYSICAL)
                .or_default();
            *phys_dmg = phys_dmg.saturating_add(combined_props.weapon_ammo_damage);

            ItemData::Weapon(WeaponItemData {
                drawn: false,
                current_ammo: 0,
                ammo_capacity: combined_props.weapon_capacity,
                damage: combined_props.weapon_damage,
                aim_props: AimProps {
                    // All cone of fire angles are in degrees.
                    cone_of_fire_angle_min: combined_props
                        .weapon_cone_of_fire_min_angle
                        .to_radians(),
                    cone_of_fire_angle_max: combined_props
                        .weapon_cone_of_fire_max_angle
                        .to_radians(),
                    cone_of_fire_inc_per_shot: combined_props
                        .weapon_cone_of_fire_increase_per_shot
                        .to_radians(),
                    cone_of_fire_dec_per_sec: combined_props
                        .weapon_cone_of_fire_decrease_per_second
                        .to_radians(),
                    cone_of_fire_dec_delay: Duration::from_millis(u64::from(
                        combined_props.weapon_cone_of_fire_decrease_delay_milliseconds,
                    )),
                },
            })
        }
    };

    EquipmentItem {
        resistances: combined_props.resistances,
        base_keywords: base_props.keywords,
        combined_keywords: combined_props.keywords,
        weight: combined_props.weight,
        value: combined_props.value,
        data,
        kind,
    }
}

pub async fn load_equipment_item_models(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    form: FormId,
    attachments: &[FormId],
    female: bool,
) -> EquipmentItemModels {
    let mut models = Vec::new();
    let mut attachment_point_parents = HashMap::new();

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
        }
        _ => unreachable!(),
    }

    let mut attachment_parents = HashMap::new();
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
    }

    EquipmentItemModels {
        models,
        attachment_parents,
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
