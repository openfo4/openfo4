use hashbrown::HashMap;
use openfo4_animation::driver::AnimationDriver;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::actions::{Action, Actions};
use crate::state::esp::EspLoader;
use crate::systems::conditions::{CompiledCondition, ConditionCompiler};
use crate::systems::quests::QuestManager;
use crate::systems::scripting::Executor;

#[derive(Clone, Debug)]
struct Reaction {
    id: RecordId,
    /// Only trigger the reaction when the animation graph is active.
    behavior_graph: DataPathBuf,
    /// The event that is sent to the animation graph when this reaction is triggered.
    event: Option<String>,
    /// Skip when this condition evauluates to `false`.
    condition: CompiledCondition,
}

#[derive(Clone, Debug)]
pub struct AnimationSystem {
    actions: Actions,
    reactions: HashMap<RecordId, Vec<Reaction>>,
}

impl AnimationSystem {
    pub async fn new(esp: &EspLoader, compiler: &ConditionCompiler) -> Self {
        let actions = Actions::load(esp).await;
        let animations = esp.lookup_all(Some(RecordKind::Animation)).await.unwrap();

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

            let conditions = compiler
                .compile(record.id, 0, &anim.conditions.conditions)
                .await;

            reactions.entry(parent).or_default().push(Reaction {
                id: record.id,
                behavior_graph: DataPathBuf::new(behavior_graph),
                event,
                condition: conditions,
            });
        }

        Self { reactions, actions }
    }

    /// Sends an animation action to an entity to start an animation.
    ///
    /// Note that due to the condition system it is not safe to defer this action event unless no
    /// simulation state has changed.
    pub fn send_action(
        &self,
        world: &mut World,
        entity: Entity,
        action: Action,
        executor: &mut Executor,
        event_bus: &mut EventBus,
        quests: &mut QuestManager,
    ) {
        let Some(action) = self.actions.0.get(&action) else {
            return;
        };

        let mut events = vec![*action];
        while let Some(id) = events.pop() {
            let Some(reactions) = self.reactions.get(&id) else {
                return;
            };

            for reaction in reactions {
                let Ok(active_graphs) = world.get::<&ActiveBehaviorGraphs>(entity) else {
                    return;
                };
                if !active_graphs.0.contains(&reaction.behavior_graph) {
                    continue;
                }
                drop(active_graphs);

                let subject = futures_lite::future::block_on(executor.get_or_load(world, entity));
                let res = reaction
                    .condition
                    .eval(executor, world, event_bus, quests, subject, None);

                tracing::trace!("condition for {} evaluated to {}", reaction.id, res);
                if !res {
                    continue;
                }

                if let Some(event) = &reaction.event {
                    let Ok(mut driver) = world.get::<&mut AnimationDriver>(entity) else {
                        return;
                    };
                    driver.send_event(event);
                }

                events.push(reaction.id);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveBehaviorGraphs(pub Vec<DataPathBuf>);
