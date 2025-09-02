//! BSTimerModifier

use std::time::Duration;

use openfo4_hkx::behavior::BSTimerModifier;

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event};
use crate::nodes::timer::Timer;
use crate::nodes::{Node, NodeInner};

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
            name: self.parent.node.name.clone(),
            inner: NodeInner::Timer(Timer {
                min_time: time,
                max_time: time,
                event,
                repeat,
                elapsed: Duration::ZERO,
                eval_duration: Duration::ZERO,
                is_running: false,
            }),
        }))
    }
}
