use std::time::Duration;

use openfo4_hkx::behavior::BSRandomAlarmModifier;

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event};
use crate::nodes::timer::Timer;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for BSRandomAlarmModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let Some(event) = parse_event(ctx, &self.alarm_event) else {
            return Ok(None);
        };

        let repeat = self.reset_alarm;
        let mut condition = ValueExpression::Const(Value::Bool(self.parent.enable));
        let mut min_time = ValueExpression::Const(Value::F32(self.alarm_time_seconds_min));
        let mut max_time = ValueExpression::Const(Value::F32(self.alarm_time_seconds_max));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => condition = ValueExpression::Variable(variable),
                    "alarmTimeSecondsMin" => min_time = ValueExpression::Variable(variable),
                    "alarmTimeSecondsMax" => max_time = ValueExpression::Variable(variable),
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
            name: self.parent.node.name.clone(),
            inner: NodeInner::Timer(Timer {
                condition,
                min_time,
                max_time,
                event,
                repeat,
                eval_duration: Duration::ZERO,
                elapsed: Duration::ZERO,
                is_running: false,
            }),
        }))
    }
}
