use std::collections::VecDeque;

use hashbrown::{HashMap, HashSet};
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_common::components::Transform;
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::hierarchy::Children;
use slab::Slab;

use crate::assets::nif::{LoadNifFlags, NifScene};
use crate::components::RootParent;
use crate::state::animation_test::SkinInstance;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EquipmentKey(usize);

#[derive(Clone, Debug, Default)]
pub struct Equipment {
    items: Slab<EquippedItem>,
}

impl Equipment {
    pub fn insert(&mut self, item: EquippedItem) -> EquipmentKey {
        EquipmentKey(self.items.insert(item))
    }

    fn remove(&mut self, key: EquipmentKey) -> Option<EquippedItem> {
        if self.items.contains(key.0) {
            Some(self.items.remove(key.0))
        } else {
            None
        }
    }

    pub fn get(&self, key: EquipmentKey) -> Option<&EquippedItem> {
        self.items.get(key.0)
    }

    pub fn get_mut(&mut self, key: EquipmentKey) -> Option<&mut EquippedItem> {
        self.items.get_mut(key.0)
    }

    pub fn items(&self) -> impl Iterator<Item = &EquippedItem> {
        self.items.iter().map(|(_, v)| v)
    }
}

#[derive(Clone, Debug)]
pub struct EquippedItem {
    /// All attached entities, including skinned entities.
    entities: Vec<Entity>,
    attachments: HashMap<String, AttachmentPoint>,
    skins: Vec<SkinInstance>,
}

impl EquippedItem {
    pub fn skins(&self) -> &[SkinInstance] {
        &self.skins
    }

    pub fn attachment(&self, name: &str) -> Option<&AttachmentPoint> {
        self.attachments.get(name)
    }
}

#[derive(Clone, Debug)]
pub struct AttachmentPoint {
    /// Entity of the node where to attach to.
    pub entity: Entity,
    pub offset: Transform,
    /// References to other [`EquippedItem`]s that are attached currently.
    pub attachments: Vec<EquipmentKey>,
}

pub fn equip_item(
    actor_root: Entity,
    scene: &NifScene,
    skeleton: &SkeletonInstance,
    world: &mut World,
    equipment: &mut Equipment,
    parent: Option<EquipmentKey>,
) -> EquipmentKey {
    let mut skins = Vec::new();
    let mut entities = Vec::new();
    let mut attachments = HashMap::new();
    let mut attachment_parents = Vec::new();

    let mut skip_nodes = HashSet::new();

    for (key, node) in scene.nodes.iter_topo_sorted() {
        if skip_nodes.contains(&key) {
            continue;
        }

        match &node.skin {
            Some(skin) => {
                let Some(object) = &node.renderable else {
                    continue;
                };

                let mut bone_indices = Vec::new();
                for name in &skin.bones {
                    let Some(bone) = skeleton
                        .skeleton
                        .bones
                        .iter()
                        .find(|b| b.name.to_lowercase() == name.to_lowercase())
                    else {
                        tracing::warn!(
                            "cannot map skinned bone {}; no such bone in skeleton",
                            name
                        );
                        continue;
                    };

                    bone_indices.push(bone.index);
                }

                let entity = world.spawn((Transform::default(), object.clone()));
                // Add the skinned component as child so that it gets despawned with the actor.
                world.add_children(actor_root, entity);

                entities.push(entity);

                skins.push(SkinInstance {
                    entity,
                    bones: bone_indices,
                });
            }
            None => {
                // If the node matches the name of a bone we will attach it there.
                // This includes all children of the node, unless they match the
                // name of another bone, in which case they will get attached there
                // instead.
                // If a node matches both a skeleton bone and an attachment point
                // the skeleton bone should be preferred.
                let root = match skeleton
                    .skeleton
                    .bones
                    .iter()
                    .find(|b| b.name.to_lowercase() == node.attaches.to_lowercase())
                {
                    Some(bone) => {
                        let bone_entity = *skeleton.bones.get(&bone.index).unwrap();
                        bone_entity
                    }
                    None => {
                        if let (Some(attaches), Some(parent_key)) = (&scene.attaches, parent) {
                            let Some(parent) = equipment.get(parent_key) else {
                                continue;
                            };

                            let Some(point) = parent.attachments.get(attaches) else {
                                continue;
                            };

                            attachment_parents.push((parent_key, attaches));

                            // If we have no offset we can directly attach to the parent
                            // scene node, otherwise we have to apply the offset transform
                            // first.
                            if point.offset == Transform::IDENTITY {
                                point.entity
                            } else {
                                let root = world.spawn((point.offset,));
                                world.add_children(point.entity, root);
                                entities.push(root);
                                root
                            }
                        } else {
                            // Mount at the root entity instead.
                            actor_root
                        }
                    }
                };

                let mut queue = VecDeque::new();
                queue.push_back((root, key));

                while let Some((parent, key)) = queue.pop_front() {
                    let node = scene.nodes.get(key).unwrap();
                    let entity = world.spawn(node.build(LoadNifFlags::GRAPHICS).build());

                    for attachment in &node.attachments {
                        attachments.insert(
                            attachment.name.clone(),
                            AttachmentPoint {
                                entity,
                                offset: attachment.offset,
                                attachments: Vec::new(),
                            },
                        );
                    }

                    // If the node is root, i.e. matches the bone name the transform
                    // of that node should be overwritten by the bone transform.
                    if parent == root {
                        world.insert_one(entity, Transform::default()).unwrap();
                    }

                    world.add_children(parent, entity);

                    for (child_key, child_node) in scene.nodes.children(key).into_iter().flatten() {
                        // Skip the node if it matches the name of another bone.
                        let has_other_bone_name = skeleton
                            .skeleton
                            .bones
                            .iter()
                            .any(|b| b.name.to_lowercase() == child_node.name.to_lowercase());

                        if has_other_bone_name || child_node.skin.is_some() {
                            continue;
                        }

                        skip_nodes.insert(child_key);
                        queue.push_back((entity, child_key));
                    }

                    entities.push(entity);
                }
            }
        }
    }

    for entity in &entities {
        world.insert_one(*entity, RootParent(actor_root)).unwrap();
    }

    let key = equipment.insert(EquippedItem {
        entities,
        attachments,
        skins,
    });

    for (parent, name) in attachment_parents {
        let parent = equipment.get_mut(parent).unwrap();
        let point = parent.attachments.get_mut(name).unwrap();
        point.attachments.push(key);
    }

    key
}

pub fn unequip_item(actor_root: Entity, mut keys: Vec<EquipmentKey>, world: &mut World) {
    let Ok(mut equipment) = world.get::<&mut Equipment>(actor_root) else {
        return;
    };

    let mut removed_entities = HashSet::<Entity>::new();

    while let Some(key) = keys.pop() {
        let Some(item) = equipment.remove(key) else {
            continue;
        };

        removed_entities.extend(&item.entities);

        for attachment in item.attachments.values() {
            debug_assert!(removed_entities.contains(&attachment.entity));
            keys.extend(&attachment.attachments);
        }
    }

    // Walk the entire entity hierarchy starting with the actor root
    // and cull all entities from Children that we just removed.
    // FIXME: In the future maybe just remeber where we attached
    // so we don't have to walk the entire tree.
    let mut queue = vec![actor_root];
    while let Some(entity) = queue.pop() {
        if let Ok(mut children) = world.get::<&mut Children>(entity) {
            children.0.retain(|v| !removed_entities.contains(v));
            queue.extend(&children.0);
        }
    }

    drop(equipment);

    // Despawn all removed entities.
    // Note that this will include all recursive children, so
    // `despawn_recursive` is not needed here.
    for entity in removed_entities {
        world.despawn(entity).unwrap();
    }
}
