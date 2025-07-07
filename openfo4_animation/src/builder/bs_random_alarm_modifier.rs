use std::time::Duration;

use openfo4_hkx::behavior::BSRandomAlarmModifier;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Node, NodeInner, Timer, parse_event};

impl BuildNode for BSRandomAlarmModifier {
    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let min_time = Duration::from_secs_f32(self.alarm_time_seconds_min);
        let max_time = Duration::from_secs_f32(self.alarm_time_seconds_max);
        let event = parse_event(state.file, &self.alarm_event);
        let repeat = self.reset_alarm;

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &binding.member_path {
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSRandomAlarmModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        Ok(Some(Node {
            name: format!("[BSRandomAlarmModifier] {}", self.parent.node.name.clone()),
            inner: NodeInner::Timer(Timer {
                min_time,
                max_time,
                event,
                repeat,
            }),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
            listeners: Vec::new(),
        }))
    }
}
