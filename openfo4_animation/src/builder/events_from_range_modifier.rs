//! hkbEventsFromRangeModifier
//!
//! This modifier sends an event when a variable enters a value between lower bound and upper bound.

use openfo4_hkx::behavior::{EventRangeMode, EventsFromRangeModifier};

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{
    Node, NodeInner, PerFrameCommands, SendEvent, SetVariableStmt, Statement, Value,
    ValueExpression, parse_event,
};

impl BuildNode for EventsFromRangeModifier {
    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut input_value = ValueExpression::Const(Value::F32(self.input_value));
        let mut lower_bound = ValueExpression::Const(Value::F32(self.lower_bound));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = state
                    .get_variable_id(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
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

        let mut stmts = Vec::new();

        for range in &*self.event_ranges.event_data {
            let upper_bound = ValueExpression::Const(Value::F32(range.upper_bound));

            let event = parse_event(state.file, &range.event);

            // Evalutes to true if `input_value <= lower_bound && input_value >= upper_bound`.
            let condition = ValueExpression::And(
                Box::new(ValueExpression::CmpLe(
                    Box::new(input_value.clone()),
                    Box::new(lower_bound.clone()),
                )),
                Box::new(ValueExpression::CmpGe(
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
                    let active = state.create_variable(Value::Bool(false));

                    // Expand the condition such that it only triggers if `active` is `false`.
                    let on_enter_condition = ValueExpression::And(
                        Box::new(condition.clone()),
                        Box::new(ValueExpression::CmpEq(
                            Box::new(ValueExpression::Variable(active)),
                            Box::new(ValueExpression::Const(Value::Bool(false))),
                        )),
                    );

                    stmts.push(Statement::SendEvent(SendEvent {
                        event,
                        condition: on_enter_condition.clone(),
                    }));
                    stmts.push(Statement::SetVariable(SetVariableStmt {
                        variable: active,
                        condition: on_enter_condition,
                        value: ValueExpression::Const(Value::Bool(true)),
                    }));

                    let on_leave_condition = ValueExpression::CmpEq(
                        Box::new(condition),
                        Box::new(ValueExpression::Const(Value::Bool(false))),
                    );

                    stmts.push(Statement::SetVariable(SetVariableStmt {
                        variable: active,
                        condition: on_leave_condition,
                        value: ValueExpression::Const(Value::Bool(false)),
                    }));
                }
                EventRangeMode::WhenInRange => {
                    stmts.push(Statement::SendEvent(SendEvent { event, condition }));
                }
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::PerFrameCommands(PerFrameCommands { stmts }),
        }))
    }
}
