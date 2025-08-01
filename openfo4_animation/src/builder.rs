mod behavior_reference_generator;
mod blender_generator;
mod bs_assign_variables_modifier;
mod bs_event_on_deactivate_modifier;
mod bs_events_every_n_events_modifier;
mod bs_is_active_modifier;
mod bs_modify_once_modifier;
mod bs_offset_animation_generator;
mod bs_random_alarm_modifier;
mod bs_timer_modifier;
mod bsi_state_tagging_generator;
mod clip_generator;
mod event_driven_modifier;
mod events_from_range_modifier;
mod layer_generator;
mod manual_selector_generator;
mod pose_matching_generator;
mod state_machine;
mod timer_modifier;

use std::collections::HashMap;
use std::fmt::Pointer;
use std::hash::Hash;
use std::time::Duration;

use openfo4_hkx::behavior::{BSBehaviorGraphSwapGenerator, PlaybackMode, PoseMatchingGenerator};
use openfo4_hkx::skeleton::BindingType;
use openfo4_hkx::{HkObject, HkxFile, OpaquePtr};
use slab::Slab;
use thiserror::Error;

use crate::{
    AnimationGraph, Blend, BlendNode, Clip, ClipTrigger, Node, NodeId, NodeInner, SetVariable,
    SetVariables, Value, ValueExpression, ValueKind, VariableId, VariableSet,
    load_initial_variable_set, parse_event,
};

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("file contains no BehaviorGraph class")]
    NoGraph,
    #[error(transparent)]
    UnknownVariableName(UnknownVariableName),
    #[error("unknown variable id: {0}")]
    UnknownVariableId(VariableId),
    #[error("unknown variable id: {0}")]
    UnknownEventId(i64),
    #[error("undeclared variable {0}")]
    UndeclaredVariable(String),
}

#[derive(Copy, Clone, Debug)]
pub enum Hint<'a> {
    NeedFile(&'a str),
}

#[derive(Clone, Debug)]
pub enum Dependency<'a> {
    File(&'a str),
}

pub struct Builder<'a> {
    root: &'a HkxFile,
    props: HashMap<String, Value>,
    pointees: HashMap<u32, Value>,
}

impl<'a> Builder<'a> {
    pub fn new(root: &'a HkxFile) -> Self {
        Self {
            root,
            props: HashMap::new(),
            pointees: HashMap::new(),
        }
    }

    /// Registers a property from the character file.
    ///
    /// The `name` is considered case-insensitive.
    pub fn register_character_property(&mut self, name: &str, value: Value) {
        let name = name.to_lowercase();
        self.props.insert(name, value);
    }

    pub fn build(&mut self) -> Result<Result<AnimationGraph, Hint<'_>>, Error> {
        let Some(root) = self.root.iter().find_map(|obj| match obj {
            HkObject::BehaviorGraph(node) => Some(node),
            _ => None,
        }) else {
            return Err(Error::NoGraph);
        };

        let g = animation_graph(&self.root, &self.props);
        Ok(Ok(g))
    }
}

#[derive(Clone, Debug, Error)]
#[error("unknown variable name {name} for class {class}")]
pub struct UnknownVariableName {
    pub class: &'static str,
    pub name: String,
}

pub fn animation_graph(f: &HkxFile, props: &HashMap<String, Value>) -> AnimationGraph {
    let mut root = None;
    for obj in f.iter() {
        match obj {
            HkObject::BehaviorGraph(node) => {
                root = Some(node);
                break;
            }
            _ => (),
        }
    }

    let graph = root.unwrap();

    let mut variables = load_initial_variable_set(graph, props);

    let event_names: Vec<_> = graph.data.string_data.event_names.iter().cloned().collect();

    let mut state = GraphBuilderState {
        nodes: Slab::new(),
        file: f,
        ptr_to_id: HashMap::new(),
        variables: &mut variables,
        events: event_names,
    };

    let root = if let Some(generator) = &graph.root_generator {
        let mut stack = vec![generator.ptr];
        let mut node_children = HashMap::new();
        let mut nodes_ready = Vec::new();

        while let Some(ptr) = stack.pop() {
            let children = match f.get(ptr) {
                Some(object) => object.children(&mut state).unwrap(),
                None => todo!(),
            };

            if children.iter().all(|ptr| state.ptr_to_id.contains_key(ptr)) {
                nodes_ready.push(ptr);
            } else {
                stack.extend(&children);
                node_children.insert(ptr, children);
            }

            while let Some(ptr) = nodes_ready.pop() {
                let node = match f.get(ptr) {
                    Some(object) => object.build(&mut state).unwrap(),
                    None => todo!(),
                };

                if let Some(node) = node {
                    let id = state.nodes.insert(node);
                    state.ptr_to_id.insert(ptr, id);
                }

                node_children.retain(|parent, children| {
                    children.retain(|p| *p != ptr);
                    if children.is_empty() {
                        nodes_ready.push(*parent);
                        false
                    } else {
                        true
                    }
                });
            }
        }

        let id = *state.ptr_to_id.get(&generator.ptr).unwrap();
        Some(id)
    } else {
        None
    };

    let mut graph = AnimationGraph {
        nodes: state.nodes,
        root,
        event_names: state.events,
        variables,
    };

    graph
}

#[derive(Debug)]
struct GraphBuilderState<'a> {
    nodes: Slab<Node>,
    file: &'a HkxFile,
    ptr_to_id: HashMap<OpaquePtr, NodeId>,
    variables: &'a mut VariableSet,
    events: Vec<String>,
}

impl<'a> GraphBuilderState<'a> {
    fn get_variable_id(
        &self,
        binding_type: BindingType,
        variable_index: i32,
    ) -> Option<VariableId> {
        match binding_type {
            BindingType::Variable => self
                .variables
                .variables_by_index
                .get(&(variable_index as usize))
                .copied(),
            BindingType::CharacterProperty => self
                .variables
                .character_property_by_index
                .get(&(variable_index as usize))
                .copied(),
        }
    }

    fn get_event_id(&self, index: i32) -> Option<i64> {
        if index >= 0 && (index as usize) < self.events.len() {
            Some(index as i64)
        } else {
            None
        }
    }

    fn create_variable(&mut self, value: Value) -> VariableId {
        self.variables.variables.insert(value)
    }
}

trait BuildNode {
    /// Returns a list of nodes that must be visited before the node can be built.
    fn children(&self, state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(Vec::new())
    }

    fn dependencies(&self, state: &mut GraphBuilderState<'_>) {}

    /// Build this node.
    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error>;
}

impl BuildNode for HkObject {
    fn children(&self, state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        match self {
            Self::StateMachine(node) => node.children(state),
            Self::ModifierGenerator(node) => node.children(state),
            Self::ModifierList(node) => node.children(state),
            Self::BlenderGenerator(node) => node.children(state),
            Self::ClipGenerator(node) => node.children(state),
            Self::BSIsActiveModifier(node) => node.children(state),
            Self::BSAssignVariablesModifier(node) => node.children(state),
            Self::BSDirectAtModifier(node) => Ok(Vec::new()),
            Self::PoseMatchingGenerator(node) => node.children(state),
            Self::TimerModifier(node) => node.children(state),
            Self::BSRagdollContactListenerModifier(node) => Ok(Vec::new()),
            Self::PoweredRagdollControlsModifier(node) => Ok(Vec::new()),
            Self::ManualSelectorGenerator(node) => node.children(state),
            Self::EventDrivenModifier(node) => node.children(state),
            Self::BSModifyOnceModifier(node) => node.children(state),
            Self::RigidBodyRagdollControlsModifier(node) => Ok(Vec::new()),
            Self::KeyframesBonesModifier(node) => Ok(Vec::new()),
            Self::GetUpModifier(node) => Ok(Vec::new()),
            Self::EvaluateExpressionModifier(node) => Ok(Vec::new()),
            Self::BSOffsetAnimationGenerator(node) => node.children(state),
            Self::FootIkControlsModifier(node) => Ok(Vec::new()),
            Self::BehaviorReferenceGenerator(node) => node.children(state),
            Self::LayerGenerator(node) => node.children(state),
            Self::BSBehaviorGraphSwapGenerator(node) => node.children(state),
            Self::MirrorModifier(node) => Ok(Vec::new()),
            Self::BSEventEveryNEventsModifier(node) => Ok(Vec::new()),
            Self::EventsFromRangeModifier(node) => node.children(state),
            Self::BSEventOnDeactivateModifier(node) => node.children(state),
            Self::BSiStateTaggingGenerator(node) => node.children(state),
            Self::BSCyclicBlendTransitionGenerator(node) => Ok(Vec::new()),
            Self::BSTimerModifier(node) => node.children(state),
            Self::BSRandomAlarmModifier(node) => node.children(state),
            Self::BSInterpValueModifier(node) => Ok(Vec::new()),
            Self::BSEventOnFalseToTrueModifier(node) => Ok(Vec::new()),
            Self::TwistModifier(node) => Ok(Vec::new()),
            Self::BSDirectAtCapturePoseModifier(node) => Ok(Vec::new()),
            Self::ReferencePoseGenerator(node) => Ok(Vec::new()),
            Self::GetHandleOnBoneModifier(node) => Ok(Vec::new()),
            Self::HandIkControlsModifier(node) => Ok(Vec::new()),
            Self::BSRootTwistModifier(node) => Ok(Vec::new()),
            Self::BSLookAtModifier(node) => Ok(Vec::new()),
            Self::BSGetTimeStepModifier(node) => Ok(Vec::new()),
            x => todo!("{:?}", x),
            _ => todo!(),
        }
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        match self {
            Self::StateMachine(node) => node.build(state),
            Self::ModifierGenerator(node) => node.build(state),
            Self::ModifierList(node) => node.build(state),
            Self::BlenderGenerator(node) => node.build(state),
            Self::ClipGenerator(node) => node.build(state),
            Self::BSIsActiveModifier(node) => node.build(state),
            Self::BSAssignVariablesModifier(node) => node.build(state),
            Self::BSDirectAtModifier(node) => Ok(None),
            Self::PoseMatchingGenerator(node) => node.build(state),
            Self::TimerModifier(node) => node.build(state),
            Self::BSRagdollContactListenerModifier(node) => Ok(None),
            Self::PoweredRagdollControlsModifier(node) => Ok(None),
            Self::ManualSelectorGenerator(node) => node.build(state),
            Self::EventDrivenModifier(node) => node.build(state),
            Self::BSModifyOnceModifier(node) => node.build(state),
            Self::RigidBodyRagdollControlsModifier(node) => Ok(None),
            Self::KeyframesBonesModifier(node) => Ok(None),
            Self::GetUpModifier(node) => Ok(None),
            Self::EvaluateExpressionModifier(node) => Ok(None),
            Self::BSOffsetAnimationGenerator(node) => node.build(state),
            Self::FootIkControlsModifier(node) => Ok(None),
            Self::BehaviorReferenceGenerator(node) => node.build(state),
            Self::LayerGenerator(node) => node.build(state),
            Self::BSBehaviorGraphSwapGenerator(node) => node.build(state),
            Self::MirrorModifier(node) => Ok(None),
            Self::BSEventEveryNEventsModifier(node) => node.build(state),
            Self::EventsFromRangeModifier(node) => node.build(state),
            Self::BSEventOnDeactivateModifier(node) => node.build(state),
            Self::BSiStateTaggingGenerator(node) => node.build(state),
            Self::BSCyclicBlendTransitionGenerator(node) => Ok(None),
            Self::BSTimerModifier(node) => node.build(state),
            Self::BSRandomAlarmModifier(node) => node.build(state),
            Self::BSInterpValueModifier(node) => Ok(None),
            Self::BSEventOnFalseToTrueModifier(node) => Ok(None),
            Self::TwistModifier(node) => Ok(None),
            Self::BSDirectAtCapturePoseModifier(node) => Ok(None),
            Self::ReferencePoseGenerator(node) => Ok(None),
            Self::GetHandleOnBoneModifier(node) => Ok(None),
            Self::HandIkControlsModifier(node) => Ok(None),
            Self::BSRootTwistModifier(node) => Ok(None),
            Self::BSLookAtModifier(node) => Ok(None),
            Self::BSGetTimeStepModifier(node) => Ok(None),
            _ => todo!(),
        }
    }
}

impl BuildNode for openfo4_hkx::behavior::ModifierGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(vec![self.modifier.ptr, self.generator.ptr])
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut list = Vec::new();

        if let Some(id) = state.ptr_to_id.get(&self.modifier.ptr) {
            list.push(*id);
        }

        if let Some(id) = state.ptr_to_id.get(&self.generator.ptr) {
            list.push(*id);
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(list),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        }))
    }
}

impl BuildNode for openfo4_hkx::behavior::ModifierList {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(self.modifiers.iter().map(|m| m.ptr).collect())
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut list = Vec::new();

        for modifier in &*self.modifiers {
            if let Some(id) = state.ptr_to_id.get(&modifier.ptr) {
                list.push(*id);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(list),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        }))
    }
}

impl BuildNode for BSBehaviorGraphSwapGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(self.p_default_generator.iter().map(|v| v.ptr).collect())
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut nodes = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            if let Some(node) = state.ptr_to_id.get(&generator.ptr).copied() {
                nodes.push(node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(nodes),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        }))
    }
}
