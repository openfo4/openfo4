use std::sync::Arc;

use hashbrown::HashSet;
use openfo4_animation::driver::AnimationDriver;
use openfo4_asset::AssetLoader;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_esp::records::{SubgraphPerspective, SubgraphRole};
use openfo4_tasks::TaskPool;
use openfo4_tracing::trace_span;

use crate::state::actor::behavior::{ActorBehaviorData, ActorBehaviorDescriptor};
use crate::state::anim_starter::ActiveBehaviorGraphs;
use crate::state::npc::{ActorAnimationGraphData, ObjectKeywords};
use crate::systems::inventory::{Inventory, ItemData};

pub fn update_actor_animation_graphs(world: &mut World, loader: &AssetLoader, pool: &TaskPool) {
    let _span = trace_span!("update_actor_animation_graphs").entered();

    let mut buffer = CommandBuffer::new();

    for (entity, graphs, keywords, inventory, active_graph) in world.query_mut::<(
        Entity,
        &ActorAnimationGraphData,
        &ObjectKeywords,
        &Inventory,
        Option<&ActiveAnimationGraph>,
    )>() {
        let mut active_index = None;
        let mut max_actor_keywords = 0;
        let mut max_target_keywords = 0;

        let is_weapon_drawn = inventory.items().any(|(_, item)| match &item.data {
            ItemData::Weapon(weapon) => weapon.drawn,
            _ => false,
        });
        let weapon_keywords: HashSet<_> = inventory
            .items()
            .filter(|(_, item)| match &item.data {
                ItemData::Weapon(weapon) => weapon.drawn,
                _ => false,
            })
            .map(|(_, v)| v.combined_keywords.iter())
            .flatten()
            .copied()
            .collect();

        for (index, graph) in graphs.subgraphs.iter().enumerate() {
            let expected_role = if is_weapon_drawn {
                SubgraphRole::Weapon
            } else {
                SubgraphRole::Mt
            };

            if graph.flags.role != expected_role {
                continue;
            }

            if !matches!(graph.flags.perspective, SubgraphPerspective::Third) {
                continue;
            }

            if !graph.actor_keywords.iter().all(|kw| keywords.contains(*kw)) {
                continue;
            }

            if !graph
                .target_keywords
                .iter()
                .all(|kw| weapon_keywords.contains(kw))
            {
                continue;
            }

            if max_actor_keywords > graph.actor_keywords.len() {
                continue;
            }

            if max_target_keywords > graph.target_keywords.len() {
                continue;
            }

            active_index = Some(index);
            max_actor_keywords = graph.actor_keywords.len();
            max_target_keywords = graph.target_keywords.len();
        }

        let Some(active_index) = active_index else {
            continue;
        };

        if active_graph.is_some_and(|v| v.0 == active_index) {
            continue;
        }

        let graph = &graphs.subgraphs[active_index];

        let Ok(behavior) = futures_lite::future::block_on(ActorBehaviorData::load(
            loader,
            pool,
            ActorBehaviorDescriptor {
                root_path: &graphs.root_path,
                behavior_path: graphs.root_behavior_path.clone(),
                character: &graphs.character,
                subgraph_path: graph.behavior.clone(),
                search_paths: &graph.search_paths,
            },
        ))
        .inspect_err(|err| {
            tracing::error!(
                "failed to load behavior graph {} with subgraph {}: {:?}",
                graphs.root_behavior_path,
                graph.behavior,
                err
            );
        }) else {
            // Continue as if we loaded the behavior files successfully,
            // otherwise we would reload the same (broken) behavior every
            // frame.
            buffer.insert_one(entity, ActiveAnimationGraph(active_index));
            continue;
        };

        buffer.insert(
            entity,
            (
                ActiveAnimationGraph(active_index),
                AnimationDriver::new(Arc::new(behavior.graph)),
                behavior.clips,
                ActiveBehaviorGraphs(behavior.behavior_files.clone()),
            ),
        );
    }

    buffer.run_on(world);
}

#[derive(Copy, Clone, Debug)]
struct ActiveAnimationGraph(usize);
