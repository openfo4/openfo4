use std::collections::VecDeque;

use hashbrown::HashMap;
use openfo4_animation::driver::AnimationDriver;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::conditions::Condition;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::esp::EspLoader;
use crate::systems::conditions;
use crate::systems::global::GlobalProperties;

#[derive(Clone, Debug)]
struct Reaction {
    id: RecordId,
    /// Only trigger the reaction when the animation graph is active.
    behavior_graph: DataPathBuf,
    /// The event that is sent to the animation graph when this reaction is triggered.
    event: Option<String>,
    conditions: Vec<Condition>,
}

#[derive(Clone, Debug)]
pub struct AnimationSystem {
    reactions: HashMap<RecordId, Vec<Reaction>>,
}

impl AnimationSystem {
    pub fn new(esp: &EspLoader) -> Self {
        let animations =
            futures_lite::future::block_on(esp.lookup_all(Some(RecordKind::Animation))).unwrap();

        let mut reactions: HashMap<RecordId, Vec<Reaction>> = HashMap::new();

        for record in animations {
            let RecordData::Animation(anim) = record.local.data else {
                continue;
            };

            if anim.relations.parent.is_null() {
                continue;
            }

            let Some(behavior_graph) = &anim.behavior_graph else {
                continue;
            };

            let behavior_graph = behavior_graph.as_str().unwrap();

            let event = anim
                .animation_event
                .as_ref()
                .map(|e| e.as_str().unwrap().to_owned());

            let parent = esp.resolve_form(anim.relations.parent, record.id).unwrap();

            reactions.entry(parent).or_default().push(Reaction {
                id: record.id,
                behavior_graph: DataPathBuf::new(behavior_graph),
                event,
                conditions: anim.conditions.conditions,
            });
        }

        Self { reactions }
    }

    pub fn update(
        &mut self,
        world: &mut World,
        esp_loader: &EspLoader,
        globals: &GlobalProperties,
    ) {
        let mut events_to_send = Vec::new();

        for (entity, active_graphs, mut events) in world
            .query::<(Entity, &ActiveBehaviorGraphs, &mut AnimationEventQueue)>()
            .iter()
        {
            while let Some(id) = events.0.pop_front() {
                let Some(reactions) = self.reactions.get(&id) else {
                    continue;
                };

                for reaction in reactions {
                    if !active_graphs.0.contains(&reaction.behavior_graph) {
                        continue;
                    }

                    if !conditions::evaluate(
                        world,
                        esp_loader,
                        globals,
                        reaction.id,
                        &reaction.conditions,
                        entity,
                        entity,
                    ) {
                        continue;
                    }

                    if let Some(event) = &reaction.event {
                        events_to_send.push((entity, event));
                    }

                    events.0.push_back(reaction.id);
                }
            }
        }

        for (entity, event) in events_to_send {
            if let Ok(mut driver) = world.get::<&mut AnimationDriver>(entity) {
                driver.send_event(event);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveBehaviorGraphs(pub Vec<DataPathBuf>);

#[derive(Clone, Debug, Default)]
pub struct AnimationEventQueue(pub VecDeque<RecordId>);
