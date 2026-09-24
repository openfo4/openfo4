use std::collections::VecDeque;
use std::time::Duration;

use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::hierarchy::Children;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::records::object_modification::{
    ObjectModificationFunctionType, ObjectModificationProperty, ObjectModificationPropertyType,
};
use openfo4_esp::records::{BipedObjectFlags, WeaponDataFlags};
use openfo4_esp::{FormId, RecordData, RecordKind};
use thiserror::Error;

use crate::assets::nif::{LoadNifFlags, NifKey, NifScene};
use crate::components::WorldSpace;
use crate::state::esp::EspLoader;
use crate::state::hardcoded::DAMAGE_TYPE_PHYSICAL;
use crate::state::level::items::{LoadItemProps, resolve_item};
use crate::systems::equipment::{Equipment, EquipmentKey, equip_item};
use crate::systems::inventory::{
    AimProps, ItemData, ItemKind, ItemStack, WeaponItemData, WeaponProjectileNode,
};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

// Note: The "P-" prefix is already stripped in the NifScene.
const PROJECTILE_NODE_NAME: &str = "ProjectileNode";

#[derive(Clone, Debug)]
pub struct EquipmentItem {
    pub name: String,
    pub resistances: HashMap<RecordId, u32>,
    /// Keywords attached to the base item.
    pub base_keywords: HashSet<RecordId>,
    /// Keywords after applying attachments.
    pub combined_keywords: HashSet<RecordId>,
    pub weight: f32,
    pub value: f32,
    pub kind: ItemKind,
    pub data: ItemData,
    pub equipment_slots: HashSet<u32>,
    pub visibility_slots: HashSet<u32>,
    pub script: Option<ScriptObject>,
}

#[derive(Clone, Debug)]
pub struct EquipmentItemModels {
    /// List of all renderable scenes for ths item.
    models: Vec<Handle<NifScene>>,
    /// Contains the parent scene that must be loaded before the current scene.
    attachment_parents: HashMap<usize, usize>,
    /// Index of the model which contains the `P-ProjectileNode` attachment point.
    ///
    /// This is where weapon projectiles should be spawned.
    projectile_node: Option<usize>,
}

impl EquipmentItemModels {
    pub async fn load(
        loader: &AssetLoader,
        esp_loader: &EspLoader,
        record: RecordId,
        attachments: &[RecordId],
        female: bool,
        visible_slots: &HashSet<u32>,
    ) -> Self {
        let mut models = Vec::new();
        let mut attachment_point_parents = HashMap::new();

        let mut default_addon_index = None;
        let mut scenes_by_addon_index = HashMap::<_, Vec<_>>::new();

        let record = esp_loader.lookup(None, record).await.unwrap();
        match record.local.data {
            RecordData::Armor(armor) => {
                for model in &*armor.models {
                    let armor_addon = esp_loader
                        .resolve_form(model.armor_addon, record.id)
                        .unwrap();

                    let addon = esp_loader
                        .lookup(Some(RecordKind::ArmorAddon), armor_addon)
                        .await
                        .unwrap();
                    let addon = match addon.local.data {
                        RecordData::ArmorAddon(data) => data,
                        _ => unreachable!(),
                    };

                    // We only want to include armor addons where ALL slots of the
                    // armor addon are also marked visible.
                    let mut slots = HashSet::new();
                    extract_biped_object_flags(addon.biped_body_template, &mut slots);
                    if !slots.is_subset(visible_slots) {
                        continue;
                    }

                    let model_path = if female {
                        addon.female_model.model
                    } else {
                        addon.male_model.model
                    };

                    let Some(model_path) = model_path else {
                        continue;
                    };

                    if default_addon_index.is_none() {
                        default_addon_index = Some(model.addon_index);
                    }

                    scenes_by_addon_index
                        .entry(model.addon_index)
                        .or_default()
                        .push(model_path);
                }
            }
            RecordData::Weapon(weapon) => {
                let model_index = models.len();
                if let Some(model) = &weapon.model.model {
                    let scene = loader
                        .load::<NifScene, _>(&NifKey {
                            path: DataPathBuf::new(model.as_bytes()),
                            flags: LoadNifFlags::all(),
                        })
                        .await
                        .unwrap();

                    models.push(scene);
                }

                if let Some(attach_parent_slots) = weapon.attach_parent_slots {
                    for slot in &attach_parent_slots {
                        let slot = esp_loader.resolve_form(*slot, record.id).unwrap();
                        attachment_point_parents.insert(slot, model_index);
                    }
                }
            }
            RecordData::HeadPart(data) => {
                if let Some(model) = &data.model.model {
                    let scene = loader
                        .load::<NifScene, _>(&NifKey {
                            path: DataPathBuf::new(model.as_bytes()),
                            flags: LoadNifFlags::all(),
                        })
                        .await
                        .unwrap();

                    models.push(scene);
                }
            }
            _ => unreachable!(),
        }

        let mut attachment_records = HashMap::new();
        let mut requires = HashMap::new();
        let mut provides = HashMap::<_, Vec<_>>::new();

        let mut addon_index = default_addon_index;

        for id in attachments {
            let RecordData::ObjectModification(attachment) =
                esp_loader.lookup(None, *id).await.unwrap().local.data
            else {
                unreachable!()
            };

            for prop in &attachment.data.properties {
                match prop.ty {
                    ObjectModificationPropertyType::AddonIndex(index) => match prop.function_type {
                        ObjectModificationFunctionType::Set => {
                            addon_index = u16::try_from(index).ok();
                        }
                        ty => {
                            tracing::warn!(
                                "object mod property addon index only supports funftion type set, found {:?}",
                                ty
                            );
                        }
                    },
                    _ => {}
                }
            }

            let attach_point = esp_loader
                .resolve_form(attachment.data.attach_point, *id)
                .unwrap();

            // Don't depend on the root attachment point, it is always provided and
            // loaded at the very first.
            if !attachment_point_parents.contains_key(&attach_point) {
                requires.insert(*id, attach_point);
            }

            for slot in &attachment.data.attach_parent_slots {
                let slot = esp_loader.resolve_form(*slot, *id).unwrap();
                provides.entry(*id).or_default().push(slot);
            }

            attachment_records.insert(*id, attachment);
        }

        if let Some(addon_index) = addon_index
            && let Some(paths) = scenes_by_addon_index.remove(&addon_index)
        {
            for path in paths {
                let scene = loader
                    .load::<NifScene, _>(&NifKey {
                        path: DataPathBuf::new(path.as_bytes()),
                        flags: LoadNifFlags::all(),
                    })
                    .await
                    .unwrap();
                models.push(scene);
            }
        }

        let mut attachment_parents = HashMap::new();

        for id in load_attachments_order(attachments, &requires, &provides) {
            let attachment = &attachment_records[&id];

            let attach_point = esp_loader
                .resolve_form(attachment.data.attach_point, id)
                .unwrap();

            let Some(parent) = attachment_point_parents.get(&attach_point) else {
                continue;
            };

            let model_index = if let Some(model) = &attachment.model.model {
                let scene = loader
                    .load::<NifScene, _>(&NifKey {
                        path: DataPathBuf::new(model.as_bytes()),
                        flags: LoadNifFlags::all(),
                    })
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
                let slot = esp_loader.resolve_form(*slot, id).unwrap();
                attachment_point_parents.insert(slot, model_index);
            }
        }

        let projectile_node =
            find_deepest_scene_with_attachment(&models, &attachment_parents, PROJECTILE_NODE_NAME);

        Self {
            models,
            attachment_parents,
            projectile_node,
        }
    }

    pub fn attach_scene(&self, world: &mut World, root: Entity) -> AttachedScenes {
        let _span = trace_span!("EquipmentItemModels::attach_scene").entered();

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

        // Copy worldspace to all new entities.
        // This is required for them to get rendered correctly.
        if let Ok(worldspace) = world.get::<&WorldSpace>(root).map(|v| *v) {
            let mut queue = vec![root];
            while let Some(e) = queue.pop() {
                world.insert_one(e, worldspace).unwrap();
                if let Ok(children) = world.get::<&Children>(e) {
                    queue.extend(&children.0);
                }
            }
        }

        let projectile_node = self.projectile_node.map(|index| {
            let key = keys.get(&index).unwrap();
            let item = equipment.get(*key).unwrap();
            let attachment = item.attachment(PROJECTILE_NODE_NAME).unwrap();
            WeaponProjectileNode {
                entity: attachment.entity,
                offset: attachment.offset,
            }
        });

        world.insert(root, (skeleton, equipment)).unwrap();

        AttachedScenes {
            keys: keys.into_values().collect(),
            projectile_node,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AttachedScenes {
    pub keys: Vec<EquipmentKey>,
    pub projectile_node: Option<WeaponProjectileNode>,
}

#[derive(Clone, Debug, Default)]
struct EquipmentProperties {
    resistances: HashMap<RecordId, u32>,
    keywords: HashSet<RecordId>,
    weight: f32,
    value: f32,
    weapon_ammo: Option<RecordId>,
    weapon_capacity: u32,
    weapon_damage: HashMap<RecordId, u32>,
    weapon_ammo_damage: u32,
    weapon_cone_of_fire_min_angle: f32,
    weapon_cone_of_fire_max_angle: f32,
    weapon_cone_of_fire_increase_per_shot: f32,
    weapon_cone_of_fire_decrease_per_second: f32,
    weapon_cone_of_fire_decrease_delay_milliseconds: u32,
    weapon_npcs_use_ammo: bool,
}

pub async fn load_item(
    esp_loader: &EspLoader,
    script_loader: &ScriptLoader,
    record: RecordId,
    attachments: &[RecordId],
    count: u64,
) -> Vec<ItemStack> {
    resolve_item(
        esp_loader,
        script_loader,
        record,
        count,
        attachments,
        &LoadItemProps {
            level: PLAYER_LEVEL,
            object_template_keywords: HashSet::new(),
        },
    )
    .await
}

pub async fn load_equipment_item(
    esp_loader: &EspLoader,
    script_loader: &ScriptLoader,
    record: RecordId,
    attachments: &[RecordId],
) -> EquipmentItem {
    let mut base_props = EquipmentProperties::default();
    let name;

    let mut equipment_slots = HashSet::new();
    let mut visibility_slots = HashSet::new();

    let kind = match esp_loader.lookup(None, record).await.unwrap().local.data {
        RecordData::Armor(armor) => {
            name = armor.id.as_str().unwrap().to_owned();

            base_props.weight = armor.data.weight;
            base_props.value = armor.data.value as f32;

            for resistance in armor.resistances.iter().flatten() {
                let damage_type = esp_loader
                    .resolve_form(resistance.damage_type, record)
                    .unwrap();

                *base_props.resistances.entry(damage_type).or_default() += resistance.value;
            }

            if let Some(keywords) = armor.keywords {
                for keyword in &*keywords {
                    let keyword = esp_loader.resolve_form(*keyword, record).unwrap();
                    base_props.keywords.insert(keyword);
                }
            }

            extract_biped_object_flags(armor.biped_body_template, &mut equipment_slots);

            for model in &*armor.models {
                let armor_addon = esp_loader.resolve_form(model.armor_addon, record).unwrap();

                let RecordData::ArmorAddon(addon) = esp_loader
                    .lookup(None, armor_addon)
                    .await
                    .unwrap()
                    .local
                    .data
                else {
                    todo!();
                };

                extract_biped_object_flags(addon.biped_body_template, &mut visibility_slots);
            }

            ItemKind::Armor
        }
        RecordData::Weapon(weapon) => {
            name = weapon.id.as_str().unwrap().to_owned();

            base_props.weight = weapon.data.weight;
            base_props.value = weapon.data.value as f32;

            if let Some(keywords) = weapon.keywords {
                for keyword in &*keywords {
                    let keyword = esp_loader.resolve_form(*keyword, record).unwrap();
                    base_props.keywords.insert(keyword);
                }
            }

            base_props.weapon_capacity = u32::from(weapon.data.capacity);

            base_props.weapon_npcs_use_ammo =
                weapon.data.flags.contains(WeaponDataFlags::NPCS_USE_AMMO);

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
                    let ty = esp_loader.resolve_form(damage_type.typ, record).unwrap();

                    base_props.weapon_damage.insert(ty, damage_type.amount);
                }
            }

            if !weapon.data.ammo.is_null() {
                let ammo = esp_loader.resolve_form(weapon.data.ammo, record).unwrap();
                base_props.weapon_ammo = Some(ammo);

                match esp_loader
                    .lookup(Some(RecordKind::Ammo), ammo)
                    .await
                    .unwrap()
                    .local
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
                let aim_model = esp_loader.resolve_form(form, record).unwrap();

                match esp_loader
                    .lookup(Some(RecordKind::AimModel), aim_model)
                    .await
                    .unwrap()
                    .local
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
        RecordData::Ammo(ammo) => {
            name = ammo.id.as_str().unwrap().to_owned();
            ItemKind::Ammo
        }
        _ => unreachable!(),
    };

    let mut combined_props = base_props.clone();

    for attachment_id in attachments {
        let RecordData::ObjectModification(attachment) = esp_loader
            .lookup(None, *attachment_id)
            .await
            .unwrap()
            .local
            .data
        else {
            unreachable!()
        };

        for property in &attachment.data.properties {
            if let Err(err) = apply_object_modification(
                esp_loader,
                *attachment_id,
                &base_props,
                &mut combined_props,
                property,
            ) {
                tracing::error!(
                    "failed to apply object modification property of attachment {:?}: {}",
                    attachment_id,
                    err
                );
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
                ammo: combined_props.weapon_ammo,
                projectile_node: None,
                npcs_use_ammo: combined_props.weapon_npcs_use_ammo,
            })
        }
        ItemKind::Ammo | ItemKind::Misc | ItemKind::Aid | ItemKind::Junk | ItemKind::HeadPart => {
            ItemData::None
        }
    };

    let script = load_item_script(record, script_loader).await;

    EquipmentItem {
        resistances: combined_props.resistances,
        base_keywords: base_props.keywords,
        combined_keywords: combined_props.keywords,
        weight: combined_props.weight,
        value: combined_props.value,
        data,
        kind,
        name,
        equipment_slots,
        visibility_slots,
        script,
    }
}

fn load_attachments_order(
    attachments: &[RecordId],
    requires: &HashMap<RecordId, RecordId>,
    provides: &HashMap<RecordId, Vec<RecordId>>,
) -> Vec<RecordId> {
    let mut queue: VecDeque<_> = attachments
        .iter()
        .filter(|id| !requires.contains_key(*id))
        .collect();

    let mut required_by = HashMap::<_, Vec<_>>::new();
    for (attachment, keyword) in requires {
        required_by.entry(keyword).or_default().push(attachment);
    }

    let mut attachments = Vec::new();

    while let Some(id) = queue.pop_front() {
        attachments.push(*id);

        let Some(keywords) = provides.get(id) else {
            continue;
        };

        for keyword in keywords {
            let Some(attachments) = required_by.remove(keyword) else {
                continue;
            };

            queue.extend(attachments);
        }
    }

    attachments
}

/// Finds the index of the deepest [`NifScene`] with the given attachment name.
///
/// If multiple independent branches have the same attachment, which leaf node is returned is
/// not specified.
fn find_deepest_scene_with_attachment(
    models: &[Handle<NifScene>],
    parents: &HashMap<usize, usize>,
    name: &str,
) -> Option<usize> {
    // Mark all scenes that contain a node with the given name.
    let mut models_with_name = Vec::new();
    for (index, model) in models.iter().enumerate() {
        if model
            .nodes
            .values()
            .flat_map(|node| &node.attachments)
            .find(|attachment| attachment.name == name)
            .is_none()
        {
            continue;
        }

        models_with_name.push(index);
    }

    // For each scene:
    // Walk up the hierarchy and remove all parent nodes
    // from the set of viable nodes.
    // This leaves the set of leaf nodes remaining.
    let mut remaining = models_with_name.len();
    while remaining != 0 {
        let index = models_with_name.pop().unwrap();

        let mut current = index;
        while let Some(parent) = parents.get(&current) {
            if models_with_name.contains(parent) {
                models_with_name.retain(|v| v != parent);
                remaining -= 1;
            }

            current = *parent;
        }

        models_with_name.push(index);
        remaining -= 1;
    }

    // This may contain more than one node, however in that
    // case we are allowed to return any node.
    models_with_name.pop()
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

fn extract_biped_object_flags(slots: BipedObjectFlags, dst: &mut HashSet<u32>) {
    let bits: u32 = slots.bits();
    for index in 0..u32::BITS {
        let bit = bits & (1 << index);
        if bit != 0 {
            dst.insert(bit);
        }
    }
}

async fn load_item_script(record: RecordId, loader: &ScriptLoader) -> Option<ScriptObject> {
    loader.load_instance(record).await
}

#[derive(Clone, Debug, Error)]
enum ObjectModApplicationError {
    #[error("unapplicable function type {ty:?} for property {property:?}")]
    UnapplicableFunctionType {
        property: ObjectModificationPropertyType,
        ty: ObjectModificationFunctionType,
    },
}

fn apply_object_modification(
    esp_loader: &EspLoader,
    omod_id: RecordId,
    base: &EquipmentProperties,
    combined: &mut EquipmentProperties,
    property: &ObjectModificationProperty,
) -> Result<(), ObjectModApplicationError> {
    enum ModificationClass<'combined> {
        ScalarF32 {
            base: f32,
            combined: &'combined mut f32,
            value: f32,
        },
        ScalarU32 {
            base: u32,
            combined: &'combined mut u32,
            value: u32,
        },
    }

    let class = match property.ty {
        ObjectModificationPropertyType::Keywords(keyword) => {
            let keyword = esp_loader.resolve_form(keyword, omod_id).unwrap();

            match property.function_type {
                ObjectModificationFunctionType::Add => {
                    combined.keywords.insert(keyword);
                }
                ObjectModificationFunctionType::Rem => {
                    combined.keywords.remove(&keyword);
                }
                _ => {
                    return Err(ObjectModApplicationError::UnapplicableFunctionType {
                        property: property.ty,
                        ty: property.function_type,
                    });
                }
            }

            return Ok(());
        }
        ObjectModificationPropertyType::Weight(weight) => ModificationClass::ScalarF32 {
            base: base.weight,
            combined: &mut combined.weight,
            value: weight,
        },
        ObjectModificationPropertyType::Value(value) => ModificationClass::ScalarF32 {
            base: base.value,
            combined: &mut combined.value,
            value,
        },
        // "Rating" actually is physical damage resistance.
        ObjectModificationPropertyType::Rating(rating) => {
            let base = base
                .resistances
                .get(&DAMAGE_TYPE_PHYSICAL)
                .copied()
                .unwrap_or_default();
            let combined = combined
                .resistances
                .entry(DAMAGE_TYPE_PHYSICAL)
                .or_default();

            ModificationClass::ScalarU32 {
                base,
                combined,
                value: rating,
            }
        }
        ObjectModificationPropertyType::DamageTypeValue(dmg_type, value) => {
            let dmg_type = esp_loader.resolve_form(dmg_type, omod_id).unwrap();

            let base = base.resistances.get(&dmg_type).copied().unwrap_or_default();
            let combined = combined.resistances.entry(dmg_type).or_default();

            ModificationClass::ScalarU32 {
                base,
                combined,
                value,
            }
        }
        ObjectModificationPropertyType::Ammo(_) => {
            todo!();
        }
        ObjectModificationPropertyType::NpcsUseAmmo(use_ammo) => {
            combined.weapon_npcs_use_ammo = use_ammo;
            return Ok(());
        }
        ObjectModificationPropertyType::AimModelMinConeDegrees(value) => {
            ModificationClass::ScalarF32 {
                base: base.weapon_cone_of_fire_min_angle,
                combined: &mut combined.weapon_cone_of_fire_min_angle,
                value,
            }
        }
        ObjectModificationPropertyType::AimModelMaxConeDegrees(value) => {
            ModificationClass::ScalarF32 {
                base: base.weapon_cone_of_fire_max_angle,
                combined: &mut combined.weapon_cone_of_fire_max_angle,
                value,
            }
        }
        ObjectModificationPropertyType::AimModelConeIncreasePerShot(value) => {
            ModificationClass::ScalarF32 {
                base: base.weapon_cone_of_fire_increase_per_shot,
                combined: &mut combined.weapon_cone_of_fire_increase_per_shot,
                value,
            }
        }
        ObjectModificationPropertyType::AimModelConeDecreasePerSec(value) => {
            ModificationClass::ScalarF32 {
                base: base.weapon_cone_of_fire_decrease_per_second,
                combined: &mut combined.weapon_cone_of_fire_decrease_per_second,
                value,
            }
        }
        ObjectModificationPropertyType::AimModelConeDecreaseDelayMs(value) => {
            ModificationClass::ScalarU32 {
                base: base.weapon_cone_of_fire_decrease_delay_milliseconds,
                combined: &mut combined.weapon_cone_of_fire_decrease_delay_milliseconds,
                value,
            }
        }
        _ => return Ok(()),
    };

    match class {
        ModificationClass::ScalarF32 {
            base,
            combined,
            value,
        } => match property.function_type {
            ObjectModificationFunctionType::Set => *combined = value,
            ObjectModificationFunctionType::Add => *combined += value,
            ObjectModificationFunctionType::MulAdd => *combined += base * value,
            _ => {
                return Err(ObjectModApplicationError::UnapplicableFunctionType {
                    property: property.ty,
                    ty: property.function_type,
                });
            }
        },
        ModificationClass::ScalarU32 {
            base,
            combined,
            value,
        } => match property.function_type {
            ObjectModificationFunctionType::Set => *combined = value,
            ObjectModificationFunctionType::Add => *combined += value,
            ObjectModificationFunctionType::MulAdd => *combined += base * value,
            _ => {
                return Err(ObjectModApplicationError::UnapplicableFunctionType {
                    property: property.ty,
                    ty: property.function_type,
                });
            }
        },
    }

    Ok(())
}
