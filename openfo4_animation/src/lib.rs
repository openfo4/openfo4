pub mod animation;
pub mod builder;
pub mod debug;
pub mod driver;
pub mod graph;
pub mod skeleton;
pub mod systems;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use animation::AnimationClip;
use glam::{Quat, Vec3, Vec4};
use openfo4_hkx::behavior::{BehaviorGraph, EventProperty, VariableType};
use openfo4_hkx::{HkObject, HkxFile};
use petgraph::Graph;
use petgraph::dot::Dot;
use petgraph::prelude::StableGraph;
use slab::Slab;

pub fn build_graph(file: &HkxFile) {
    let mut root = None;
    for obj in file.iter() {
        match obj {
            HkObject::BehaviorGraph(node) => {
                root = Some(node);
                break;
            }
            _ => (),
        }
    }

    let root = root.unwrap();

    let sm = match file.get(root.root_generator.as_ref().unwrap().ptr).unwrap() {
        HkObject::StateMachine(s) => s,
        _ => todo!(),
    };

    let mut graph = Graph::new();
    let root = graph.add_node(&sm.parent.node.name);

    let mut queue = VecDeque::new();
    for state in &*sm.states {
        queue.push_back((state.ptr, root));
    }
    let mut visited = HashSet::new();

    while let Some((ptr, parent)) = queue.pop_front() {
        if visited.contains(&ptr) {
            // continue;
        }
        visited.insert(ptr);

        match file.get(ptr) {
            Some(HkObject::StateMachine(sm)) => {
                let node = graph.add_node(&sm.parent.node.name);

                graph.add_edge(parent, node, "");

                for state in &*sm.states {
                    queue.push_back((state.ptr, node));
                }
            }
            Some(HkObject::StateMachineStateInfo(info)) => {
                let node = graph.add_node(&info.name);
                graph.add_edge(parent, node, "");

                if let Some(generator) = &info.generator {
                    queue.push_back((generator.ptr, node));
                }
            }
            Some(HkObject::ModifierGenerator(g)) => {
                let node = graph.add_node(&g.parent.node.name);
                graph.add_edge(parent, node, "");

                queue.push_back((g.modifier.ptr, node));
                queue.push_back((g.generator.ptr, node));
            }
            Some(HkObject::ModifierList(list)) => {
                let node = graph.add_node(&list.parent.node.name);
                graph.add_edge(parent, node, "");

                for modifier in &*list.modifiers {
                    queue.push_back((modifier.ptr, node));
                }
            }
            Some(HkObject::EventDrivenModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.modifer.node.name);
                graph.add_edge(parent, node, "");

                queue.push_back((modifier.parent.modifer.ptr, node));
            }
            Some(HkObject::BSGetTimeStepModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::EvaluateExpressionModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::TwistModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSOffsetAnimationGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                if let Some(g) = &generator.p_default_generator {
                    queue.push_back((g.ptr, node));
                }
            }
            Some(HkObject::BSIsActiveModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::FootIkControlsModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BlenderGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                for children in &*generator.children {
                    if let Some(g) = &children.generator {
                        queue.push_back((g.ptr, node));
                    }
                }
            }
            Some(HkObject::KeyframesBonesModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSRagdollContactListenerModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSLookAtModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSBehaviorGraphSwapGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                if let Some(g) = &generator.p_default_generator {
                    queue.push_back((g.ptr, node));
                }
            }
            Some(HkObject::ClipGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                let anim = graph.add_node(&generator.animation_name);
                graph.add_edge(node, anim, "");
            }
            Some(HkObject::ManualSelectorGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                for g in &*generator.generators {
                    queue.push_back((g.ptr, node));
                }
            }
            Some(HkObject::BSInterpValueModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::EventsFromRangeModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::GetUpModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::RigidBodyRagdollControlsModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSModifyOnceModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");

                if let Some(m) = &modifier.p_on_activate_modifier {
                    queue.push_back((m.ptr, node));
                }

                if let Some(m) = &modifier.p_on_deactivate_modifier {
                    queue.push_back((m.ptr, node));
                }
            }
            Some(HkObject::PoseMatchingGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.parent.node.name);
                graph.add_edge(parent, node, "");

                for children in &*generator.parent.children {
                    if let Some(g) = &children.generator {
                        queue.push_back((g.ptr, node));
                    }
                }
            }
            Some(HkObject::PoweredRagdollControlsModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::TimerModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BehaviorReferenceGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSAssignVariablesModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSEventOnDeactivateModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::LayerGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                for layer in &*generator.layers {
                    queue.push_back((layer.generator.ptr, node));
                }
            }
            Some(HkObject::BSEventEveryNEventsModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSCyclicBlendTransitionGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                if let Some(g) = &generator.p_blender_generator {
                    queue.push_back((g.ptr, node));
                }
            }
            Some(HkObject::BSiStateTaggingGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");

                if let Some(g) = &generator.p_default_generator {
                    queue.push_back((g.ptr, node));
                }
            }
            Some(HkObject::HandIkControlsModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::GetHandleOnBoneModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSTimerModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSDirectAtModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::MirrorModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSEventOnFalseToTrueModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSDirectAtCapturePoseModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::ReferencePoseGenerator(generator)) => {
                let node = graph.add_node(&generator.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSRandomAlarmModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            Some(HkObject::BSRootTwistModifier(modifier)) => {
                let node = graph.add_node(&modifier.parent.node.name);
                graph.add_edge(parent, node, "");
            }
            x => {
                dbg!(x);
                todo!();
            }
        }
    }

    std::fs::write("graph.dot", Dot::new(&graph).to_string().as_bytes()).unwrap();
}

fn load_initial_variable_set(graph: &BehaviorGraph) -> VariableSet {
    let mut set = VariableSet::default();

    if let Some(v) = &graph.data.variable_initial_values {
        // assert!(v.quad_variable_values.is_empty(), "TODO");
    }

    if let Some(v) = &graph.data.variable_initial_values {
        // assert!(v.variant_variable_values.is_empty(), "TODO");
    }

    assert_eq!(
        graph.data.variable_infos.len(),
        graph.data.string_data.variable_names.len()
    );

    let mut word_values = graph
        .data
        .variable_initial_values
        .iter()
        .map(|v| v.word_variable_values.iter())
        .flatten();
    let mut quad_values = graph
        .data
        .variable_initial_values
        .iter()
        .map(|v| v.quad_variable_values.iter())
        .flatten();

    for (index, (info, name)) in graph
        .data
        .variable_infos
        .iter()
        .zip(graph.data.string_data.variable_names.iter())
        .enumerate()
    {
        let value = match info.type_ {
            VariableType::Invalid => {
                word_values.next();
                Value::None
            }
            VariableType::Bool => {
                let val = word_values.next().map(|v| v.value).unwrap_or_default();
                Value::Bool(val != 0)
            }
            VariableType::Int8 => {
                let val = word_values.next().map(|v| v.value).unwrap_or_default();
                Value::I8(val as i8)
            }
            VariableType::Int16 => {
                let val = word_values.next().map(|v| v.value).unwrap_or_default();
                Value::I16(val as i16)
            }
            VariableType::Int32 => {
                let val = word_values.next().map(|v| v.value).unwrap_or_default();
                Value::I32(val)
            }
            VariableType::Real => {
                let val = word_values.next().map(|v| v.value).unwrap_or_default();
                Value::F32(f32::from_bits(val as u32))
            }
            VariableType::Pointer => {
                let val = word_values.next().map(|v| v.value).unwrap_or_default();
                Value::Pointer(val as u32)
            }
            VariableType::Vector3 => {
                let val = quad_values.next().copied().unwrap_or_default();
                Value::Vec3(val.truncate())
            }
            VariableType::Vector4 => {
                let val = quad_values.next().copied().unwrap_or_default();
                Value::Vec4(val)
            }
            VariableType::Quaternion => {
                let val = quad_values.next().copied().unwrap_or(Vec4::W);
                Value::Quat(Quat::from_vec4(val))
            }
        };

        let key = set.variables.insert(value);
        set.variables_by_index.insert(index, key);
        set.variables_by_name.insert(name.clone(), key);
    }

    for (index, (info, name)) in graph
        .data
        .character_property_infos
        .iter()
        .zip(graph.data.string_data.character_property_names.iter())
        .enumerate()
    {
        let value = match info.type_ {
            VariableType::Invalid => Value::None,
            VariableType::Bool => Value::Bool(false),
            VariableType::Int8 => Value::I8(0),
            VariableType::Int16 => Value::I16(0),
            VariableType::Int32 => Value::I32(0),
            VariableType::Real => Value::F32(0.0),
            VariableType::Pointer => Value::Pointer(0),
            VariableType::Vector3 => Value::Vec3(Vec3::ZERO),
            VariableType::Vector4 => Value::Vec4(Vec4::ZERO),
            VariableType::Quaternion => Value::Quat(Quat::IDENTITY),
        };

        let key = set.variables.insert(value);
        set.character_property_by_index.insert(index, key);
        set.character_property_by_name.insert(name.clone(), key);
    }

    set
}

// fn process_node(f: &HkxFile, ptr: OpaquePtr) -> Option<Node> {
//     match f.get(ptr) {
//         Some(HkObject::StateMachine(node)) => {
//             let mut states = Vec::new();
//             let mut transitions = Vec::new();
//             let mut state_id_to_index = HashMap::new();

//             for state in &*node.states {
//                 let Some(generator) = &state.generator else {
//                     continue;
//                 };

//                 if let Some(s) = process_node(f, generator.ptr) {
//                     let index = states.len();
//                     state_id_to_index.insert(state.state_id, index);

//                     states.push(Box::new(s));
//                 }
//             }

//             for state in &*node.states {
//                 let Some(transition) = &state.transitions else {
//                     continue;
//                 };

//                 let Some(from) = state_id_to_index.get(&state.state_id).copied() else {
//                     continue;
//                 };

//                 for transition in &*transition.transitions {
//                     let Some(to) = state_id_to_index.get(&transition.to_state_id).copied() else {
//                         continue;
//                     };

//                     transitions.push(Transition {
//                         from: Some(from),
//                         to,
//                         event_id: transition.event_id as i64,
//                         priority: transition.priority as i64,
//                     });
//                 }
//             }

//             if let Some(t) = &node.wildcard_transitions {
//                 for transition in &*t.transitions {
//                     let Some(to) = state_id_to_index.get(&transition.to_state_id).copied() else {
//                         continue;
//                     };

//                     transitions.push(Transition {
//                         from: None,
//                         to,
//                         event_id: transition.event_id as i64,
//                         priority: transition.priority as i64,
//                     });
//                 }
//             }

//             let start_state = match node.start_state_mode {
//                 StartStateMode::Default => match state_id_to_index.get(&node.start_state_id) {
//                     Some(index) => StartState::Index(*index),
//                     None => StartState::None,
//                 },
//                 StartStateMode::Random => StartState::Random,
//                 _ => todo!(),
//             };

//             Some(Node::StateMachine(StateMachine {
//                 states,
//                 transitions,
//                 start_state,
//             }))
//         }
//         Some(HkObject::ClipGenerator(node)) => {
//             let mut triggers = Vec::new();
//             if let Some(t) = &node.triggers {
//                 for trigger in &*t.triggers {
//                     let event = parse_event(f, &trigger.event);

//                     triggers.push(ClipTrigger {
//                         local_time: trigger.local_time,
//                         from_end: trigger.relative_to_end_of_clip,
//                         event,
//                     });
//                 }
//             }

//             Some(Node::Clip(Clip {
//                 path: node.animation_name.clone(),
//                 triggers,
//                 looping: matches!(node.mode, PlaybackMode::Looping),
//             }))
//         }
//         Some(HkObject::ModifierGenerator(node)) => {
//             let mut list = Vec::new();
//             if let Some(modifier) = process_node(f, node.modifier.ptr) {
//                 list.push(modifier);
//             }

//             if let Some(generator) = process_node(f, node.generator.ptr) {
//                 list.push(generator);
//             }

//             Some(Node::List(list))
//         }
//         Some(HkObject::ModifierList(node)) => {
//             let mut list = Vec::new();

//             for modifier in &*node.modifiers {
//                 if let Some(m) = process_node(f, modifier.ptr) {
//                     list.push(m);
//                 }
//             }

//             Some(Node::List(list))
//         }
//         Some(HkObject::BSAssignVariablesModifier(node)) => None,
//         Some(HkObject::BSIsActiveModifier(node)) => None,
//         Some(HkObject::BSDirectAtModifier(node)) => None,
//         Some(HkObject::BlenderGenerator(node)) => {
//             let mut list = Vec::new();

//             for children in &*node.children {
//                 let Some(generator) = &children.generator else {
//                     continue;
//                 };

//                 if let Some(node) = process_node(f, generator.ptr) {
//                     list.push(node);
//                 }
//             }

//             Some(Node::List(list))
//         }
//         x => todo!("{:?}", x),
//         _ => None,
//     }
// }

fn parse_event(f: &HkxFile, event: &EventProperty) -> Event {
    let payload = match &event.base.payload {
        Some(payload) => match f.get(payload.ptr) {
            Some(HkObject::StringEventPayload(payload)) => Value::String(payload.data.clone()),
            _ => todo!(),
        },
        None => Value::None,
    };

    Event {
        id: event.base.id as i64,
        payload,
    }
}

type NodeId = usize;

#[derive(Clone, Debug)]
pub struct AnimationGraph {
    nodes: Slab<Node>,
    root: Option<NodeId>,
    variables: VariableSet,
    event_names: Vec<String>,
}

impl AnimationGraph {
    pub fn event_name(&self, id: i64) -> Option<&str> {
        self.event_names.get(id as usize).map(|v| v.as_str())
    }

    pub fn graph(&self) -> StableGraph<GraphNode, String> {
        let mut graph = Graph::new();
        let mut queue = VecDeque::new();

        let mut node_to_id = HashMap::new();

        if let Some(root) = &self.root {
            let id = graph.add_node(GraphNode {
                id: *root,
                name: "Root".to_owned(),
            });
            // node_to_id.insert(*root, id);
            queue.push_back((id, root));
        }

        let mut transitions = Vec::new();

        while let Some((parent, node_id)) = queue.pop_front() {
            let node = &self.nodes[*node_id];

            if let Some(id) = node_to_id.get(node_id) {
                graph.add_edge(parent, *id, "".to_owned());
                continue;
            }

            let id = graph.add_node(GraphNode {
                id: *node_id,
                name: node.name.clone(),
            });
            node_to_id.insert(*node_id, id);

            graph.add_edge(parent, id, "".to_owned());

            match &node.inner {
                NodeInner::List(node) => {
                    for node in node {
                        queue.push_back((id, node));
                    }
                }
                NodeInner::StateMachine(node) => {
                    for state in &node.states {
                        queue.push_back((id, &state.node));
                    }

                    for t in &node.transitions {
                        match t.from {
                            Some(from) => {
                                transitions.push((*node_id, from, t.to, t.event_id));
                            }
                            None => {
                                for from in 0..node.states.len() {
                                    transitions.push((*node_id, from, t.to, t.event_id));
                                }
                            }
                        }

                        // match &t.effect {
                        //     TransitionEffect::Node(node) => {
                        //         let t = graph.add_node("[TRANSITION]".to_owned());
                        //         node_to_id.insert(node.node, t);
                        //         graph.add_edge(id, t, "".to_owned());

                        //         queue.push_back((t, &node.node));
                        //     }
                        //     _ => (),
                        // }
                    }
                }
                NodeInner::Clip(_) => {}
                NodeInner::Blend(node) => {
                    for node in &node.children {
                        let t = graph.add_node(GraphNode {
                            id: *node_id,
                            name: "[BLEND_CHILDREN]".to_owned(),
                        });
                        node_to_id.insert(*node_id, id);
                        graph.add_edge(id, t, "".to_owned());

                        queue.push_back((t, &node.node));
                    }
                }
                NodeInner::VariableChangeTracker(_) => {}
                NodeInner::Timer(_) => {}
                NodeInner::Nop => {}
            }
        }

        // for (node, from, to, event_id) in transitions {
        //     let node = match &self.nodes[node] {
        //         Node::StateMachine(node) => node,
        //         _ => unreachable!(),
        //     };

        //     if from == to {
        //         continue;
        //     }

        //     let src = node_to_id[&node.states[from].node];
        //     let dst = node_to_id[&node.states[to].node];
        //     let event = &self.event_names[event_id as usize];
        //     graph.add_edge(src, dst, format!("Event: {}", event));
        // }

        StableGraph::from(graph)
    }

    pub fn animation_files(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|(_, node)| match &node.inner {
                NodeInner::Clip(node) => Some(node.path.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct GraphNode {
    id: usize,
    name: String,
}

#[derive(Clone, Debug, Default)]
struct VariableSet {
    variables: Slab<Value>,
    variables_by_index: HashMap<usize, usize>,
    variables_by_name: HashMap<String, usize>,
    character_property_by_index: HashMap<usize, usize>,
    character_property_by_name: HashMap<String, usize>,
}

impl VariableSet {
    fn get_by_name(&self, name: &str) -> Option<Value> {
        let key = self.variables_by_name.get(name)?;
        self.variables.get(*key).cloned()
    }

    fn get_mut_by_name(&mut self, name: &str) -> Option<&mut Value> {
        let key = self.variables_by_name.get(name)?;
        self.variables.get_mut(*key)
    }

    fn get_mut_by_index(&mut self, index: usize) -> Option<&mut Value> {
        let key = self.variables_by_index.get(&index)?;
        self.variables.get_mut(*key)
    }
}

/// A state machine of one or more states.
///
/// Exactly one state is always active.
#[derive(Clone, Debug)]
pub struct StateMachine {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub start_state: StartState,
    pub state_variable: Option<VariableId>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub node: NodeId,
    /// Events that are fired once this `State` becomes active.
    pub enter_events: Vec<Event>,
    /// Events that are fied once this `State` becomes inactive.
    pub exit_events: Vec<Event>,
}

#[derive(Clone, Debug)]
pub struct Transition {
    /// The source state.
    ///
    /// Can be `None` in which case the transition is always valid.
    pub from: Option<usize>,
    /// The destination state.
    pub to: usize,
    /// The event which will trigger this transition if recevied.
    pub event_id: i64,
    pub priority: i64,
    pub effect: TransitionEffect,
}

#[derive(Clone, Debug)]
pub enum TransitionEffect {
    None,
    Blend(Duration),
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    pub inner: NodeInner,
    pub on_activate: Vec<Command>,
    pub on_deactivate: Vec<Command>,
    pub on_update: Vec<Command>,
    pub listeners: Vec<Listener>,
}

#[derive(Clone, Debug, Default)]
pub enum NodeInner {
    #[default]
    Nop,
    /// A list where all nodes are active.
    List(Vec<NodeId>),
    StateMachine(StateMachine),
    Clip(Clip),
    Blend(Blend),
    VariableChangeTracker(VariableChangeTracker),
    Timer(Timer),
}

#[derive(Clone, Debug)]
pub struct Listener {
    pub event: ListenerEvent,
    pub reaction: Vec<Command>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ListenerEvent {
    Variable(VariableId),
    Event(i64),
}

#[derive(Clone, Debug)]
pub struct Blend {
    /// Contribution of this node including all children.
    pub blend_factor: ValueExpression,
    pub children: Vec<BlendNode>,
    pub is_parametric: bool,
}

#[derive(Clone, Debug)]
pub struct BlendNode {
    pub weight: ValueExpression,
    pub node: NodeId,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub id: i64,
    pub payload: Value,
}

/// An animation clip.
#[derive(Clone, Debug)]
pub struct Clip {
    pub path: String,
    pub triggers: Vec<ClipTrigger>,
    pub looping: bool,
    pub speed: ValueExpression,
}

#[derive(Clone, Debug)]
pub struct ClipTrigger {
    /// The offset from the animation start or end depending on `from_end`.
    ///
    /// If `from_end` is `true` this is the offset from the end in the negative direction.
    pub local_time: Duration,
    pub from_end: bool,
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    F32(f32),
    String(String),
    Pointer(u32),
    Vec3(Vec3),
    Vec4(Vec4),
    Quat(Quat),
}

impl Value {
    fn to_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(val) => Some(*val),
            _ => None,
        }
    }

    fn to_f32(&self) -> Option<f32> {
        match self {
            Self::None => Some(0.0),
            Self::Bool(val) => Some(*val as u8 as f32),
            Self::I8(val) => Some(*val as f32),
            Self::I16(val) => Some(*val as f32),
            Self::I32(val) => Some(*val as f32),
            Self::F32(val) => Some(*val),
            Self::Pointer(_) => None,
            Self::String(_) => None,
            Self::Vec3(_) => None,
            Self::Vec4(_) => None,
            Self::Quat(_) => None,
        }
    }

    fn to_usize(&self) -> Option<usize> {
        match self {
            Self::I8(val) => Some(*val as usize),
            Self::I16(val) => Some(*val as usize),
            Self::I32(val) => Some(*val as usize),
            _ => None,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::None, Self::None) => None,
            (Self::Bool(lhs), Self::Bool(rhs)) => lhs.partial_cmp(rhs),
            (Self::I8(lhs), Self::I8(rhs)) => lhs.partial_cmp(rhs),
            (Self::I16(lhs), Self::I16(rhs)) => lhs.partial_cmp(rhs),
            (Self::I32(lhs), Self::I32(rhs)) => lhs.partial_cmp(rhs),
            (Self::F32(lhs), Self::F32(rhs)) => lhs.partial_cmp(rhs),
            (Self::Pointer(lhs), Self::Pointer(rhs)) => lhs.partial_cmp(rhs),
            (Self::String(lhs), Self::String(rhs)) => lhs.partial_cmp(rhs),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValueKind {
    None,
    Bool,
    I16,
    F32,
    String,
}

#[derive(Clone, Debug)]
pub enum StartState {
    None,
    Index(usize),
    Expression(ValueExpression),
    Random,
}

#[derive(Clone, Debug)]
pub struct SetVariables {
    pub on_activate: Vec<SetVariable>,
    pub on_deactivate: Vec<SetVariable>,
}

#[derive(Clone, Debug)]
pub struct SetVariable {
    pub variable: VariableId,
    #[deprecated]
    pub condition: ValueExpression,
    pub value: ValueExpression,
}

/// An expression that can be evaluated to a [`Value`].
#[derive(Clone, Debug)]
pub enum ValueExpression {
    /// The variable is set to a constant value.
    Const(Value),
    /// The variable is set to the value of another variable.
    Variable(VariableId),
    /// Evaluates to `true` if both expressions are equal, or `false` otherwise.
    ///
    /// Evaluates to a `Bool`.
    CmpEq(Box<ValueExpression>, Box<ValueExpression>),
    /// Evaluates to `false` if both expressions are equal, or `true` otherwise.
    ///
    /// Evaluates to a `Bool`.
    CmpNe(Box<ValueExpression>, Box<ValueExpression>),
    CmpLt(Box<ValueExpression>, Box<ValueExpression>),
    CmpLe(Box<ValueExpression>, Box<ValueExpression>),
    CmpGt(Box<ValueExpression>, Box<ValueExpression>),
    CmpGe(Box<ValueExpression>, Box<ValueExpression>),
    And(Box<ValueExpression>, Box<ValueExpression>),
    Or(Box<ValueExpression>, Box<ValueExpression>),
    Not(Box<ValueExpression>),
}

impl ValueExpression {
    fn eval(&self, set: &VariableSet) -> Value {
        match self {
            Self::Const(val) => val.clone(),
            Self::Variable(id) => set.variables[*id].clone(),
            Self::CmpEq(lhs, rhs) => {
                let lhs = lhs.eval(set);
                let rhs = rhs.eval(set);
                Value::Bool(lhs == rhs)
            }
            Self::CmpNe(lhs, rhs) => {
                let lhs = lhs.eval(set);
                let rhs = rhs.eval(set);
                Value::Bool(lhs != rhs)
            }
            Self::CmpLt(lhs, rhs) => {
                let lhs = lhs.eval(set);
                let rhs = rhs.eval(set);
                Value::Bool(lhs < rhs)
            }
            Self::CmpLe(lhs, rhs) => {
                let lhs = lhs.eval(set);
                let rhs = rhs.eval(set);
                Value::Bool(lhs <= rhs)
            }
            Self::CmpGt(lhs, rhs) => {
                let lhs = lhs.eval(set);
                let rhs = rhs.eval(set);
                Value::Bool(lhs > rhs)
            }
            Self::CmpGe(lhs, rhs) => {
                let lhs = lhs.eval(set);
                let rhs = rhs.eval(set);
                Value::Bool(lhs >= rhs)
            }
            Self::And(lhs, rhs) => {
                let lhs = lhs.eval(set).to_bool().unwrap();
                let rhs = rhs.eval(set).to_bool().unwrap();
                Value::Bool(lhs && rhs)
            }
            Self::Or(lhs, rhs) => {
                let lhs = lhs.eval(set).to_bool().unwrap();
                let rhs = rhs.eval(set).to_bool().unwrap();
                Value::Bool(lhs || rhs)
            }
            Self::Not(value) => Value::Bool(!value.eval(set).to_bool().unwrap()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct VariableChangeTracker {
    pub trackers: Vec<VariableChangeEvent>,
}

#[derive(Clone, Debug)]
pub struct VariableChangeEvent {
    pub expr: ValueExpression,
    pub event: Event,
}

pub type VariableId = usize;

#[derive(Clone, Debug)]
pub struct AnimationSet(pub Arc<HashMap<String, AnimationClip>>);

#[derive(Clone, Debug)]
pub struct PerFrameCommands {
    pub stmts: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub enum Statement {
    SetVariable(SetVariableStmt),
    SendEvent(SendEvent),
}

#[derive(Clone, Debug)]
pub struct SetVariableStmt {
    pub variable: VariableId,
    pub condition: ValueExpression,
    pub value: ValueExpression,
}

#[derive(Clone, Debug)]
pub struct SendEvent {
    pub event: Event,
    pub condition: ValueExpression,
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub min_time: Duration,
    pub max_time: Duration,
    pub event: Event,
    pub repeat: bool,
}

#[derive(Clone, Debug)]
pub enum Command {
    /// Run a list of commands if the condition evaluates to `true`.
    Conditional(ValueExpression, Vec<Command>),
    SetVariable(SetVariable),
    SendEvent(Event),
    /// Activates the node. Does nothing if the node is already active.
    ActivateNode(NodeId),
    /// Deactivate the node. Does nothing if the node is already inactive.
    DeactivateNode(NodeId),
}

impl Command {
    fn run(&self, variables: &mut VariableSet, events: &mut VecDeque<DriverEvent>) {
        match self {
            Self::Conditional(condition, commands) => {
                if condition.eval(&variables).to_bool() == Some(true) {
                    for cmd in commands {
                        cmd.run(variables, events);
                    }
                }
            }
            Self::SetVariable(cmd) => {
                let old_value = &variables.variables[cmd.variable];
                let new_value = cmd.value.eval(&variables);

                if old_value != &new_value {
                    variables.variables[cmd.variable] = cmd.value.eval(&variables);
                    events.push_back(DriverEvent::VariableChanged(cmd.variable));
                }
            }
            Self::SendEvent(cmd) => {
                events.push_back(DriverEvent::HandleEvent(cmd.clone()));
            }
            Self::ActivateNode(id) => {
                events.push_back(DriverEvent::ActivateNode(*id));
            }
            Self::DeactivateNode(id) => {
                events.push_back(DriverEvent::DeactivateNode(*id));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum DriverEvent {
    UpdateNode(NodeId),
    HandleEvent(Event),
    ActivateNode(NodeId),
    DeactivateNode(NodeId),
    VariableChanged(VariableId),
}
