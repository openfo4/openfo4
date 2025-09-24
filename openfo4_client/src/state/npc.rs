use std::collections::VecDeque;
use std::num::NonZeroU16;
use std::sync::Arc;

use bitflags::bitflags;
use futures_lite::FutureExt;
use hecs::{Entity, World};
use openfo4_animation::AnimationSet;
use openfo4_animation::animation::AnimationClip;
use openfo4_animation::driver::AnimationDriver;
use openfo4_animation::skeleton::SkeletonInstance;
use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;
use openfo4_esp::records::{LeveledActorFlags, NpcBaseFlags, NpcTemplateFlags};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_physics::components::{Collider, RigidBody};
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;

use crate::assets::nif_v2::NifScene;
use crate::state::animation_test::{ActorCharacter, load_behavior_data, load_character};
use crate::state::equipment::{EquipmentItem, load_armor};
use crate::state::esp::EspLoader;
use crate::systems::controller::ControllerShape;
use crate::systems::equipment::{Equipment, EquippedItem, equip_item};

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

#[derive(Debug)]
pub struct Actor {
    character: ActorCharacter,
    driver: AnimationDriver,
    clips: AnimationSet,
    controller: ControllerShape,
    skin: Option<EquipmentItem>,
}

impl Actor {
    pub fn instantiate(self, world: &mut World) -> Entity {
        let collider = world.spawn((
            self.controller.offset,
            Collider {
                friction: 1.0,
                restitution: 1.0,
                shape: self.controller.shape.clone(),
            },
        ));

        let root = world.reserve_entity();
        let instance = self.character.instantiate(root, world);

        let mut equipment = Equipment::default();
        if let Some(skin) = self.skin {
            for scene in &skin.models {
                equip_item(&scene, &instance, world, &mut equipment, None);
            }
        }

        world
            .insert(
                root,
                (
                    self.clips,
                    self.driver,
                    RigidBody::FIXED,
                    self.controller,
                    instance,
                    equipment,
                ),
            )
            .unwrap();
        world.add_children(root, collider);

        root
    }
}

pub async fn spawn_actor(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    npc_form: FormId,
) -> Result<Actor, ()> {
    let props = resolve_actor_props(&loader, esp_loader, npc_form).await;

    let race = esp_loader
        .lookup(Some(RecordKind::Race), props.race)
        .await
        .unwrap();
    let data = match &race.data {
        RecordData::Race(data) => data,
        _ => unreachable!(),
    };

    let character = load_character(data, props.female, &loader).await.unwrap();

    let behavior_path = data.behavior_graph.as_ref().unwrap();
    let behavior_path = if props.female {
        behavior_path.0.female.as_ref().unwrap()
    } else {
        behavior_path.0.male.as_ref().unwrap()
    };

    let Some(subgraph) = data
        .subgraph_data
        .0
        .iter()
        .find(|v| v.target_keywords.is_empty())
    else {
        return Err(());
    };

    let search_paths = subgraph
        .animation_paths
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>();

    let behavior = load_behavior_data(
        &character.root_path,
        subgraph.behavior_graph.as_str().unwrap(),
        &character,
        &search_paths,
        loader,
    )
    .await
    .unwrap();

    let mut skin = None;
    if let Some(skin_form) = props.skin.or(data.skin) {
        skin = Some(load_armor(loader, esp_loader, skin_form, props.female).await);
    }

    Ok(Actor {
        driver: AnimationDriver::new(Arc::new(behavior.graph)),
        clips: behavior.clips,
        controller: ControllerShape {
            offset: character.controller_offset,
            shape: character.controller_shape.clone(),
        },
        character,
        skin,
    })
}

async fn resolve_actor_props(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    npc_form: FormId,
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
    let mut race = FormId::NULL;
    let mut skin = None;
    let mut female = false;

    // Stats
    let mut properties = Vec::new();
    let mut level = ActorLevel::default();
    let mut class = None;

    // Factions
    let mut factions = Vec::new();

    let mut keywords = Vec::new();

    let mut queue = VecDeque::new();
    queue.push_back((NpcTemplateFlags::all(), npc));

    while let Some((flags, record)) = queue.pop_front() {
        match &record.data {
            RecordData::Npc(data) => {
                if flags.contains(NpcTemplateFlags::TRAITS) {
                    race = data.race;
                    skin = data.skin;
                    female = data.base_stats.flags.contains(NpcBaseFlags::FEMALE);
                }

                if flags.contains(NpcTemplateFlags::STATS) {
                    properties = data.properties.clone();
                    class = data.class;
                    level = ActorLevel {
                        level: data.base_stats.level,
                        scales_with_player: data
                            .base_stats
                            .flags
                            .contains(NpcBaseFlags::PC_LEVEL_MULT),
                        min_level: NonZeroU16::new(data.base_stats.calc_min_level),
                        max_level: NonZeroU16::new(data.base_stats.calc_max_level),
                    };
                }

                if flags.contains(NpcTemplateFlags::FACTIONS) {
                    factions = data
                        .factions
                        .iter()
                        .map(|membership| FactionMembership {
                            faction: membership.faction,
                            rank: membership.rank,
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
                        .collect();
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

                        if let Some((f, _)) =
                            template_forms.iter_mut().find(|(_, form)| *form == form_id)
                        {
                            *f |= flags;
                        } else {
                            template_forms.push((flags, form_id));
                        }
                    }

                    for (flags, form) in template_forms {
                        let npc = esp_loader.lookup(None, form).await.unwrap();
                        queue.push_back((flags, npc));
                    }
                }
            }
            RecordData::LeveledActor(data) => {
                let mut weights = Vec::new();
                let mut indices = Vec::new();

                if data.flags.contains(LeveledActorFlags::ALL_LEVELS) {
                    for (index, entry) in data.entries.iter().enumerate() {
                        if entry.level > PLAYER_LEVEL {
                            continue;
                        }

                        let weight = 100 - u8::min(100, entry.chance_none);
                        if weight == 0 {
                            continue;
                        }

                        indices.push(index);
                        weights.push(u64::from(weight));
                    }
                } else {
                    let mut max_level = 0;

                    for (index, entry) in data.entries.iter().enumerate() {
                        if entry.level > PLAYER_LEVEL || entry.level < max_level {
                            continue;
                        }

                        // We found an entry with a higher level.
                        // All previous entries become invalid.
                        if entry.level != max_level {
                            weights.clear();
                            indices.clear();
                            max_level = entry.level;
                        }

                        let weight = 100 - u8::min(100, entry.chance_none);
                        if weight == 0 {
                            continue;
                        }

                        indices.push(index);
                        weights.push(u64::from(weight));
                    }
                }

                if weights.len() == 0 {
                    continue;
                }

                let distribution = WeightedIndex::new(weights).unwrap();
                let index = indices[distribution.sample(&mut rand::rng())];

                let form = data.entries[index].reference;
                let record = esp_loader.lookup(None, form).await.unwrap();
                queue.push_back((flags, record));
            }
            _ => unreachable!(),
        }
    }

    ActorProps { race, skin, female }
}

#[derive(Clone, Debug)]
struct ActorProps {
    race: FormId,
    /// Skin overwrite
    skin: Option<FormId>,
    female: bool,
}

#[derive(Clone, Debug)]
pub struct FactionMemberships(pub Vec<FactionMembership>);

#[derive(Copy, Clone, Debug)]
pub struct FactionMembership {
    pub faction: FormId,
    pub rank: i8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NpcProtection {
    None,
    Essential,
    Protected,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct ActorLevel {
    level: i16,
    scales_with_player: bool,
    min_level: Option<NonZeroU16>,
    max_level: Option<NonZeroU16>,
}
