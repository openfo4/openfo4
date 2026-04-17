use std::collections::VecDeque;

use hashbrown::HashMap;
use openfo4_animation::driver::AnimationDriver;
use openfo4_core::ecs::World;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::esp::EspLoader;

#[derive(Clone, Debug)]
struct Reaction {
    id: RecordId,
    /// Only trigger the reaction when the animation graph is active.
    behavior_graph: DataPathBuf,
    /// The event that is sent to the animation graph when this reaction is triggered.
    event: Option<String>,
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

        for record in &animations {
            let RecordData::Animation(anim) = &record.local.data else {
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
            });
        }

        Self { reactions }
    }

    pub fn update(&mut self, world: &mut World) {
        for (active_graphs, mut events, mut driver) in world.query_mut::<(
            &ActiveBehaviorGraphs,
            &mut AnimationEventQueue,
            &mut AnimationDriver,
        )>() {
            while let Some(id) = events.0.pop_front() {
                let Some(reactions) = self.reactions.get(&id) else {
                    continue;
                };

                for reaction in reactions {
                    if !active_graphs.0.contains(&reaction.behavior_graph) {
                        continue;
                    }

                    if let Some(event) = &reaction.event {
                        if !event.to_lowercase().contains("injured") {
                            driver.send_event(event);
                        }
                    }

                    events.0.push_back(reaction.id);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveBehaviorGraphs(pub Vec<DataPathBuf>);

#[derive(Clone, Debug, Default)]
pub struct AnimationEventQueue(pub VecDeque<RecordId>);
