use std::collections::HashMap;
use std::time::Duration;

use openfo4_hkx::HkObject;
use openfo4_hkx::behavior::{GeneratorTransitionEffect, StartStateMode};
use openfo4_hkx::skeleton::BindingType;
use slab::Slab;

use crate::builder::{BuilderContext, Dependency, parse_event};
use crate::{
    ClipTrigger, Event, Node, NodeId, NodeInner, StartState, State, StateMachine, Transition,
    TransitionEffect, Value, ValueExpression,
};

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
        let mut states = Vec::new();
        let mut transitions = Vec::new();
        let mut state_id_to_index = HashMap::new();

        for state in &*self.states {
            // Note that it is possible that the children generator for this state
            // evaluated to no node. In this case we must not skip the state entirely
            // so that events and transitions are still handled correctly.
            // We create a dummy node and use that when this case happens.
            let id = state
                .generator
                .as_ref()
                .and_then(|generator| ctx.get_node(generator.ptr))
                .unwrap_or_else(|| {
                    ctx.create_node(Node {
                        name: state.name.clone(),
                        ..Default::default()
                    })
                });

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
                node: id,
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

                let event_id = ctx.get_event(transition.event_id).unwrap();

                transitions.push(Transition {
                    from: Some(from),
                    to,
                    event_id,
                    priority: transition.priority as i64,
                    effect: TransitionEffect::None,
                });
                transition_effects.push(&transition.transition);
            }
        }

        if let Some(t) = &self.wildcard_transitions {
            for transition in &*t.transitions {
                let Some(to) = state_id_to_index.get(&transition.to_state_id).copied() else {
                    continue;
                };

                let event_id = ctx.get_event(transition.event_id).unwrap();

                transitions.push(Transition {
                    from: None,
                    to,
                    event_id,
                    priority: transition.priority as i64,
                    effect: TransitionEffect::None,
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
                            transition.effect = TransitionEffect::Blend(duration);
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

        let start_state = match self.start_state_mode {
            StartStateMode::Default => match state_id_to_index.get(&self.start_state_id) {
                Some(index) => StartState::Index(*index),
                None => StartState::None,
            },
            StartStateMode::Random => StartState::Random,
            StartStateMode::Sync => {
                let variable = ctx
                    .get_variable(BindingType::Variable, self.sync_variable_index)
                    .unwrap();

                state_variable = Some(variable);
                StartState::Expression(ValueExpression::Variable(variable))
            }
            x => todo!("{:?}", x),
        };

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::StateMachine(StateMachine {
                states,
                transitions,
                start_state,
                state_variable,
            }),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
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
        node,
        enter_events: Vec::new(),
        exit_events: Vec::new(),
    });

    let transition_in_effect = if blend_in.is_zero() {
        TransitionEffect::None
    } else {
        TransitionEffect::Blend(blend_in)
    };
    let transition_out_effect = if blend_out.is_zero() {
        TransitionEffect::None
    } else {
        TransitionEffect::Blend(blend_out)
    };

    // Redirect the original transition to our temporary transition
    // state.
    let original_transition = &mut transitions[original_transition];
    let to = original_transition.to;
    original_transition.to = transition_state;
    original_transition.effect = transition_in_effect;

    transitions.push(Transition {
        from: Some(transition_state),
        to,
        event_id: exit_event as i64,
        priority: 0,
        effect: transition_out_effect,
    });
}

fn find_leaf_nodes(nodes: &Slab<Node>, root: NodeId) -> Vec<NodeId> {
    let mut queue = vec![root];
    let mut leaves = Vec::new();

    while let Some(id) = queue.pop() {
        let is_leaf = match &nodes[id].inner {
            NodeInner::StateMachine(node) => {
                for state in &node.states {
                    queue.push(state.node);
                }

                false
            }
            NodeInner::List(list) => {
                queue.extend(list);

                false
            }
            NodeInner::Blend(node) => {
                for c in &node.children {
                    queue.push(c.node);
                }

                false
            }
            NodeInner::Clip(_) => true,
            NodeInner::VariableChangeTracker(_) => true,
            NodeInner::Timer(_) => true,
            NodeInner::Nop => true,
        };

        if is_leaf {
            leaves.push(id);
        }
    }

    leaves
}
