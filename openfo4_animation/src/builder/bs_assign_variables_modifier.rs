//! BSAssignVariablesModifier

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName};
use crate::nodes::assign_variables::{AssignVariable, AssignVariables};
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for openfo4_hkx::behavior::BSAssignVariablesModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let Some(set) = &self.parent.node.bindable.variable_binding_set else {
            return Ok(None);
        };

        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));
        let mut float_variables = [None; 20];
        let mut float_values = [None; 20];
        let mut int_variables = [None; 4];
        let mut int_values = [None; 4];

        for binding in &*set.bindings {
            let variable = ctx
                .get_variable(binding.binding_type, binding.variable_index)
                .unwrap();

            let slot = match binding.member_path.as_str() {
                "enable" => {
                    enable = ValueExpression::Variable(variable);
                    continue;
                }
                "floatVariable1" => &mut float_variables[0],
                "floatVariable2" => &mut float_variables[1],
                "floatVariable3" => &mut float_variables[2],
                "floatVariable4" => &mut float_variables[3],
                "floatVariable5" => &mut float_variables[4],
                "floatVariable6" => &mut float_variables[5],
                "floatVariable7" => &mut float_variables[6],
                "floatVariable8" => &mut float_variables[7],
                "floatVariable9" => &mut float_variables[8],
                "floatVariable10" => &mut float_variables[9],
                "floatVariable11" => &mut float_variables[10],
                "floatVariable12" => &mut float_variables[11],
                "floatVariable13" => &mut float_variables[12],
                "floatVariable14" => &mut float_variables[13],
                "floatVariable15" => &mut float_variables[14],
                "floatVariable16" => &mut float_variables[15],
                "floatVariable17" => &mut float_variables[16],
                "floatVariable18" => &mut float_variables[17],
                "floatVariable19" => &mut float_variables[18],
                "floatVariable20" => &mut float_variables[19],
                "floatValue1" => &mut float_values[0],
                "floatValue2" => &mut float_values[1],
                "floatValue3" => &mut float_values[2],
                "floatValue4" => &mut float_values[3],
                "floatValue5" => &mut float_values[4],
                "floatValue6" => &mut float_values[5],
                "floatValue7" => &mut float_values[6],
                "floatValue8" => &mut float_values[7],
                "floatValue9" => &mut float_values[8],
                "floatValue10" => &mut float_values[9],
                "floatValue11" => &mut float_values[10],
                "floatValue12" => &mut float_values[11],
                "floatValue13" => &mut float_values[12],
                "floatValue14" => &mut float_values[13],
                "floatValue15" => &mut float_values[14],
                "floatValue16" => &mut float_values[15],
                "floatValue17" => &mut float_values[16],
                "floatValue18" => &mut float_values[17],
                "floatValue19" => &mut float_values[18],
                "floatValue20" => &mut float_values[19],
                "intVariable1" => &mut int_variables[0],
                "intVariable2" => &mut int_variables[1],
                "intVariable3" => &mut int_variables[2],
                "intVariable4" => &mut int_variables[3],
                "intValue1" => &mut int_values[0],
                "intValue2" => &mut int_values[1],
                "intValue3" => &mut int_values[2],
                "intValue4" => &mut int_values[3],
                name => {
                    return Err(Error::UnknownVariableName(UnknownVariableName {
                        class: "BSAssignVariablesModifier",
                        name: name.to_owned(),
                    }));
                }
            };

            *slot = Some(variable);
        }

        let mut variables = Vec::new();
        for (index, (var, val)) in float_variables.iter().zip(float_values).enumerate() {
            // If no variable binding is set the write is equivalent
            // to a no-op, so we can skip it.
            let Some(variable) = *var else {
                continue;
            };

            let value = match val {
                Some(val) => ValueExpression::Variable(val),
                None => match index {
                    0 => ValueExpression::Const(Value::F32(self.float_value_1)),
                    1 => ValueExpression::Const(Value::F32(self.float_value_2)),
                    2 => ValueExpression::Const(Value::F32(self.float_value_3)),
                    3 => ValueExpression::Const(Value::F32(self.float_value_4)),
                    4 => ValueExpression::Const(Value::F32(self.float_value_5)),
                    5 => ValueExpression::Const(Value::F32(self.float_value_6)),
                    6 => ValueExpression::Const(Value::F32(self.float_value_7)),
                    7 => ValueExpression::Const(Value::F32(self.float_value_8)),
                    8 => ValueExpression::Const(Value::F32(self.float_value_9)),
                    9 => ValueExpression::Const(Value::F32(self.float_value_10)),
                    10 => ValueExpression::Const(Value::F32(self.float_value_11)),
                    11 => ValueExpression::Const(Value::F32(self.float_value_12)),
                    12 => ValueExpression::Const(Value::F32(self.float_value_13)),
                    13 => ValueExpression::Const(Value::F32(self.float_value_14)),
                    14 => ValueExpression::Const(Value::F32(self.float_value_15)),
                    15 => ValueExpression::Const(Value::F32(self.float_value_16)),
                    16 => ValueExpression::Const(Value::F32(self.float_value_17)),
                    17 => ValueExpression::Const(Value::F32(self.float_value_18)),
                    18 => ValueExpression::Const(Value::F32(self.float_value_19)),
                    19 => ValueExpression::Const(Value::F32(self.float_value_20)),
                    _ => unreachable!(),
                },
            };

            variables.push(AssignVariable { variable, value });
        }

        for (index, (var, val)) in int_variables.iter().zip(int_values).enumerate() {
            // If no variable binding is set the write is equivalent
            // to a no-op, so we can skip it.
            let Some(variable) = *var else {
                continue;
            };

            let value = match val {
                Some(val) => ValueExpression::Variable(val),
                None => match index {
                    0 => ValueExpression::Const(Value::I32(self.int_value_1)),
                    1 => ValueExpression::Const(Value::I32(self.int_value_2)),
                    2 => ValueExpression::Const(Value::I32(self.int_value_3)),
                    3 => ValueExpression::Const(Value::I32(self.int_value_4)),
                    _ => unreachable!(),
                },
            };

            variables.push(AssignVariable { variable, value });
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::AssignVariables(AssignVariables {
                condition: enable,
                variables,
            }),
        }))
    }
}
