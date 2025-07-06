//! hkbLayerGenerator
//!
//! Layer is a blendable generator with multiple layers that can be activated/deactivated on demand
//! with events.

use std::time::Duration;

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::LayerGenerator;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Node, NodeInner, StartState, State, StateMachine, Transition, TransitionEffect};

impl BuildNode for LayerGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(self.layers.iter().map(|l| l.generator.ptr).collect())
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut layers = Vec::new();

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &*binding.member_path {
                    "userData" => continue,
                    "name" => continue,
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbLayerGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        // We compile every layer into an individual state machine
        // with the two states representing on and off.
        for layer in &*self.layers {
            let Some(node) = state.ptr_to_id.get(&layer.generator.ptr) else {
                continue;
            };

            // if let Some(set) = &layer.parent.variable_binding_set {
            //     for binding in &*set.bindings {
            //         match &*binding.member_path {
            //             name => {
            //                 return Err(Error::UnknownVariableName(UnknownVariableName {
            //                     class: "hkbLayer",
            //                     name: name.to_owned(),
            //                 }));
            //             }
            //         }
            //     }
            // }

            let on_node = *node;
            let off_node = state.nodes.insert(Node {
                name: "".to_owned(),
                inner: NodeInner::List(Vec::new()),
            });

            let states = vec![
                State {
                    node: off_node,
                    enter_events: Vec::new(),
                    exit_events: Vec::new(),
                },
                State {
                    node: on_node,
                    enter_events: Vec::new(),
                    exit_events: Vec::new(),
                },
            ];
            let mut transitions = Vec::new();
            let start_state = if layer.on_by_default {
                StartState::Index(1)
            } else {
                StartState::Index(0)
            };

            // Off -> On transition
            if let Some(event) = state.get_event_id(layer.on_event_id) {
                transitions.push(Transition {
                    from: Some(0),
                    to: 1,
                    event_id: event,
                    priority: 0,
                    effect: TransitionEffect::Blend(Duration::from_secs_f32(
                        layer.fade_in_duration,
                    )),
                });
            }

            // On -> Off transition
            if let Some(event) = state.get_event_id(layer.off_event_id) {
                transitions.push(Transition {
                    from: Some(1),
                    to: 0,
                    event_id: event,
                    priority: 0,
                    effect: TransitionEffect::Blend(Duration::from_secs_f32(
                        layer.fade_out_duration,
                    )),
                });
            }

            let state_machine = state.nodes.insert(Node {
                name: "".to_owned(),
                inner: NodeInner::StateMachine(StateMachine {
                    states,
                    transitions,
                    start_state,
                    state_variable: None,
                }),
            });

            layers.push(state_machine);
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(layers),
        }))
    }
}
