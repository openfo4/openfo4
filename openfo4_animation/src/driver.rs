use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use game_tracing::trace_span;
use slab::Slab;

use crate::events::EventSet;
use crate::nodes::{Context, GraphNode, Node, NodeEvent, NodeInner, Property};
use crate::variables::VariableSet;
use crate::{AnimationGraph, AnimationProvider, BoneWeights, Event, NodeId, Value, VariableId};

#[derive(Debug)]
pub struct AnimationDriver {
    graph: Arc<AnimationGraph>,
    nodes: Slab<Node>,
    variables: VariableSet,
    queue: VecDeque<NodeEvent>,
    properties: PropertySet,
    active_nodes: HashMap<NodeId, NodeState>,
}

impl AnimationDriver {
    pub fn new(graph: Arc<AnimationGraph>) -> Self {
        let mut driver = Self {
            nodes: graph.nodes.clone(),
            variables: graph.variables.clone(),
            queue: VecDeque::new(),
            active_nodes: HashMap::new(),
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
        self.active_nodes.keys().copied().collect()
    }

    pub(crate) fn global_weight(&self, node: NodeId) -> Option<&BoneWeights> {
        self.properties.global_weight.get(&node)
    }

    pub fn update(
        &mut self,
        delta: Duration,
        animations: &dyn AnimationProvider,
        events: &mut Vec<Event>,
    ) {
        let _span = trace_span!("AnimationDriver::update").entered();

        while let Some(event) = self.queue.pop_front() {
            match event {
                NodeEvent::ActivateNode(id) => {
                    let node = &mut self.nodes[id];

                    let state = self.active_nodes.entry(id).or_insert_with(|| {
                        let mut state = NodeState::default();

                        let ctx = UpdateContext::new(
                            id,
                            &self.graph,
                            delta,
                            &mut self.variables,
                            &mut self.queue,
                            &mut self.properties,
                            animations,
                            &mut state,
                        );
                        node.activate(ctx);

                        state
                    });

                    state.active_count += 1;
                }
                NodeEvent::DeactivateNode(id) => {
                    let state = self.active_nodes.get_mut(&id).unwrap();
                    state.active_count -= 1;

                    if state.active_count == 0 {
                        let node = &mut self.nodes[id];

                        let ctx = UpdateContext::new(
                            id,
                            &self.graph,
                            delta,
                            &mut self.variables,
                            &mut self.queue,
                            &mut self.properties,
                            animations,
                            state,
                        );
                        node.deactivate(ctx);

                        for children in state.active_children.drain() {
                            self.queue.push_back(NodeEvent::DeactivateNode(children));
                        }

                        self.active_nodes.remove(&id);
                    }
                }
                NodeEvent::Event(event) => {
                    // The caller also gets the event.
                    // FIXME: This will feed back events produced by the caller,
                    // but this is probably not desired?
                    events.push(event.clone());

                    for (id, state) in &mut self.active_nodes {
                        let node = &mut self.nodes[*id];

                        let ctx = UpdateContext::new(
                            *id,
                            &self.graph,
                            delta,
                            &mut self.variables,
                            &mut self.queue,
                            &mut self.properties,
                            animations,
                            state,
                        );
                        node.handle_event(ctx, &event);
                    }
                }
            }
        }

        self.compute_global_speed();

        for (id, state) in &mut self.active_nodes {
            let node = &mut self.nodes[*id];

            let speed = self.properties.global_speed.get(id).copied().unwrap_or(1.0);
            let delta = delta.mul_f32(speed);

            let ctx = UpdateContext::new(
                *id,
                &self.graph,
                delta,
                &mut self.variables,
                &mut self.queue,
                &mut self.properties,
                animations,
                state,
            );
            node.update(ctx);
        }

        self.compute_global_weights();
        self.compute_global_speed();
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
        self.variables.get_by_name(name).cloned()
    }

    pub fn get_variable_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut_by_name(name)
    }

    pub fn send_event(&mut self, event: &str) {
        let Some(id) = self.graph.events.lookup_name(&event.to_lowercase()) else {
            return;
        };

        let event = Event {
            id,
            payload: Value::None,
        };

        self.queue.push_back(NodeEvent::Event(event));
    }

    pub fn animations(&self) -> impl Iterator<Item = PlayingAnimationClip<'_>> {
        self.active_nodes
            .keys()
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

        // Note that it is possible for multiple nodes to reference the same other
        // node. In that case the weights for each bone need to be computed for each
        // subtree of nodes and then summed up.
        // However since nodes only apply weights of their immediate children we only
        // need to extract the weights for that particular (parent, children) set.
        // For this we keep a (Node, Parent that set weight) for each weight.
        // Also note: all of this is very cursed.

        let mut global_weights: HashMap<usize, Vec<_>> = HashMap::new();

        let mut queue = VecDeque::new();
        queue.push_back((root, usize::MAX, BoneWeights::Shared(1.0)));

        while let Some((id, parent, parent_weight)) = queue.pop_front() {
            let node = &self.nodes[id];

            let weight = self
                .properties
                .weight
                .get(&id)
                .map(|v| v.get(&parent))
                .flatten()
                .unwrap_or(&BoneWeights::Shared(1.0));

            let global_weight = parent_weight * weight.clone();
            global_weights
                .entry(id)
                .or_default()
                .push(global_weight.clone());

            for children in node.children() {
                if !self.active_nodes.contains_key(&children) {
                    continue;
                }

                queue.push_back((children, id, global_weight.clone()));
            }
        }

        for (id, weights) in global_weights {
            let weights = weights
                .into_iter()
                .reduce(|acc, w| acc + w)
                .unwrap_or(BoneWeights::Shared(0.0));

            self.properties.global_weight.insert(id, weights);
        }
    }

    fn compute_global_speed(&mut self) {
        let Some(root) = self.graph.root else {
            return;
        };

        let mut queue = VecDeque::new();
        queue.push_back((root, 1.0));

        while let Some((id, parent_speed)) = queue.pop_front() {
            let node = &self.nodes[id];

            let local_speed = self.properties.speed.get(&id).copied().unwrap_or(1.0);

            let speed = parent_speed * local_speed;
            self.properties.global_speed.insert(id, speed);

            for children in node.children() {
                if !self.active_nodes.contains_key(&children) {
                    continue;
                }

                queue.push_back((children, speed));
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
struct NodeState {
    active_count: usize,
    active_children: HashSet<NodeId>,
}

struct UpdateContext<'a> {
    node: NodeId,
    graph: &'a AnimationGraph,
    delta: Duration,
    variables: &'a mut VariableSet,
    queue: &'a mut VecDeque<NodeEvent>,
    properties: &'a mut PropertySet,
    animations: &'a dyn AnimationProvider,
    state: &'a mut NodeState,
}

impl<'a> UpdateContext<'a> {
    fn new(
        node: NodeId,
        graph: &'a AnimationGraph,
        delta: Duration,
        variables: &'a mut VariableSet,
        queue: &'a mut VecDeque<NodeEvent>,
        properties: &'a mut PropertySet,
        animations: &'a dyn AnimationProvider,
        state: &'a mut NodeState,
    ) -> Self {
        Self {
            node,
            graph,
            delta,
            variables,
            queue,
            properties,
            animations,
            state,
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

        if self.state.active_children.insert(id) {
            self.queue.push_back(NodeEvent::ActivateNode(id));
        }
    }

    fn deactivate_node(&mut self, id: NodeId) {
        tracing::trace!(
            "Node {} deactivated node {}",
            &self.graph.nodes[self.node].name,
            &self.graph.nodes[id].name,
        );

        if self.state.active_children.remove(&id) {
            self.queue.push_back(NodeEvent::DeactivateNode(id));
        }
    }

    fn send_event(&mut self, event: Event) {
        tracing::trace!(
            "Node {} sent event {}",
            &self.graph.nodes[self.node].name,
            self.graph.event_name(event.id).unwrap_or(""),
        );

        self.queue.push_back(NodeEvent::Event(event));
    }

    fn variables(&self) -> &VariableSet {
        &self.variables
    }

    fn events(&self) -> &EventSet {
        &self.graph.events
    }

    fn set_variable(&mut self, id: VariableId, value: Value) {
        *self.variables.get_mut(id).unwrap() = value;
    }

    fn animations(&self) -> &dyn AnimationProvider {
        self.animations
    }

    fn set_property(&mut self, node: NodeId, property: Property, value: Value) {
        match property {
            Property::Weight => {
                let weights = match value {
                    Value::F32(weight) => BoneWeights::Shared(weight),
                    Value::BoneWeights(weights) => BoneWeights::Unique(weights),
                    _ => panic!("invalid value for bone weights"),
                };

                self.properties
                    .weight
                    .entry(node)
                    .or_default()
                    .insert(self.node, weights);
            }
            Property::GlobalWeight => {}
            Property::Speed => {
                let speed = value.to_f32().unwrap();
                self.properties.speed.insert(node, speed);
            }
            Property::GlobalSpeed => {}
        }
    }
}

#[derive(Clone, Debug, Default)]
struct PropertySet {
    /// Bone weights for each node as assigned by each other node.
    ///
    /// See [`compute_global_weights`] for why we need multiple weights per node.
    ///
    /// [`compute_global_weights`]: AnimationDriver::compute_global_weights
    weight: HashMap<NodeId, HashMap<NodeId, BoneWeights>>,
    global_weight: HashMap<NodeId, BoneWeights>,
    speed: HashMap<NodeId, f32>,
    global_speed: HashMap<NodeId, f32>,
}

#[derive(Clone, Debug)]
pub struct PlayingAnimationClip<'a> {
    pub path: &'a str,
    pub weight: &'a BoneWeights,
    pub time: Duration,
    pub looping: bool,
}
