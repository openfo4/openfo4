//! hkbTimerModifier

use std::time::Duration;

use openfo4_hkx::behavior::TimerModifier;

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event};
use crate::nodes::timer::Timer;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for TimerModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let event = parse_event(ctx, &self.alarm_event);
        let mut condition = ValueExpression::Const(Value::Bool(self.parent.enable));
        let mut time = ValueExpression::Const(Value::F32(self.alarm_time_seconds));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => condition = ValueExpression::Variable(variable),
                    "alarmTimeSeconds" => time = ValueExpression::Variable(variable),
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
                condition,
                min_time: time.clone(),
                max_time: time,
                event,
                repeat: false,
                eval_duration: Duration::ZERO,
                elapsed: Duration::ZERO,
                is_running: false,
            }),
        }))
    }
}
