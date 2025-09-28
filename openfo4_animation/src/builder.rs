mod behavior_reference_generator;
mod blender_generator;
mod bs_assign_variables_modifier;
mod bs_cyclic_blend_transition_generator;
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
mod modifier_generator;
mod modifier_list;
mod pose_matching_generator;
mod state_machine;
mod timer_modifier;

use std::collections::HashMap;
use std::hash::Hash;

use openfo4_hkx::behavior::{
    BSBehaviorGraphSwapGenerator, DynamicAnimationTaggingGenerator, EventProperty,
};
use openfo4_hkx::skeleton::BindingType;
use openfo4_hkx::{HkObject, HkxFile, OpaquePtr};
use slab::Slab;
use thiserror::Error;

use crate::events::{EventId, EventSet};
use crate::nodes::Node;
use crate::nodes::list::List;
use crate::variables::{VariableKind, VariableSet};
use crate::{
    AnimationGraph, Event, NodeId, NodeInner, Value, ValueExpression, VariableId,
    load_initial_variable_set,
};

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("file contains no BehaviorGraph class")]
    NoGraph,
    #[error(transparent)]
    UnknownVariableName(UnknownVariableName),
    #[error("unknown variable id: {0:?}")]
    UnknownVariableId(VariableId),
    #[error("unknown variable id: {0}")]
    UnknownEventId(i64),
    #[error("undeclared variable {0}")]
    UndeclaredVariable(String),
}

#[derive(Clone, Debug)]
pub enum Hint {
    NeedFile(String),
    NeedSwapGenerator,
}

#[derive(Clone, Debug)]
enum Dependency {
    Node(OpaquePtr),
    File(String),
}

pub struct Builder {
    state: BuilderState,
    stack: Vec<(String, RootOrPtr)>,
    hkx_files: HashMap<String, HkxFile>,
    node_deps: HashMap<(String, OpaquePtr), Vec<(String, RootOrPtr)>>,
}

impl Builder {
    pub fn new(file: HkxFile, props: HashMap<String, Value>) -> Result<Self, Error> {
        let props = props
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();

        let mut builder = Self {
            state: BuilderState {
                props,
                nodes: Slab::new(),
                variables: VariableSet::default(),
                files: HashMap::new(),
                events: EventSet::default(),
            },
            hkx_files: HashMap::new(),
            stack: Vec::new(),
            node_deps: HashMap::new(),
        };

        // Empty string represents the root file.
        builder.register_file("", file)?;
        builder.stack.push(("".to_owned(), RootOrPtr::Root));

        Ok(builder)
    }

    /// Registers a property from the character file.
    ///
    /// The `name` is considered case-insensitive.
    pub fn register_character_property(&mut self, name: &str, value: Value) {
        // let name = name.to_lowercase();
        // self.props.insert(name, value);
        // todo!()
    }

    pub fn register_file(&mut self, path: &str, file: HkxFile) -> Result<(), Error> {
        let Some(graph) = file.iter().find_map(|obj| match obj {
            HkObject::BehaviorGraph(node) => Some(node),
            _ => None,
        }) else {
            return Err(Error::NoGraph);
        };

        let variables = load_initial_variable_set(graph, &self.state.props);

        let mut table = IndexTable::default();

        // Variables
        for (index, (name, value)) in variables.variables.into_iter().enumerate() {
            let id = self
                .state
                .variables
                .lookup_variable_name(&name)
                .unwrap_or_else(|| {
                    self.state
                        .variables
                        .insert(VariableKind::Variable, name.clone(), value)
                });

            table.variable_by_index.insert(index as u32, id);
            table.variable_by_name.insert(name, id);
        }

        // Character properties
        for (index, name) in graph
            .data
            .string_data
            .character_property_names
            .iter()
            .enumerate()
        {
            let name = name.to_lowercase();

            let Some(value) = self.state.props.get(&name) else {
                todo!("undeclated variable: {}", name);
            };

            let id = self
                .state
                .variables
                .lookup_character_property_name(&name)
                .unwrap_or_else(|| {
                    self.state.variables.insert(
                        VariableKind::CharacterProperty,
                        name.clone(),
                        value.clone(),
                    )
                });

            table.character_property_by_index.insert(index as u32, id);
            table.character_property_by_name.insert(name, id);
        }

        for (index, (name, value)) in variables.character_properties.into_iter().enumerate() {
            let id = self
                .state
                .variables
                .lookup_character_property_name(&name)
                .unwrap_or_else(|| {
                    self.state.variables.insert(
                        VariableKind::CharacterProperty,
                        name.clone(),
                        value,
                    )
                });

            table.character_property_by_index.insert(index as u32, id);
            table.character_property_by_name.insert(name, id);
        }

        // Events
        for (index, name) in graph.data.string_data.event_names.iter().enumerate() {
            let name = name.to_lowercase();

            let id = self
                .state
                .events
                .lookup_name(&name)
                .unwrap_or_else(|| self.state.events.insert(name.clone()));

            table.event_by_index.insert(index as u32, id);
            table.event_by_name.insert(name, id);
        }

        let root = graph.root_generator.as_ref().map(|ptr| ptr.ptr);

        self.state.files.insert(
            path.to_owned(),
            BuilderFileState {
                ptr_to_id: HashMap::new(),
                index_table: table,
                root,
            },
        );
        self.hkx_files.insert(path.to_owned(), file);

        Ok(())
    }

    pub fn register_swap_generator(&mut self, file: HkxFile) -> Result<(), Error> {
        // FIXME: This is hardcoded to a special file name for now,
        // but we should probably add an enum as the filekey to more
        // clearly express this.
        self.register_file("#SWAP_GENERATOR", file)
    }

    pub fn build(&mut self) -> Result<Result<AnimationGraph, Hint>, Error> {
        let mut nodes_ready = Vec::new();

        while let Some((file, _)) = self.stack.last() {
            let Some(state) = self.state.files.get(file) else {
                if file == "#SWAP_GENERATOR" {
                    return Ok(Err(Hint::NeedSwapGenerator));
                } else {
                    return Ok(Err(Hint::NeedFile(file.clone())));
                }
            };

            let (file, ptr) = self.stack.pop().unwrap();

            let ptr = match ptr {
                RootOrPtr::Root => state.root.unwrap(),
                RootOrPtr::Ptr(ptr) => ptr,
            };

            let hkx_file = self.hkx_files.get(&file).unwrap();
            let object = match hkx_file.get(ptr) {
                Some(object) => object,
                None => todo!(),
            };

            let mut ctx = BuilderContext {
                state: &mut self.state,
                file: &file,
                hkx_file,
            };

            let mut dependencies = object.dependencies(&mut ctx)?;

            // Retain all dependencies that are not already loaded
            // to prevent double loading.
            dependencies.retain(|dep| match dep {
                Dependency::Node(ptr) => {
                    let state = self.state.files.get(&file).unwrap();
                    !state.ptr_to_id.contains_key(ptr)
                }
                Dependency::File(file) => {
                    let Some(state) = self.state.files.get(file) else {
                        return true;
                    };

                    match state.root {
                        Some(root) => !state.ptr_to_id.contains_key(&root),
                        None => false,
                    }
                }
            });

            if dependencies.is_empty() {
                nodes_ready.push((file, ptr));
            } else {
                let deps: Vec<_> = dependencies
                    .into_iter()
                    .map(|dep| match dep {
                        Dependency::Node(ptr) => (file.clone(), RootOrPtr::Ptr(ptr)),
                        Dependency::File(file) => (file, RootOrPtr::Root),
                    })
                    .collect();

                self.node_deps.insert((file, ptr), deps.clone());
                self.stack.extend(deps);
            }

            while let Some((file, ptr)) = nodes_ready.pop() {
                let hkx_file = self.hkx_files.get(&file).unwrap();

                let mut ctx = BuilderContext {
                    state: &mut self.state,
                    file: &file,
                    hkx_file,
                };

                let node = match hkx_file.get(ptr) {
                    Some(object) => object.build(&mut ctx)?,
                    None => unreachable!(),
                };

                let id = node.map(|node| self.state.nodes.insert(node));
                let state = self.state.files.get_mut(&file).unwrap();
                state.ptr_to_id.insert(ptr, id);

                self.node_deps.retain(|parent, deps| {
                    deps.retain(|(file, ptr)| {
                        let Some(state) = self.state.files.get(file) else {
                            return true;
                        };

                        let ptr = match ptr {
                            RootOrPtr::Root => match state.root {
                                Some(root) => root,
                                // If the file has no root no node can be loaded
                                // and we always consider it ready.
                                None => return false,
                            },
                            RootOrPtr::Ptr(ptr) => *ptr,
                        };

                        !state.ptr_to_id.contains_key(&ptr)
                    });

                    if deps.is_empty() {
                        nodes_ready.push(parent.clone());
                        false
                    } else {
                        true
                    }
                });
            }
        }

        debug_assert!(self.node_deps.is_empty());

        let root = self.state.files.get("").unwrap();
        let root = root
            .root
            .and_then(|ptr| root.ptr_to_id.get(&ptr).copied())
            .flatten();

        Ok(Ok(AnimationGraph {
            nodes: self.state.nodes.clone(),
            root,
            variables: self.state.variables.clone(),
            events: self.state.events.clone(),
        }))
    }
}

#[derive(Clone, Debug)]
struct BuilderState {
    nodes: Slab<Node>,
    variables: VariableSet,
    events: EventSet,
    files: HashMap<String, BuilderFileState>,
    props: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
struct BuilderFileState {
    ptr_to_id: HashMap<OpaquePtr, Option<NodeId>>,
    index_table: IndexTable,
    root: Option<OpaquePtr>,
}

struct BuilderContext<'a> {
    file: &'a str,
    hkx_file: &'a HkxFile,
    state: &'a mut BuilderState,
}

impl<'a> BuilderContext<'a> {
    fn hkx_file(&self) -> &'a HkxFile {
        self.hkx_file
    }

    fn variables(&self) -> &VariableSet {
        &self.state.variables
    }

    /// Returns the [`NodeId`] of a node in the current file if it was loaded.
    fn get_node(&self, ptr: OpaquePtr) -> Option<NodeId> {
        let state = self.state.files.get(self.file).unwrap();
        state.ptr_to_id.get(&ptr).copied().flatten()
    }

    fn get_file_root_node(&self, file: &str) -> Option<NodeId> {
        let state = self.state.files.get(file)?;
        state
            .root
            .and_then(|ptr| state.ptr_to_id.get(&ptr).copied())
            .flatten()
    }

    fn get_variable(&self, binding_type: BindingType, variable_index: i32) -> Option<VariableId> {
        let state = self.state.files.get(self.file).unwrap();
        let Ok(index) = u32::try_from(variable_index) else {
            return None;
        };

        match binding_type {
            BindingType::Variable => state.index_table.variable_by_index.get(&index).copied(),
            BindingType::CharacterProperty => state
                .index_table
                .character_property_by_index
                .get(&index)
                .copied(),
        }
    }

    fn get_event(&self, index: i32) -> Option<EventId> {
        let state = self.state.files.get(self.file).unwrap();
        let Ok(index) = u32::try_from(index) else {
            return None;
        };

        state.index_table.event_by_index.get(&index).copied()
    }

    fn create_node(&mut self, node: Node) -> NodeId {
        self.state.nodes.insert(node)
    }

    fn create_event(&mut self, event: String) -> EventId {
        self.state.events.insert(event)
    }

    fn create_variable(&mut self, value: Value) -> VariableId {
        self.state
            .variables
            .insert(VariableKind::Variable, String::new(), value)
    }
}

#[derive(Clone, Debug, Error)]
#[error("unknown variable name {name} for class {class}")]
pub struct UnknownVariableName {
    pub class: &'static str,
    pub name: String,
}

trait BuildNode {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(Vec::new())
    }

    /// Build this node.
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error>;
}

impl BuildNode for HkObject {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        match self {
            Self::StateMachine(node) => node.dependencies(ctx),
            Self::ModifierGenerator(node) => node.dependencies(ctx),
            Self::ModifierList(node) => node.dependencies(ctx),
            Self::BlenderGenerator(node) => node.dependencies(ctx),
            Self::ClipGenerator(node) => node.dependencies(ctx),
            Self::BSIsActiveModifier(node) => node.dependencies(ctx),
            Self::BSAssignVariablesModifier(node) => node.dependencies(ctx),
            Self::BSDirectAtModifier(node) => Ok(Vec::new()),
            Self::PoseMatchingGenerator(node) => node.dependencies(ctx),
            Self::TimerModifier(node) => node.dependencies(ctx),
            Self::BSRagdollContactListenerModifier(node) => Ok(Vec::new()),
            Self::PoweredRagdollControlsModifier(node) => Ok(Vec::new()),
            Self::ManualSelectorGenerator(node) => node.dependencies(ctx),
            Self::EventDrivenModifier(node) => node.dependencies(ctx),
            Self::BSModifyOnceModifier(node) => node.dependencies(ctx),
            Self::RigidBodyRagdollControlsModifier(node) => Ok(Vec::new()),
            Self::KeyframesBonesModifier(node) => Ok(Vec::new()),
            Self::GetUpModifier(node) => Ok(Vec::new()),
            Self::EvaluateExpressionModifier(node) => Ok(Vec::new()),
            Self::BSOffsetAnimationGenerator(node) => node.dependencies(ctx),
            Self::FootIkControlsModifier(node) => Ok(Vec::new()),
            Self::BehaviorReferenceGenerator(node) => node.dependencies(ctx),
            Self::LayerGenerator(node) => node.dependencies(ctx),
            Self::BSBehaviorGraphSwapGenerator(node) => node.dependencies(ctx),
            Self::MirrorModifier(node) => Ok(Vec::new()),
            Self::BSEventEveryNEventsModifier(node) => node.dependencies(ctx),
            Self::EventsFromRangeModifier(node) => node.dependencies(ctx),
            Self::BSEventOnDeactivateModifier(node) => node.dependencies(ctx),
            Self::BSiStateTaggingGenerator(node) => node.dependencies(ctx),
            Self::BSCyclicBlendTransitionGenerator(node) => node.dependencies(ctx),
            Self::BSTimerModifier(node) => node.dependencies(ctx),
            Self::BSRandomAlarmModifier(node) => node.dependencies(ctx),
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
            Self::BSBoneSwitchGenerator(node) => Ok(Vec::new()),
            Self::DynamicAnimationTaggingGenerator(node) => node.dependencies(ctx),
            x => todo!("{:?}", x),
            _ => todo!(),
        }
    }

    fn build(&self, state: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
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
            Self::BSCyclicBlendTransitionGenerator(node) => node.build(state),
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
            Self::BSBoneSwitchGenerator(node) => Ok(None),
            Self::DynamicAnimationTaggingGenerator(node) => node.build(state),
            _ => todo!(),
        }
    }
}

impl BuildNode for BSBehaviorGraphSwapGenerator {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(vec![Dependency::File("#SWAP_GENERATOR".to_owned())])
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut children = Vec::new();

        if let Some(node) = ctx.get_file_root_node("#SWAP_GENERATOR") {
            children.push(node);
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(List { nodes: children }),
        }))
    }
}

impl BuildNode for DynamicAnimationTaggingGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        if let Some(ptr) = &self.p_default_generator {
            Ok(vec![Dependency::Node(ptr.ptr)])
        } else {
            Ok(vec![])
        }
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut children = Vec::new();

        if let Some(ptr) = &self.p_default_generator {
            if let Some(node) = ctx.get_node(ptr.ptr) {
                children.push(node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(List { nodes: children }),
        }))
    }
}

#[derive(Clone, Debug, Default)]
struct IndexTable {
    variable_by_index: HashMap<u32, VariableId>,
    variable_by_name: HashMap<String, VariableId>,
    character_property_by_index: HashMap<u32, VariableId>,
    character_property_by_name: HashMap<String, VariableId>,
    event_by_index: HashMap<u32, EventId>,
    event_by_name: HashMap<String, EventId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum RootOrPtr {
    Ptr(OpaquePtr),
    Root,
}

fn parse_event(ctx: &BuilderContext<'_>, event: &EventProperty) -> Option<Event> {
    if event.base.id < 0 {
        return None;
    }

    let payload = match &event.base.payload {
        Some(payload) => match ctx.hkx_file().get(payload.ptr) {
            Some(HkObject::StringEventPayload(payload)) => Value::String(payload.data.clone()),
            _ => todo!(),
        },
        None => Value::None,
    };

    Some(Event {
        id: ctx.get_event(event.base.id).unwrap(),
        payload,
    })
}

fn parse_condition(ctx: &BuilderContext<'_>, ptr: OpaquePtr) -> ValueExpression {
    match ctx.hkx_file().get(ptr) {
        Some(HkObject::ExpressionCondition(condition)) => {
            crate::expr::parse_expr(&condition.expression, ctx.variables())
        }
        _ => todo!(),
    }
}
