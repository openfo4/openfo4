use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use game_tracing::trace_span;
use slab::Slab;

use crate::nodes::{Context, GraphNode, Node, NodeEvent, NodeInner, Property};
use crate::{
    AnimationGraph, AnimationSet, BoneWeights, Event, NodeId, Value, VariableId, VariableSet,
};

#[derive(Debug)]
pub struct AnimationDriver {
    graph: Arc<AnimationGraph>,
    nodes: Slab<Node>,
    variables: VariableSet,
    queue: VecDeque<NodeEvent>,
    active_nodes: HashSet<NodeId>,
    properties: PropertySet,
}

impl AnimationDriver {
    pub fn new(graph: Arc<AnimationGraph>) -> Self {
        let mut driver = Self {
            nodes: graph.nodes.clone(),
            variables: graph.variables.clone(),
            queue: VecDeque::new(),
            active_nodes: HashSet::new(),
            properties: PropertySet::default(),
            graph,
        };
        driver.reset();
        driver
    }

    pub fn graph(&self) -> &AnimationGraph {
        &self.graph
    }

    pub(crate) fn active_nodes(&self) -> Vec<usize> {
        self.active_nodes.iter().copied().collect()
    }

    pub fn update(&mut self, delta: Duration, animations: &AnimationSet, events: &mut Vec<Event>) {
        let _span = trace_span!("AnimationDriver::update").entered();

        while let Some(event) = self.queue.pop_front() {
            match event {
                NodeEvent::ActivateNode(id) => {
                    if self.active_nodes.contains(&id) {
                        continue;
                    }

                    let node = &mut self.nodes[id];

                    let ctx = UpdateContext::new(
                        id,
                        &self.graph,
                        delta,
                        &mut self.variables,
                        &mut self.queue,
                        &mut self.properties,
                        animations,
                    );
                    node.activate(ctx);

                    self.active_nodes.insert(id);
                }
                NodeEvent::DeactivateNode(id) => {
                    if !self.active_nodes.contains(&id) {
                        continue;
                    }

                    let node = &mut self.nodes[id];

                    let ctx = UpdateContext::new(
                        id,
                        &self.graph,
                        delta,
                        &mut self.variables,
                        &mut self.queue,
                        &mut self.properties,
                        animations,
                    );
                    node.deactivate(ctx);

                    self.active_nodes.remove(&id);
                }
                NodeEvent::Event(event) => {
                    // The caller also gets the event.
                    // FIXME: This will feed back events produced by the caller,
                    // but this is probably not desired?
                    events.push(event.clone());

                    for id in &self.active_nodes {
                        let node = &mut self.nodes[*id];

                        let ctx = UpdateContext::new(
                            *id,
                            &self.graph,
                            delta,
                            &mut self.variables,
                            &mut self.queue,
                            &mut self.properties,
                            animations,
                        );
                        node.handle_event(ctx, &event);
                    }
                }
            }
        }

        for id in &self.active_nodes {
            let node = &mut self.nodes[*id];

            let ctx = UpdateContext::new(
                *id,
                &self.graph,
                delta,
                &mut self.variables,
                &mut self.queue,
                &mut self.properties,
                animations,
            );
            node.update(ctx);
        }

        self.compute_global_weights();
    }

    pub fn reset(&mut self) {
        self.nodes = self.graph.nodes.clone();
        self.variables = self.graph.variables.clone();
        self.queue.clear();

        self.queue.clear();
        if let Some(root) = self.graph.root {
            self.queue.push_back(NodeEvent::ActivateNode(root));
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.get_by_name(name)
    }

    pub fn get_variable_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut_by_name(name)
    }

    pub fn send_event(&mut self, event: &str) {
        let Some(event_id) = self
            .graph
            .event_names
            .iter()
            .position(|e| e.to_lowercase() == event.to_lowercase())
        else {
            return;
        };

        let event = Event {
            id: event_id as i64,
            payload: Value::None,
        };

        self.queue.push_back(NodeEvent::Event(event));
    }

    pub fn animations(&self) -> impl Iterator<Item = PlayingAnimationClip<'_>> {
        self.active_nodes
            .iter()
            .filter_map(|id| match &self.nodes[*id].inner {
                NodeInner::Clip(clip) => {
                    let weight = self
                        .properties
                        .global_weight
                        .get(id)
                        .unwrap_or(&BoneWeights::Shared(1.0));

                    Some(PlayingAnimationClip {
                        path: &clip.path,
                        weight,
                        time: clip.time,
                        looping: clip.looping,
                    })
                }
                _ => None,
            })
    }

    fn compute_global_weights(&mut self) {
        let Some(root) = self.graph.root else {
            return;
        };

        let mut queue = VecDeque::new();
        queue.push_back((root, BoneWeights::Shared(1.0)));

        while let Some((id, parent_weight)) = queue.pop_front() {
            let node = &self.nodes[id];

            let weight = self
                .properties
                .weight
                .get(&id)
                .unwrap_or(&BoneWeights::Shared(1.0));

            let global_weight = parent_weight * weight.clone();
            self.properties
                .global_weight
                .insert(id, global_weight.clone());

            for children in node.children() {
                if !self.active_nodes.contains(&children) {
                    continue;
                }

                queue.push_back((children, global_weight.clone()));
            }
        }
    }
}

struct UpdateContext<'a> {
    node: NodeId,
    graph: &'a AnimationGraph,
    delta: Duration,
    variables: &'a mut VariableSet,
    queue: &'a mut VecDeque<NodeEvent>,
    properties: &'a mut PropertySet,
    animations: &'a AnimationSet,
}

impl<'a> UpdateContext<'a> {
    fn new(
        node: NodeId,
        graph: &'a AnimationGraph,
        delta: Duration,
        variables: &'a mut VariableSet,
        queue: &'a mut VecDeque<NodeEvent>,
        properties: &'a mut PropertySet,
        animations: &'a AnimationSet,
    ) -> Self {
        Self {
            node,
            graph,
            delta,
            variables,
            queue,
            properties,
            animations,
        }
    }
}

impl Context for UpdateContext<'_> {
    fn delta(&self) -> Duration {
        self.delta
    }

    fn activate_node(&mut self, id: NodeId) {
        tracing::trace!(
            "Node {} activated node {}",
            &self.graph.nodes[self.node].name,
            &self.graph.nodes[id].name,
        );

        self.queue.push_back(NodeEvent::ActivateNode(id));
    }

    fn deactivate_node(&mut self, id: NodeId) {
        tracing::trace!(
            "Node {} deactivated node {}",
            &self.graph.nodes[self.node].name,
            &self.graph.nodes[id].name,
        );

        self.queue.push_back(NodeEvent::DeactivateNode(id));
    }

    fn send_event(&mut self, event: Event) {
        tracing::trace!(
            "Node {} sent event {}",
            &self.graph.nodes[self.node].name,
            &self.graph.event_names[event.id as usize],
        );

        self.queue.push_back(NodeEvent::Event(event));
    }

    fn variables(&self) -> &VariableSet {
        &self.variables
    }

    fn set_variable(&mut self, id: VariableId, value: Value) {
        self.variables.variables[id] = value;
    }

    fn animations(&self) -> &AnimationSet {
        &self.animations
    }

    fn set_property(&mut self, node: NodeId, property: Property, value: Value) {
        match property {
            Property::Weight => {
                let weights = match value {
                    Value::F32(weight) => BoneWeights::Shared(weight),
                    Value::BoneWeights(weights) => BoneWeights::Unique(weights),
                    _ => panic!("invalid value for bone weights"),
                };

                self.properties.weight.insert(node, weights);
            }
            Property::GlobalWeight => {}
        }
    }
}

#[derive(Clone, Debug, Default)]
struct PropertySet {
    weight: HashMap<NodeId, BoneWeights>,
    global_weight: HashMap<NodeId, BoneWeights>,
}

#[derive(Clone, Debug)]
pub struct PlayingAnimationClip<'a> {
    pub path: &'a str,
    pub weight: &'a BoneWeights,
    pub time: Duration,
    pub looping: bool,
}
