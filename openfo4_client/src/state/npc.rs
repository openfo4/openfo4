use std::collections::VecDeque;
use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{Class, NpcBaseFlags, NpcTemplateFlags, Race, SubgraphFlags};
use openfo4_esp::{RecordData, RecordKind, ZString};
use openfo4_physics::components::{Collider, CollisionLayer, RigidBody};

use crate::assets::nif::{LoadNifFlags, NifKey, NifScene};
use crate::components::{ActorProperties, ActorProtection, DeadActor};
use crate::state::animation_test::{ActorCharacter, BodyPartBoneProps, load_character};
use crate::state::esp::EspLoader;
use crate::state::hardcoded::KEYWORD_FEMALE;
use crate::state::level::leveled_list::select_from_leveled_list;
use crate::systems::FacingDirection;
use crate::systems::actor_value::{ActorValue, ActorValueProperties};
use crate::systems::controller::ControllerShape;
use crate::systems::equipment::{Equipment, equip_item};
use crate::systems::inventory::{AddItemEventAsync, Inventory};

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

/// Keywords for ObjectReferences.
#[derive(Clone, Debug, Default)]
pub struct ObjectKeywords {
    keywords: HashSet<RecordId>,
    /// Dynamically added keywords.
    added: HashSet<RecordId>,
    /// Dynamically removed keywords.
    removed: HashSet<RecordId>,
}

impl ObjectKeywords {
    pub fn new(keywords: HashSet<RecordId>) -> Self {
        Self {
            keywords,
            added: HashSet::new(),
            removed: HashSet::new(),
        }
    }

    pub fn contains(&self, keyword: RecordId) -> bool {
        self.keywords.contains(&keyword)
    }

    pub fn add(&mut self, keyword: RecordId) {
        if self.keywords.contains(&keyword) {
            return;
        }

        self.keywords.insert(keyword);
        self.added.insert(keyword);
        self.removed.remove(&keyword);
    }

    pub fn remove(&mut self, keyword: RecordId) {
        if !self.keywords.contains(&keyword) {
            return;
        }

        self.keywords.remove(&keyword);
        self.added.remove(&keyword);
        self.removed.insert(keyword);
    }

    /// Resets the dynamically added/removed keyword to the default state.
    pub fn reset(&mut self, keyword: RecordId) {
        // Keyword was either removed or added, but not both.
        debug_assert!(!(self.added.contains(&keyword) && self.removed.contains(&keyword)));

        if self.removed.remove(&keyword) {
            self.keywords.insert(keyword);
        } else if self.added.remove(&keyword) {
            self.keywords.remove(&keyword);
        }
    }
}

/// A body part of an actor.
#[derive(Copy, Clone, Debug)]
pub struct BodyPartBone {
    /// The actor that this body part is attached to.
    pub actor: Entity,
    /// The unique ID of the body part.
    pub id: u32,
}

#[derive(Clone, Debug)]
pub struct ActorAnimationGraphs(pub Vec<ActorAnimationGraph>);

#[derive(Clone, Debug)]
pub struct ActorBodyParts(Vec<ActorBodyPart>);

impl ActorBodyParts {
    pub fn get(&self, id: u32) -> Option<&ActorBodyPart> {
        self.0.get(id as usize)
    }
}

#[derive(Clone, Debug)]
pub struct ActorBodyPart {
    /// The actor value of the body part local health condition.
    pub condition: RecordId,
    /// Factor applied to incoming damage hitting this body part.
    pub damage_factor: f32,
}

#[derive(Clone, Debug)]
pub struct Actor {
    anim_graphs: ActorAnimationGraphData,
    character: ActorCharacter,
    controller: ControllerShape,
    skin: Option<RecordId>,
    properties: ActorValueProperties,
    body_parts: ActorBodyParts,
    dead: bool,
    customization_scenes: Vec<Handle<NifScene>>,
    pub keywords: HashSet<RecordId>,
    outfit_items: Vec<RecordId>,
    protection: ActorProtection,
    level: u32,
}

impl Actor {
    pub fn instantiate(self, world: &mut World, events: &mut EventBus) -> Entity {
        let collider = world.spawn((
            self.controller.offset,
            Collider {
                friction: 1.0,
                restitution: 1.0,
                shape: self.controller.shape.clone(),
                layer: CollisionLayer::CHARACTER_CONTROLLER,
            },
        ));

        let root = world.reserve_entity();
        let spawned_actor = self.character.instantiate(root, world);
        let instance = spawned_actor.skeleton;

        if let Some(skin) = self.skin {
            events.send(AddItemEventAsync {
                actor: root,
                item: skin,
                attachments: Vec::new(),
                hidden: true,
                count: 1,
                equip: true,
            });
        }

        let mut equipment = Equipment::default();

        for scene in self.customization_scenes {
            equip_item(root, &scene, &instance, world, &mut equipment, None);
        }

        for item in self.outfit_items {
            events.send(AddItemEventAsync {
                actor: root,
                item,
                attachments: Vec::new(),
                hidden: false,
                count: 1,
                equip: true,
            });
        }

        world
            .insert(
                root,
                (
                    Inventory::default(),
                    equipment,
                    self.anim_graphs,
                    RigidBody::FIXED,
                    self.controller,
                    instance,
                    self.properties,
                    FacingDirection::default(),
                    self.body_parts,
                    ActorProperties {
                        protection: self.protection,
                        level: self.level,
                        bleedout_recovery: true,
                    },
                ),
            )
            .unwrap();
        world.add_children(root, collider);

        if self.dead {
            world.insert_one(root, DeadActor).unwrap();
        }

        root
    }
}

/// Actor properties defined by an instantiation.
#[derive(Clone, Debug)]
pub struct ActorInstanceProps {
    /// Whether the actor is dead.
    pub dead: bool,
}

pub async fn spawn_actor(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    npc_form: RecordId,
    instance_props: &ActorInstanceProps,
) -> Result<Actor, ()> {
    let props = resolve_actor_props(&loader, esp_loader, npc_form).await;

    let mut properties = props.properties;

    if let Some(class) = props.class
        && !class.form.is_null()
    {
        let class = esp_loader
            .lookup(Some(RecordKind::Class), class)
            .await
            .unwrap();
        let class_data = &class.local.data.as_variant::<Class>().unwrap();

        for prop in &*class_data.properties {
            let actor_value = esp_loader.resolve_form(prop.actor_value, class.id).unwrap();

            let old_value = properties.get(actor_value).unwrap_or_default();
            properties.set(actor_value, ActorValue::new(old_value.value + prop.value));
        }
    }

    let race = props.race.unwrap();
    let race = esp_loader
        .lookup(Some(RecordKind::Race), race)
        .await
        .unwrap();
    let data = match &race.local.data {
        RecordData::Race(data) => data,
        _ => unreachable!(),
    };

    let mut character = load_character(data, props.female, &loader).await.unwrap();

    let mut actor_body_parts = Vec::new();
    if let Some(body_part_form) = data.body_part_data {
        let body_part_form = esp_loader.resolve_form(body_part_form, race.id).unwrap();

        let mut body_parts = Vec::new();

        let body_part_data_record = esp_loader
            .lookup(Some(RecordKind::BodyPartData), body_part_form)
            .await
            .unwrap();

        let RecordData::BodyPartData(body_part_data) = body_part_data_record.local.data else {
            unreachable!();
        };

        for (id, part) in body_part_data.parts.iter().enumerate() {
            body_parts.push(BodyPartBoneProps {
                id: id as u32,
                start_bone: part.part_node.as_str().unwrap().to_owned(),
            });

            let condition = esp_loader
                .resolve_form(part.data.actor_value, body_part_data_record.id)
                .unwrap();

            actor_body_parts.push(ActorBodyPart {
                condition,
                damage_factor: part.data.damage_mult,
            });
        }

        character.body_parts = body_parts;
    }

    let mut subgraphs = Vec::new();
    for subgraph in load_subgraphs(esp_loader, props.race.unwrap(), data).await {
        let search_paths = subgraph
            .animation_paths
            .iter()
            .map(|v| DataPathBuf::new(v.as_bytes()))
            .collect::<Vec<_>>();

        subgraphs.push(ActorAnimationGraph {
            // graph: Arc::new(behavior.graph),
            // animation: behavior.clips,
            behavior: DataPathBuf::new(subgraph.behavior_graph.as_bytes()),
            actor_keywords: subgraph.actor_keywords.clone(),
            target_keywords: subgraph.target_keywords.clone(),
            flags: subgraph.flags,
            search_paths,
        });
    }

    let skin = props.skin.or_else(|| {
        data.skin
            .map(|form| esp_loader.resolve_form(form, race.id).unwrap())
    });

    let mut keywords: HashSet<RecordId> = props.keywords.into_iter().collect();
    keywords.extend(
        data.keywords
            .iter()
            .flatten()
            .map(|form| esp_loader.resolve_form(*form, race.id).unwrap()),
    );

    let customization_scenes = load_actor_customization(loader, esp_loader, npc_form).await;

    let outfit_items = if let Some(outfit) = props.outfit {
        load_outfit(esp_loader, outfit).await
    } else {
        Vec::new()
    };

    let level = match props.level {
        ActorLevel::Fixed(level) => u32::from(level),
        ActorLevel::PlayerScaled { factor, min, max } => {
            let level = u32::from(PLAYER_LEVEL).saturating_mul(u32::from(factor));
            u32::clamp(level, u32::from(min), u32::from(max))
        }
    };

    Ok(Actor {
        anim_graphs: ActorAnimationGraphData {
            character: character.clone(),
            root_path: character.root_path.clone(),
            root_behavior_path: DataPathBuf::new(&*character.behavior_filename),
            subgraphs,
        },
        controller: ControllerShape {
            offset: character.controller_offset,
            shape: character.controller_shape.clone(),
        },
        character,
        skin,
        properties,
        keywords,
        body_parts: ActorBodyParts(actor_body_parts),
        dead: instance_props.dead,
        customization_scenes,
        outfit_items,
        protection: props.protection,
        level,
    })
}

async fn resolve_actor_props(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    npc_form: RecordId,
) -> ActorProps {
    let npc = esp_loader
        .lookup(Some(RecordKind::Npc), npc_form)
        .await
        .unwrap();

    // Base Stats
    let mut essential = false;
    let mut protected = false;
    let mut respawn = false;

    // Traits
    let mut race = None;
    let mut skin = None;
    let mut female = false;

    // Stats
    let mut properties = ActorValueProperties::default();
    let mut level = ActorLevel::default();
    let mut class = None;

    // Factions
    let mut factions = Vec::new();

    let mut keywords = Vec::new();

    // Inventory
    let mut outfit = None;

    let mut queue = VecDeque::new();
    queue.push_back((NpcTemplateFlags::all(), npc));
    let mut visited = HashSet::new();

    while let Some((flags, record)) = queue.pop_front() {
        visited.insert(record.id);

        match record.local.data {
            RecordData::Npc(data) => {
                if flags.contains(NpcTemplateFlags::TRAITS) {
                    race = Some(esp_loader.resolve_form(data.race, record.id).unwrap());

                    if let Some(form) = data.skin {
                        skin = Some(esp_loader.resolve_form(form, record.id).unwrap());
                    }

                    female = data.base_stats.flags.contains(NpcBaseFlags::FEMALE);
                }

                if flags.contains(NpcTemplateFlags::STATS) {
                    properties.clear();
                    for prop in &*data.properties {
                        let actor_value = esp_loader
                            .resolve_form(prop.actor_value, record.id)
                            .unwrap();

                        let old_value = properties.get(actor_value).unwrap_or_default();
                        properties.set(actor_value, ActorValue::new(old_value.value + prop.value));
                    }

                    if let Some(form) = data.class {
                        class = Some(esp_loader.resolve_form(form, record.id).unwrap());
                    }

                    level = if data.base_stats.flags.contains(NpcBaseFlags::PC_LEVEL_MULT) {
                        let min = data.base_stats.calc_min_level;
                        let max = data.base_stats.calc_max_level;

                        // max == 0 means no limit.
                        let max = if max == 0 {
                            u16::MAX
                        } else {
                            // min > max is probably a mistake when the record was created.
                            // In this case we will use the min value and the level will become fixed.
                            u16::max(min, max)
                        };

                        if min == max {
                            ActorLevel::Fixed(min)
                        } else {
                            ActorLevel::PlayerScaled {
                                factor: data.base_stats.level as u16,
                                min,
                                max,
                            }
                        }
                    } else {
                        ActorLevel::Fixed(data.base_stats.level as u16)
                    };
                }

                if flags.contains(NpcTemplateFlags::FACTIONS) {
                    factions = data
                        .factions
                        .iter()
                        .map(|membership| {
                            let faction = esp_loader
                                .resolve_form(membership.faction, record.id)
                                .unwrap();

                            FactionMembership {
                                faction,
                                rank: membership.rank,
                            };
                        })
                        .collect();
                }

                if flags.contains(NpcTemplateFlags::BASE_DATA) {
                    // FIXME: Set Name
                    essential = data.base_stats.flags.contains(NpcBaseFlags::ESSENTIAL);
                    protected = data.base_stats.flags.contains(NpcBaseFlags::PROTECTED);
                    respawn = data.base_stats.flags.contains(NpcBaseFlags::RESPAWN);
                }

                if flags.contains(NpcTemplateFlags::KEYWORDS) {
                    keywords = data
                        .keywords
                        .iter()
                        .map(|v| v.iter())
                        .flatten()
                        .copied()
                        .map(|form| esp_loader.resolve_form(form, record.id).unwrap())
                        .collect();
                }

                if flags.contains(NpcTemplateFlags::INVENTORY) {
                    outfit = data
                        .default_outfit
                        .map(|form| esp_loader.resolve_form(form, record.id).unwrap());
                }

                if let Some(template) = data.template_actors {
                    // Leveled template actor refer to the same actor for multiple
                    // categories. It is important that we select the same one for
                    // all of them, otherwise we may end up with inconsistent properties
                    // if the next actor in the chain is randomly decided (via a leveled npc
                    // form).
                    let mut template_forms = Vec::new();

                    for (flags, form_id) in [
                        (NpcTemplateFlags::TRAITS, template.traits),
                        (NpcTemplateFlags::STATS, template.stats),
                        (NpcTemplateFlags::FACTIONS, template.factions),
                        (NpcTemplateFlags::SPELL_LIST, template.spell_list),
                        (NpcTemplateFlags::AI_DATA, template.ai_data),
                        (NpcTemplateFlags::AI_PACKAGES, template.ai_packages),
                        (NpcTemplateFlags::MODEL, template.model),
                        (NpcTemplateFlags::BASE_DATA, template.base_data),
                        (NpcTemplateFlags::INVENTORY, template.inventory),
                        (NpcTemplateFlags::SCRIPT, template.script),
                        (NpcTemplateFlags::DEF_PACK_LIST, template.def_pack_list),
                        (NpcTemplateFlags::ATTACK_DATA, template.attack_data),
                        (NpcTemplateFlags::KEYWORDS, template.keywords),
                    ] {
                        if !data.base_stats.template_flags.contains(flags) {
                            continue;
                        }

                        let record_id = esp_loader.resolve_form(form_id, record.id).unwrap();

                        if let Some((f, _)) = template_forms
                            .iter_mut()
                            .find(|(_, form)| *form == record_id)
                        {
                            *f |= flags;
                        } else {
                            template_forms.push((flags, record_id));
                        }
                    }

                    for (flags, form) in template_forms {
                        let npc = esp_loader.lookup(None, form).await.unwrap();
                        queue.push_back((flags, npc));
                    }
                }
            }
            RecordData::LeveledActor(mut data) => {
                // Discard any entries that we have already seen, otherwise
                // we might get a cycle in the reference graph.
                data.entries.retain(|entry| {
                    esp_loader
                        .resolve_form(entry.reference, record.id)
                        .is_some_and(|id| !visited.contains(&id))
                });

                let Some(index) = select_from_leveled_list(data.flags, &data.entries, PLAYER_LEVEL)
                else {
                    continue;
                };

                let form = data.entries[index].reference;
                let record_id = esp_loader.resolve_form(form, record.id).unwrap();
                let record = esp_loader.lookup(None, record_id).await.unwrap();
                queue.push_back((flags, record));
            }
            _ => unreachable!(),
        }
    }

    // Special keyword that is present on all female actors.
    if female && !keywords.contains(&KEYWORD_FEMALE) {
        keywords.push(KEYWORD_FEMALE);
    }

    let protection = if essential {
        ActorProtection::Essential
    } else if protected {
        ActorProtection::Protected
    } else {
        ActorProtection::None
    };

    ActorProps {
        race,
        skin,
        female,
        class,
        properties,
        keywords,
        outfit,
        protection,
        level,
    }
}

#[derive(Clone, Debug)]
struct ActorProps {
    race: Option<RecordId>,
    /// Skin overwrite
    skin: Option<RecordId>,
    female: bool,
    class: Option<RecordId>,
    properties: ActorValueProperties,
    keywords: Vec<RecordId>,
    outfit: Option<RecordId>,
    protection: ActorProtection,
    level: ActorLevel,
}

#[derive(Clone, Debug)]
pub struct FactionMemberships(pub Vec<FactionMembership>);

#[derive(Copy, Clone, Debug)]
pub struct FactionMembership {
    pub faction: RecordId,
    pub rank: i8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ActorLevel {
    Fixed(u16),
    PlayerScaled { factor: u16, min: u16, max: u16 },
}

impl Default for ActorLevel {
    fn default() -> Self {
        Self::Fixed(0)
    }
}

#[derive(Clone, Debug)]
pub struct BodyParts(HashMap<Entity, BodyPart>);

#[derive(Clone, Debug)]
pub struct BodyPart {
    pub condition: RecordId,
    pub damage_factor: f32,
}

async fn load_subgraphs(
    esp_loader: &EspLoader,
    race_form: RecordId,
    race_data: &Race,
) -> Vec<RaceSubgraphData> {
    // If we have a template race all subgraph data from the referenced race
    // is added to our race.
    let template_race = race_data
        .subgraph_template_race
        .map(|form| esp_loader.resolve_form(form, race_form).unwrap());

    let mut subgraphs = Vec::new();
    for record in esp_loader.lookup_all(Some(RecordKind::Race)).await.unwrap() {
        let Some(data) = record.local.data.into_variant::<Race>() else {
            continue;
        };

        let is_original_race = record.id == race_form;
        let is_template_race = template_race.is_some_and(|id| id == record.id);
        let is_additive_race = data.subgraph_additive_race.is_some_and(|id| {
            let id = esp_loader.resolve_form(id, record.id).unwrap();
            id == record.id || Some(id) == template_race
        });

        if !is_original_race && !is_template_race && !is_additive_race {
            continue;
        }

        for data in data.subgraph_data.into_iter() {
            let map_form = |form| esp_loader.resolve_form(form, record.id).unwrap();

            subgraphs.push(RaceSubgraphData {
                actor_keywords: data.actor_keywords.into_iter().map(map_form).collect(),
                target_keywords: data.target_keywords.into_iter().map(map_form).collect(),
                behavior_graph: data.behavior_graph,
                animation_paths: data.animation_paths,
                flags: data.flags,
            });
        }
    }

    subgraphs
}

#[derive(Clone, Debug)]
pub struct RaceSubgraphData {
    pub actor_keywords: Vec<RecordId>,
    pub target_keywords: Vec<RecordId>,
    pub behavior_graph: ZString,
    pub animation_paths: Vec<ZString>,
    pub flags: SubgraphFlags,
}

#[derive(Clone, Debug)]
pub struct ActorAnimationGraphData {
    pub character: ActorCharacter,
    pub root_path: DataPathBuf,
    pub root_behavior_path: DataPathBuf,
    pub subgraphs: Vec<ActorAnimationGraph>,
}

#[derive(Clone, Debug)]
pub struct ActorAnimationGraph {
    // pub graph: Arc<AnimationGraph>,
    pub behavior: DataPathBuf,
    // pub animation: AnimationSet,
    pub actor_keywords: Vec<RecordId>,
    pub target_keywords: Vec<RecordId>,
    pub flags: SubgraphFlags,
    pub search_paths: Vec<DataPathBuf>,
}

async fn load_actor_customization(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    npc_form: RecordId,
) -> Vec<Handle<NifScene>> {
    let RecordData::Npc(npc) = esp_loader
        .lookup(Some(RecordKind::Npc), npc_form)
        .await
        .unwrap()
        .local
        .data
    else {
        unreachable!()
    };

    let mut queue = Vec::new();
    for form in &npc.head_parts {
        let id = esp_loader.resolve_form(*form, npc_form).unwrap();
        queue.push(id);
    }

    let mut scenes = Vec::new();

    while let Some(id) = queue.pop() {
        let part_record = esp_loader
            .lookup(Some(RecordKind::HeadPart), id)
            .await
            .unwrap();

        let RecordData::HeadPart(part) = part_record.local.data else {
            unreachable!();
        };

        for form in &part.extra_parts {
            let id = esp_loader.resolve_form(*form, part_record.id).unwrap();
            queue.push(id);
        }

        let Some(path) = &part.model.model else {
            continue;
        };

        let path = DataPathBuf::new(path.as_bytes());

        let scene = loader
            .load::<NifScene, _>(&NifKey {
                path,
                flags: LoadNifFlags::all(),
            })
            .await
            .unwrap();

        scenes.push(scene);
    }

    scenes
}

async fn load_outfit(esp_loader: &EspLoader, outfit: RecordId) -> Vec<RecordId> {
    let mut items = Vec::new();

    let outfit = esp_loader
        .lookup(Some(RecordKind::Outfit), outfit)
        .await
        .unwrap();

    let RecordData::Outfit(data) = outfit.local.data else {
        unreachable!();
    };

    for item in &*data.items {
        let item = esp_loader.resolve_form(*item, outfit.id).unwrap();
        items.push(item);
    }

    items
}
