//! hkbEventsFromRangeModifier
//!
//! This modifier sends an event when a variable enters a value between lower bound and upper bound.

use openfo4_hkx::behavior::{EventRangeMode, EventsFromRangeModifier};

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event};
use crate::{
    Command, Listener, ListenerEvent, Node, NodeInner, SetVariable, Value, ValueExpression,
};

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

        let mut listeners = Vec::new();
        let mut on_update = Vec::new();

        for range in &*self.event_ranges.event_data {
            let upper_bound = ValueExpression::Const(Value::F32(range.upper_bound));

            let event = parse_event(ctx, &range.event);

            // Evalutes to true if `input_value >= lower_bound && input_value <= upper_bound`.
            let condition = ValueExpression::And(
                Box::new(ValueExpression::CmpGe(
                    Box::new(input_value.clone()),
                    Box::new(lower_bound.clone()),
                )),
                Box::new(ValueExpression::CmpLe(
                    Box::new(input_value.clone()),
                    Box::new(upper_bound.clone()),
                )),
            );

            match range.event_mode {
                EventRangeMode::OnEnterRange => {
                    // Add a variable that indicates whether the condition is currently
                    // active.
                    // Only when this variable changes from `false` to `true` will we
                    // send an event and then set the variable to `true` until the
                    // condition evaluates to `false` for one frame.
                    let active = ctx.create_variable(Value::Bool(false));

                    // Expand the condition such that it only triggers if `active` is `false`.
                    let on_enter_condition = ValueExpression::And(
                        Box::new(condition.clone()),
                        Box::new(ValueExpression::CmpEq(
                            Box::new(ValueExpression::Variable(active)),
                            Box::new(ValueExpression::Const(Value::Bool(false))),
                        )),
                    );

                    let on_leave_condition = ValueExpression::CmpEq(
                        Box::new(condition),
                        Box::new(ValueExpression::Const(Value::Bool(false))),
                    );

                    let commands = vec![
                        // Equivalent to:
                        // if enable && !active && condition {
                        //    send_event(..);
                        //    active = true;
                        // }
                        Command::Conditional(
                            ValueExpression::And(
                                Box::new(enable.clone()),
                                Box::new(on_enter_condition),
                            ),
                            vec![
                                Command::SendEvent(event),
                                Command::SetVariable(SetVariable {
                                    variable: active,
                                    condition: ValueExpression::Const(Value::Bool(true)),
                                    value: ValueExpression::Const(Value::Bool(true)),
                                }),
                            ],
                        ),
                        // Equivalent to:
                        // if (active && !condition) || !enable {
                        //     active = false;
                        // }
                        // Note that we always want to reset the state if the enable value
                        // changes to false.
                        Command::Conditional(
                            ValueExpression::Or(
                                Box::new(on_leave_condition),
                                Box::new(ValueExpression::Not(Box::new(enable.clone()))),
                            ),
                            vec![Command::SetVariable(SetVariable {
                                variable: active,
                                condition: ValueExpression::Const(Value::Bool(true)),
                                value: ValueExpression::Const(Value::Bool(false)),
                            })],
                        ),
                    ];

                    if let ValueExpression::Variable(variable) = input_value {
                        listeners.push(Listener {
                            event: ListenerEvent::Variable(variable),
                            reaction: commands.clone(),
                        });
                    }

                    if let ValueExpression::Variable(variable) = lower_bound {
                        listeners.push(Listener {
                            event: ListenerEvent::Variable(variable),
                            reaction: commands.clone(),
                        });
                    }

                    if let ValueExpression::Variable(variable) = enable {
                        listeners.push(Listener {
                            event: ListenerEvent::Variable(variable),
                            reaction: commands,
                        });
                    }
                }
                EventRangeMode::WhenInRange => {
                    on_update.push(Command::Conditional(
                        ValueExpression::And(Box::new(enable.clone()), Box::new(condition)),
                        vec![Command::SendEvent(event)],
                    ));
                }
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Nop,
            on_update,
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            listeners,
        }))
    }
}
