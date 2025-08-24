//! BSTimerModifier

use std::time::Duration;

use openfo4_hkx::behavior::BSTimerModifier;

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event};
use crate::{Node, NodeInner, Timer};

impl BuildNode for BSTimerModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let time = Duration::from_secs_f32(self.alarm_time_seconds);
        let repeat = self.reset_alarm;

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &binding.member_path {
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSTimerModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let event = parse_event(ctx, &self.alarm_event);

        Ok(Some(Node {
            name: format!("[BSTimerModifier] {}", self.parent.node.name),
            inner: NodeInner::Timer(Timer {
                min_time: time,
                max_time: time,
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
