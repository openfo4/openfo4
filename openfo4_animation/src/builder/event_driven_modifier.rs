//! hkbEventDrivenModifier
//!

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::EventDrivenModifier;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Command, Listener, ListenerEvent, Node};

impl BuildNode for EventDrivenModifier {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(vec![self.parent.modifer.ptr])
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        if let Some(set) = &self.parent.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &*binding.member_path {
                    "userData" => continue,
                    "name" => continue,
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbEventDrivenModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        // let mut nodes = Vec::new();

        // if let Some(event) = state.get_event_id(self.activate_event_id) {
        //     nodes.push(Node::Listener(Listener {
        //         event: ListenerEvent::Event(event),
        //         // reaction: todo!(),

        //     }));
        // }

        Ok(None)
    }
}
