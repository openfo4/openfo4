pub mod behavior;
pub mod character;
pub mod scene;
pub mod skeleton;

use std::borrow::Cow;
use std::collections::VecDeque;

use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{Npc, NpcBaseFlags, NpcTemplateFlags, Race, RaceDataFlags};
use openfo4_esp::{RecordData, RecordKind};

use crate::components::{
    ActorMarker, ActorProperties, ActorProtection, DeadActor, MovementState, Sex,
};
use crate::state::equipment::load_item;
use crate::state::esp::{EspLoader, GlobalRecord};
use crate::state::hardcoded::KEYWORD_FEMALE;
use crate::state::hardcoded::fallout4::{
    DEFAULT_MOVEMENT_TYPE_DEFAULT, DEFAULT_MOVEMENT_TYPE_SNEAK,
};
use crate::state::level::items::VISIBILITY_PRIORITY_SKIN;
use crate::state::level::leveled_list::select_from_leveled_list;
use crate::state::npc::ActorMovementSpeeds;
use crate::systems::FacingDirection;
use crate::systems::actor_value::{ActorValue, ActorValueProperties};
use crate::systems::equipment::Equipment;
use crate::systems::inventory::{
    AddItemEvent, AddItemEventAsync, EquipItemEvent, Inventory, ItemStack,
};
use crate::systems::movement::{MovementDirection, MovementSpeed};
use crate::systems::scripting::loader::ScriptLoader;

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

#[derive(Clone, Debug)]
pub struct ActorData {
    pub race: Option<RecordId>,
    pub sex: Sex,
    properties: ActorValueProperties,
    skin: Option<Vec<ItemStack>>,
    customization_scenes: Vec<RecordId>,
    outfit_items: Vec<RecordId>,
    items: Vec<(RecordId, u64)>,
    props: ActorProperties,
    dead: bool,
    speeds: Option<ActorMovementSpeeds>,
    pub keywords: HashSet<RecordId>,
}

impl ActorData {
    pub async fn load(
        esp: &EspLoader,
        scripts: &ScriptLoader,
        npc: &GlobalRecord,
        dead: bool,
    ) -> Result<Self, ()> {
        let mut props = resolve_actor_props(esp, npc).await;

        let customization_scenes = load_actor_customization(esp, props.base_id, &props.base).await;

        if let Some(class_id) = props.class
            && !class_id.form.is_null()
        {
            let class = esp.lookup(Some(RecordKind::Class), class_id).await.unwrap();
            let RecordData::Class(class) = class.local.data else {
                todo!();
            };

            for prop in &*class.properties {
                let actor_value = esp.resolve_form(prop.actor_value, class_id).unwrap();

                let old_value = props.properties.get(actor_value).unwrap_or_default();
                props
                    .properties
                    .set(actor_value, ActorValue::new(old_value.value + prop.value));
            }
        }

        let mut is_child = false;
        let mut speeds = None;
        if let Some(race_id) = props.race
            && !race_id.form.is_null()
        {
            let RecordData::Race(race) = esp
                .lookup(Some(RecordKind::Race), race_id)
                .await
                .unwrap()
                .local
                .data
            else {
                todo!();
            };

            speeds = Some(load_movement_speeds(esp, race_id, &race).await);
            is_child = race.data.flags.contains(RaceDataFlags::CHILD);

            for condition in race.biped_object_conditions.iter().flatten() {
                if condition.is_null() {
                    continue;
                }

                let condition = esp.resolve_form(*condition, race_id).unwrap();

                // If this exists it was either overwritten to a custom value, or a
                // previous condition already set this.
                if props.properties.get(condition).is_some() {
                    continue;
                }

                let value = esp
                    .lookup(Some(RecordKind::ActorValueInfo), condition)
                    .await
                    .unwrap();

                let RecordData::ActorValueInfo(value) = value.local.data else {
                    todo!();
                };

                let value = value.default_value.unwrap_or_default();
                props.properties.set(condition, ActorValue::new(value));
            }

            // If the `Npc` record did not define a skin, we use the skin defined
            // in the `Race` record.
            if let Some(skin) = race.skin
                && props.skin.is_none()
            {
                let skin = esp.resolve_form(skin, race_id).unwrap();
                props.skin = Some(skin);
            }
        }

        let outfit_items = if let Some(outfit) = props.outfit {
            load_outfit(esp, outfit).await
        } else {
            Vec::new()
        };

        let skin = if let Some(skin) = props.skin {
            let mut skin = load_item(esp, scripts, skin, &[], 1).await;
            for item in &mut skin {
                item.equipment_slots = HashSet::new();
                item.visibility.priority = VISIBILITY_PRIORITY_SKIN;
            }

            Some(skin)
        } else {
            None
        };

        let level = match props.level {
            ActorLevel::Fixed(level) => u32::from(level),
            ActorLevel::PlayerScaled { factor, min, max } => {
                let level = u32::from(PLAYER_LEVEL).saturating_mul(u32::from(factor));
                u32::clamp(level, u32::from(min), u32::from(max))
            }
        };

        let sex = if props.female { Sex::Female } else { Sex::Male };

        Ok(Self {
            race: props.race,
            sex,
            properties: props.properties,
            keywords: props.keywords.into_iter().collect(),
            customization_scenes,
            outfit_items,
            props: ActorProperties {
                protection: props.protection,
                bleedout_recovery: true,
                level,
                perk_points: 0,
                factions: props.factions,
                is_dead: dead,
                is_child,
                voice_type: props.voice_type,
            },
            dead,
            items: props.items,
            skin,
            speeds,
        })
    }

    pub fn instantiate(self, world: &mut World, events: &mut EventBus, root: Entity) {
        if let Some(skin) = self.skin {
            for item in skin {
                events.send(EquipItemEvent {
                    actor: root,
                    item: item.item,
                });
                events.send(AddItemEvent { actor: root, item });
            }
        }

        for item in self.customization_scenes {
            events.send(AddItemEventAsync {
                actor: root,
                item: item,
                attachments: Vec::new(),
                count: 1,
                hidden: true,
                equip: true,
            });
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

        for (item, count) in self.items {
            events.send(AddItemEventAsync {
                actor: root,
                item,
                attachments: Vec::new(),
                hidden: false,
                count,
                equip: false,
            });
        }

        world
            .insert(
                root,
                (
                    ActorMarker,
                    Inventory::default(),
                    Equipment::default(),
                    self.properties,
                    self.props,
                    MovementDirection::ZERO,
                    MovementSpeed::default(),
                    MovementState::default(),
                    FacingDirection::default(),
                ),
            )
            .unwrap();

        if self.dead {
            world.insert_one(root, DeadActor).unwrap();
        }

        if let Some(speeds) = self.speeds {
            world.insert_one(root, speeds).unwrap();
        }
    }
}

async fn resolve_actor_props(esp: &EspLoader, npc: &GlobalRecord) -> ActorProps {
    // Base Stats
    let mut essential = false;
    let mut protected = false;
    let mut respawn = false;

    // Traits
    let mut race = None;
    let mut skin = None;
    let mut female = false;
    let mut voice_type = None;

    // Stats
    let mut properties = ActorValueProperties::default();
    let mut level = ActorLevel::default();
    let mut class = None;

    // Factions
    let mut factions = HashMap::new();

    let mut keywords = Vec::new();

    // Inventory
    let mut outfit = None;
    let mut items = Vec::new();

    let mut queue = VecDeque::new();
    queue.push_back((NpcTemplateFlags::all(), Cow::Borrowed(npc)));
    let mut visited = HashSet::new();

    let base = npc;

    while let Some((flags, record)) = queue.pop_front() {
        visited.insert(record.id);

        match &record.local.data {
            RecordData::Npc(data) => {
                if flags.contains(NpcTemplateFlags::TRAITS) {
                    race = Some(esp.resolve_form(data.race, record.id).unwrap());

                    if let Some(form) = data.skin {
                        skin = Some(esp.resolve_form(form, record.id).unwrap());
                    }

                    female = data.base_stats.flags.contains(NpcBaseFlags::FEMALE);

                    voice_type = data
                        .voice_type
                        .map(|id| esp.resolve_form(id, record.id).unwrap());
                }

                if flags.contains(NpcTemplateFlags::STATS) {
                    properties.clear();
                    for prop in &*data.properties {
                        let actor_value = esp.resolve_form(prop.actor_value, record.id).unwrap();

                        let old_value = properties.get(actor_value).unwrap_or_default();
                        properties.set(actor_value, ActorValue::new(old_value.value + prop.value));
                    }

                    if let Some(form) = data.class {
                        class = Some(esp.resolve_form(form, record.id).unwrap());
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
                            let faction = esp.resolve_form(membership.faction, record.id).unwrap();

                            (faction, i32::from(membership.rank))
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
                        .map(|form| esp.resolve_form(form, record.id).unwrap())
                        .collect();
                }

                if flags.contains(NpcTemplateFlags::INVENTORY) {
                    outfit = data
                        .default_outfit
                        .map(|form| esp.resolve_form(form, record.id).unwrap());

                    items = data
                        .items
                        .iter()
                        .map(|item| {
                            let id = esp.resolve_form(item.item, record.id).unwrap();
                            (id, item.count.into())
                        })
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

                        let record_id = esp.resolve_form(form_id, record.id).unwrap();

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
                        let npc = esp.lookup(None, form).await.unwrap();
                        queue.push_back((flags, Cow::Owned(npc)));
                    }
                }
            }
            RecordData::LeveledActor(data) => {
                let mut entries = data.entries.clone();

                // Discard any entries that we have already seen, otherwise
                // we might get a cycle in the reference graph.
                entries.retain(|entry| {
                    esp.resolve_form(entry.reference, record.id)
                        .is_some_and(|id| !visited.contains(&id))
                });

                for entry in select_from_leveled_list(data.flags, &entries, 1, PLAYER_LEVEL) {
                    let form = data.entries[entry.index].reference;
                    let record_id = esp.resolve_form(form, record.id).unwrap();
                    let record = esp.lookup(None, record_id).await.unwrap();
                    queue.push_back((flags, Cow::Owned(record)));
                }
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

    let base_id = base.id;
    let RecordData::Npc(base) = base.clone().local.data else {
        todo!();
    };

    ActorProps {
        base_id,
        base,
        race,
        skin,
        female,
        class,
        properties,
        keywords,
        outfit,
        protection,
        level,
        factions,
        items,
        voice_type,
    }
}

#[derive(Clone, Debug)]
struct ActorProps {
    /// The non-leveled list NPC base record.
    base_id: RecordId,
    base: Npc,
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
    factions: HashMap<RecordId, i32>,
    items: Vec<(RecordId, u64)>,
    voice_type: Option<RecordId>,
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

async fn load_actor_customization(
    esp_loader: &EspLoader,
    id: RecordId,
    npc: &Npc,
) -> Vec<RecordId> {
    let mut queue = Vec::new();
    for form in &npc.head_parts {
        let id = esp_loader.resolve_form(*form, id).unwrap();
        queue.push(id);
    }

    let mut scenes = Vec::new();

    while let Some(id) = queue.pop() {
        scenes.push(id);

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

async fn load_movement_speeds(
    loader: &EspLoader,
    race_id: RecordId,
    race: &Race,
) -> ActorMovementSpeeds {
    let default = race
        .base_movement_defaults_default
        .map(|id| loader.resolve_form(id, race_id).unwrap())
        .unwrap_or(DEFAULT_MOVEMENT_TYPE_DEFAULT);

    let RecordData::MovementType(default) = loader
        .lookup(Some(RecordKind::MovementType), default)
        .await
        .unwrap()
        .local
        .data
    else {
        todo!();
    };

    let sneak = race
        .base_movement_defaults_sneak
        .map(|id| loader.resolve_form(id, race_id).unwrap())
        .unwrap_or(DEFAULT_MOVEMENT_TYPE_SNEAK);

    let RecordData::MovementType(sneak) = loader
        .lookup(Some(RecordKind::MovementType), sneak)
        .await
        .unwrap()
        .local
        .data
    else {
        todo!();
    };

    ActorMovementSpeeds {
        default: default.speeds,
        sneak: sneak.speeds,
    }
}
