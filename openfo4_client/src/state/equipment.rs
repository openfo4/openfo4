use std::collections::VecDeque;

use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_asset::{AssetLoader, Handle};
use openfo4_esp::{FormId, RecordData, RecordKind};

use crate::assets::nif_v2::NifScene;
use crate::state::esp::EspLoader;
use crate::systems::equipment::{Equipment, EquipmentKey, equip_item};

#[derive(Clone, Debug)]
pub struct EquipmentItem {
    pub models: Vec<Handle<NifScene>>,
    pub attachment_parents: HashMap<usize, usize>,
    pub resistances: HashMap<FormId, u32>,
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
    let mut resistances = HashMap::new();

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

            for resistance in armor.resistances.iter().flatten() {
                *resistances.entry(resistance.damage_type).or_default() += resistance.value;
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

    EquipmentItem {
        models,
        resistances,
        attachment_parents,
    }
}
