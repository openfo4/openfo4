//! hkbTimerModifier

use std::time::Duration;

use openfo4_hkx::behavior::TimerModifier;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Node, NodeInner, Timer, parse_event};

impl BuildNode for TimerModifier {
    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let event = parse_event(state.file, &self.alarm_event);
        let time = Duration::from_secs_f32(self.alarm_time_seconds);

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &binding.member_path {
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbTimerModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Timer(Timer {
                min_time: time,
                max_time: time,
                event,
                repeat: false,
            }),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
            listeners: Vec::new(),
        }))
    }
}
