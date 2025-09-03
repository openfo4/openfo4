use std::collections::HashMap;
use std::time::Duration;

use openfo4_hkx::HkObject;
use openfo4_hkx::behavior::{GeneratorTransitionEffect, StartStateMode};
use openfo4_hkx::skeleton::BindingType;
use slab::Slab;

use crate::builder::{
    BuilderContext, Dependency, UnknownVariableName, parse_condition, parse_event,
};
use crate::nodes::state_machine::{State, StateMachine, Transition};
use crate::nodes::{Node, NodeInner};
use crate::{ClipTrigger, Event, NodeId, StartState, Value, ValueExpression};

use super::{BuildNode, Error};

impl BuildNode for openfo4_hkx::behavior::StateMachine {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        let mut children = Vec::new();

        let mut transitions = Vec::new();

        for state in &*self.states {
            let Some(generator) = &state.generator else {
                continue;
            };

            children.push(Dependency::Node(generator.ptr));

            if let Some(t) = &state.transitions {
                for t in &*t.transitions {
                    if let Some(effect) = &t.transition {
                        transitions.push(effect.ptr);
                    }
                }
            }
        }

        if let Some(t) = &self.wildcard_transitions {
            for t in &*t.transitions {
                if let Some(effect) = &t.transition {
                    transitions.push(effect.ptr);
                }
            }
        }

        for ptr in transitions {
            match ctx.hkx_file().get(ptr) {
                Some(HkObject::BlendingTransitionEffect(_)) => (),
                Some(HkObject::GeneratorTransitionEffect(effect)) => {
                    if let Some(ptr) = &effect.transition_generator {
                        children.push(Dependency::Node(ptr.ptr));
                    }
                }
                _ => todo!(),
            }
        }

        Ok(children)
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut start_state_id = ValueExpression::Const(Value::I32(self.start_state_id));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "startStateId" => {
                        dbg!(&self.parent.node.name);
                        start_state_id = ValueExpression::Variable(variable)
                    }
                    "isActive" => {}
                    "currentStateId" => {}
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbStateMachine",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut states = Vec::new();
        let mut transitions = Vec::new();
        let mut state_id_to_index = HashMap::new();

        for state in &*self.states {
            if let Some(set) = &state.bindable.variable_binding_set {
                for binding in &*set.bindings {
                    match &*binding.member_path {
                        name => {
                            return Err(Error::UnknownVariableName(UnknownVariableName {
                                class: "hkbStateMachineStateInfo",
                                name: name.to_owned(),
                            }));
                        }
                    }
                }
            }

            let node = state
                .generator
                .as_ref()
                .and_then(|generator| ctx.get_node(generator.ptr));

            let mut enter_events = Vec::new();
            if let Some(events) = &state.enter_notify_events {
                for event in &*events.events {
                    enter_events.push(parse_event(ctx, event));
                }
            }

            let mut exit_events = Vec::new();
            if let Some(events) = &state.exit_notify_events {
                for event in &*events.events {
                    exit_events.push(parse_event(ctx, event));
                }
            }

            let index = states.len();
            state_id_to_index.insert(state.state_id, index);
            states.push(State {
                id: state.state_id,
                node,
                enter_events,
                exit_events,
            });
        }

        let mut transition_effects = Vec::new();

        for state in &*self.states {
            let Some(transition) = &state.transitions else {
                continue;
            };

            let Some(from) = state_id_to_index.get(&state.state_id).copied() else {
                continue;
            };

            for transition in &*transition.transitions {
                let Some(to) = state_id_to_index.get(&transition.to_state_id).copied() else {
                    continue;
                };

                let condition = transition
                    .condition
                    .as_ref()
                    .map(|ptr| parse_condition(&ctx, ptr.ptr))
                    .unwrap_or(ValueExpression::Const(Value::Bool(true)));

                let event_id = ctx.get_event(transition.event_id).unwrap();

                transitions.push(Transition {
                    condition,
                    from: Some(from),
                    to,
                    event_id,
                    priority: transition.priority as i64,
                    blend_duration: Duration::ZERO,
                });
                transition_effects.push(&transition.transition);
            }
        }

        if let Some(t) = &self.wildcard_transitions {
            for transition in &*t.transitions {
                let Some(to) = state_id_to_index.get(&transition.to_state_id).copied() else {
                    continue;
                };

                let condition = transition
                    .condition
                    .as_ref()
                    .map(|ptr| parse_condition(&ctx, ptr.ptr))
                    .unwrap_or(ValueExpression::Const(Value::Bool(true)));

                let event_id = ctx.get_event(transition.event_id).unwrap();

                transitions.push(Transition {
                    condition,
                    from: None,
                    to,
                    event_id,
                    priority: transition.priority as i64,
                    blend_duration: Duration::ZERO,
                });
                transition_effects.push(&transition.transition);
            }
        }

        // Sort the transitions by priority (descending).
        // This means when we select a transition we can always
        // choose the first one, as that has the highest priority.
        transitions.sort_by(|a, b| a.priority.cmp(&b.priority).reverse());

        // There are three options for state transition effects:
        // No effect, A blending effect or a custom generator.
        // Our StateMachine supports blending effects and we can
        // implement custom generator blending effects by inserting
        // a new state that represents the transition generator.
        let mut generator_effects = Vec::new();

        for (index, (transition, effect)) in
            transitions.iter_mut().zip(transition_effects).enumerate()
        {
            match effect {
                Some(effect) => match ctx.hkx_file().get(effect.ptr) {
                    Some(HkObject::BlendingTransitionEffect(effect)) => {
                        let duration = Duration::from_secs_f32(effect.duration);

                        if !duration.is_zero() {
                            transition.blend_duration = duration;
                        }
                    }
                    Some(HkObject::GeneratorTransitionEffect(effect)) => {
                        generator_effects.push((index, effect));
                    }
                    _ => todo!(),
                },
                None => (),
            }
        }

        for (index, effect) in generator_effects {
            generator_transition(ctx, effect, &mut states, &mut transitions, index);
        }

        let mut state_variable = None;

        let start_state = match (self.start_state_mode, start_state_id) {
            (StartStateMode::Default, ValueExpression::Const(Value::I32(start_state_id))) => {
                match state_id_to_index.get(&start_state_id).copied() {
                    Some(index) => StartState::Index(index),
                    None => StartState::None,
                }
            }
            (StartStateMode::Random, _) => StartState::Random,
            (StartStateMode::Sync, ValueExpression::Const(_)) => {
                let variable = ctx
                    .get_variable(BindingType::Variable, self.sync_variable_index)
                    .unwrap();

                state_variable = Some(variable);
                StartState::Expression(ValueExpression::Variable(variable))
            }
            (StartStateMode::Sync, expr) => {
                let variable = ctx
                    .get_variable(BindingType::Variable, self.sync_variable_index)
                    .unwrap();

                state_variable = Some(variable);
                StartState::Expression(expr)
            }
            (StartStateMode::Default, expr) => StartState::Expression(expr),
            _ => todo!(),
        };

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::StateMachine(StateMachine {
                name: self.parent.node.name.clone(),
                states,
                state_id_to_index,
                transitions,
                start_state,
                state_variable,
                active_state: None,
                active_effect: None,
            }),
        }))
    }
}

fn generator_transition(
    ctx: &mut BuilderContext<'_>,
    generator: &GeneratorTransitionEffect,
    states: &mut Vec<State>,
    transitions: &mut Vec<Transition>,
    original_transition: usize,
) {
    let blend_in = Duration::from_secs_f32(generator.blend_in_duration);
    let blend_out = Duration::from_secs_f32(generator.blend_out_duration);

    let transition_generator = generator.transition_generator.as_ref().unwrap();

    let node = ctx.get_node(transition_generator.ptr).unwrap();

    let exit_event = ctx.create_event("internal_transition_event".to_owned());

    // Find the leaf nodes in the generator (usually animation clips)
    // and insert an event that will cause our state to transition forward.
    for node in find_leaf_nodes(&ctx.state.nodes, node) {
        match &mut ctx.state.nodes[node].inner {
            NodeInner::Clip(clip) => {
                clip.triggers.push(ClipTrigger {
                    local_time: blend_out,
                    from_end: true,
                    event: Event {
                        id: exit_event as i64,
                        payload: Value::None,
                    },
                });
            }
            _ => (),
        }
    }

    // Register the new state for the transition.
    let transition_state = states.len();
    states.push(State {
        id: i32::MAX,
        node: Some(node),
        enter_events: Vec::new(),
        exit_events: Vec::new(),
    });

    // Redirect the original transition to our temporary transition
    // state.
    let original_transition = &mut transitions[original_transition];
    let to = original_transition.to;
    original_transition.to = transition_state;
    original_transition.blend_duration = blend_in;

    transitions.push(Transition {
        condition: ValueExpression::Const(Value::Bool(true)),
        from: Some(transition_state),
        to,
        event_id: exit_event as i64,
        priority: 0,
        blend_duration: blend_out,
    });
}

fn find_leaf_nodes(nodes: &Slab<Node>, root: NodeId) -> Vec<NodeId> {
    let mut queue = vec![root];
    let mut leaves = Vec::new();

    while let Some(id) = queue.pop() {
        let is_leaf = match &nodes[id].inner {
            NodeInner::StateMachine(node) => {
                for state in &node.states {
                    if let Some(node) = state.node {
                        queue.push(node);
                    }
                }

                false
            }
            NodeInner::List(list) => {
                queue.extend(&list.nodes);

                false
            }
            NodeInner::Blend(node) => {
                for c in &node.children {
                    if let Some(node) = c.node {
                        queue.push(node);
                    }
                }

                false
            }
            NodeInner::ManualSelector(node) => {
                for node in node.states.values() {
                    queue.push(*node);
                }

                false
            }
            NodeInner::Layers(node) => {
                for layer in &node.layers {
                    queue.push(layer.node);
                }

                false
            }
            // Technically has children but they are only active
            // for a single frame so we cannot insert meaningful triggers.
            NodeInner::ModifyOnce(_) => false,
            NodeInner::Clip(_) => true,
            NodeInner::Timer(_) => true,
            NodeInner::AssignVariables(_) => true,
            NodeInner::EventDriven(_) => true,
            NodeInner::IsActive(_) => true,
            NodeInner::EventsFromRange(_) => true,
        };

        if is_leaf {
            leaves.push(id);
        }
    }

    leaves
}
