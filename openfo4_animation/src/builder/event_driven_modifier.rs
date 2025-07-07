//! hkbEventDrivenModifier
//!

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::EventDrivenModifier;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Command, Listener, ListenerEvent, Node, NodeInner, Value, ValueExpression};

impl BuildNode for EventDrivenModifier {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(vec![self.parent.modifer.ptr])
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut enable = ValueExpression::Const(Value::Bool(self.parent.parent.enable));

        if let Some(set) = &self.parent.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = state
                    .get_variable_id(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => {
                        enable = ValueExpression::Variable(variable);
                    }
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbEventDrivenModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let Some(modifier) = state.ptr_to_id.get(&self.parent.modifer.ptr).copied() else {
            return Ok(None);
        };

        let mut listeners = Vec::new();

        if let Some(event) = state.get_event_id(self.activate_event_id) {
            // Only activate the modifier if the event was received
            // while the enable variable was set to true.
            listeners.push(Listener {
                event: ListenerEvent::Event(event),
                reaction: vec![Command::Conditional(
                    enable.clone(),
                    vec![Command::ActivateNode(modifier)],
                )],
            });
        }

        if let Some(event) = state.get_event_id(self.deactivate_event_id) {
            // Note that we don't have to care about the enable variable
            // here since if we receive the event while enable was false
            // the variable tracker also disable the node.
            listeners.push(Listener {
                event: ListenerEvent::Event(event),
                reaction: vec![Command::DeactivateNode(modifier)],
            });
        }

        match enable {
            ValueExpression::Const(Value::Bool(true)) => (),
            ValueExpression::Const(Value::Bool(false)) => return Ok(None),
            ValueExpression::Variable(variable) => {
                listeners.push(Listener {
                    event: ListenerEvent::Variable(variable),
                    reaction: vec![Command::Conditional(
                        ValueExpression::CmpEq(
                            Box::new(enable),
                            Box::new(ValueExpression::Const(Value::Bool(false))),
                        ),
                        vec![Command::DeactivateNode(modifier)],
                    )],
                });
            }
            _ => unreachable!(),
        }

        Ok(Some(Node {
            name: self.parent.parent.node.name.clone(),
            inner: NodeInner::Nop,
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
            listeners,
        }))
    }
}
