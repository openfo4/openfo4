//! hkbEventsFromRangeModifier
//!
//! This modifier sends an event when a variable enters a value between lower bound and upper bound.

use openfo4_hkx::behavior::{EventRangeMode, EventsFromRangeModifier};

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event};
use crate::nodes::events_from_range::{EventsFromRange, Range};
use crate::{Node, NodeInner, Value, ValueExpression};

impl BuildNode for EventsFromRangeModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));
        let mut input_value = ValueExpression::Const(Value::F32(self.input_value));
        let mut lower_bound = ValueExpression::Const(Value::F32(self.lower_bound));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => {
                        enable = ValueExpression::Variable(variable);
                    }
                    "inputValue" => {
                        input_value = ValueExpression::Variable(variable);
                    }
                    "lowerBound" => {
                        lower_bound = ValueExpression::Variable(variable);
                    }
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbEventsFromRangeModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut ranges = Vec::new();
        for range in &*self.event_ranges.event_data {
            let upper_bound = ValueExpression::Const(Value::F32(range.upper_bound));

            let Some(event) = parse_event(ctx, &range.event) else {
                continue;
            };

            let continous = match range.event_mode {
                EventRangeMode::OnEnterRange => true,
                EventRangeMode::WhenInRange => false,
            };

            ranges.push(Range {
                lower: lower_bound.clone(),
                upper: upper_bound,
                continous,
                event,
                is_active: false,
            });
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::EventsFromRange(EventsFromRange {
                condition: enable,
                input: input_value,
                ranges,
            }),
        }))
    }
}
