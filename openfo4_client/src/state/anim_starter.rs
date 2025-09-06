use std::collections::VecDeque;

use hashbrown::HashMap;
use hecs::World;
use openfo4_animation::driver::AnimationDriver;
use openfo4_esp::{FormId, RecordData, RecordKind};

use crate::{loader::fix_path, state::esp::EspLoader};

#[derive(Clone, Debug)]
struct Reaction {
    id: FormId,
    /// Only trigger the reaction when the animation graph is active.
    behavior_graph: String,
    /// The event that is sent to the animation graph when this reaction is triggered.
    event: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AnimationSystem {
    reactions: HashMap<FormId, Vec<Reaction>>,
}

impl AnimationSystem {
    pub fn new(esp: &EspLoader) -> Self {
        let animations =
            futures_lite::future::block_on(esp.lookup_all(Some(RecordKind::Animation))).unwrap();

        let mut reactions: HashMap<FormId, Vec<Reaction>> = HashMap::new();

        for record in &animations {
            let RecordData::Animation(anim) = &record.data else {
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

            reactions
                .entry(anim.relations.parent)
                .or_default()
                .push(Reaction {
                    id: record.id,
                    behavior_graph: fix_path(behavior_graph),
                    event,
                });
        }

        Self { reactions }
    }

    pub fn update(&mut self, world: &mut World) {
        for (_, (active_graphs, events, driver)) in world.query_mut::<(
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
pub struct ActiveBehaviorGraphs(pub Vec<String>);

#[derive(Clone, Debug, Default)]
pub struct AnimationEventQueue(pub VecDeque<FormId>);
