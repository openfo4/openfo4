//! hkbManualSelectorGenerator
//!
//! This is a generator that selects another generator based on a variable.
//! We compile this node into a [`StateMachine`].

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::ManualSelectorGenerator;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{
    Event, Node, NodeInner, StartState, State, StateMachine, Transition, TransitionEffect, Value,
    ValueExpression, VariableChangeEvent, VariableChangeTracker,
};

impl BuildNode for ManualSelectorGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(self.generators.iter().map(|v| v.ptr).collect())
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut start_state = ValueExpression::Const(Value::I16(self.selected_generator_index));

        let mut selected_generator_variable = 0;

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = state
                    .get_variable_id(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "selectedGeneratorIndex" => {
                        start_state = ValueExpression::Variable(variable);
                        selected_generator_variable = variable;
                    }
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbManualSelectorGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        // If the selected_generator_index is a constant value, the generator
        // index will never change and we can just always forward the generator
        // directly.
        if let ValueExpression::Const(index) = start_state {
            // Must be i16, since `selected_generator_index` is i16.
            let index = index.to_usize().unwrap();

            let generator = &self.generators[index];

            let Some(node) = state.ptr_to_id.get(&generator.ptr).copied() else {
                return Ok(None);
            };

            return Ok(Some(Node {
                name: self.parent.node.name.clone(),
                inner: NodeInner::List(vec![node]),
                listeners: Vec::new(),
                on_activate: Vec::new(),
                on_deactivate: Vec::new(),
                on_update: Vec::new(),
            }));
        }

        let mut states = Vec::new();

        for generator in &*self.generators {
            let Some(node) = state.ptr_to_id.get(&generator.ptr).copied() else {
                continue;
            };

            states.push(State {
                node,
                enter_events: Vec::new(),
                exit_events: Vec::new(),
            });
        }

        // If the state can never change after activation we don't
        // need any transitions.
        if !self.selected_index_can_change_after_active {
            return Ok(Some(Node {
                name: self.parent.node.name.clone(),
                inner: NodeInner::StateMachine(StateMachine {
                    states,
                    transitions: Vec::new(),
                    start_state: StartState::Expression(start_state),
                    state_variable: None,
                }),
                listeners: Vec::new(),
                on_activate: Vec::new(),
                on_deactivate: Vec::new(),
                on_update: Vec::new(),
            }));
        }

        let mut transitions = Vec::new();
        let mut trackers = Vec::new();

        // Generator changes after node activation can be implemented with
        // state transitions.
        for to in 0..states.len() {
            let event = state.events.len();
            state.events.push("internal_transition_event".to_owned());

            transitions.push(Transition {
                from: None,
                to,
                event_id: event as i64,
                priority: 0,
                effect: TransitionEffect::None,
            });

            // Create an event when the variable is set to `selected_generator_variable`.
            // This will trigger every frame while the variable is set to the correct
            // value, but this has no further effect.
            trackers.push(VariableChangeEvent {
                expr: ValueExpression::CmpEq(
                    Box::new(ValueExpression::Variable(selected_generator_variable)),
                    Box::new(ValueExpression::Const(Value::I16(to as i16))),
                ),
                event: Event {
                    id: event as i64,
                    payload: Value::None,
                },
            });
        }

        let state_machine = state.nodes.insert(Node {
            name: "".to_owned(),
            inner: NodeInner::StateMachine(StateMachine {
                states,
                transitions,
                start_state: StartState::Expression(start_state),
                state_variable: None,
            }),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        });
        let tracker = state.nodes.insert(Node {
            name: "".to_owned(),
            inner: NodeInner::VariableChangeTracker(VariableChangeTracker { trackers }),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        });

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(vec![state_machine, tracker]),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        }))
    }
}
