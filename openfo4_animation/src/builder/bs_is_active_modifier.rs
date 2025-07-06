//! BSIsActiveModifier

use std::collections::HashMap;

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Node, NodeInner, SetVariable, SetVariables, Value, ValueExpression};

impl BuildNode for openfo4_hkx::behavior::BSIsActiveModifier {
    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let Some(set) = &self.parent.node.bindable.variable_binding_set else {
            return Ok(None);
        };

        let mut on_enable = HashMap::new();
        let mut on_disable = HashMap::new();

        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));

        for binding in &*set.bindings {
            let variable = state
                .get_variable_id(binding.binding_type, binding.variable_index)
                .unwrap();

            let invert = match binding.member_path.as_str() {
                "userData" => continue,
                "name" => continue,
                "enable" => {
                    enable = ValueExpression::Variable(variable);
                    continue;
                }
                "bIsActive0" | "bIsActive1" | "bIsActive2" | "bIsActive3" | "bIsActive4" => false,
                "bInvertActive0" | "bInvertActive1" | "bInvertActive2" | "bInvertActive3"
                | "bInvertActive4" => true,
                name => {
                    return Err(Error::UnknownVariableName(UnknownVariableName {
                        class: "BSIsActiveModifier",
                        name: name.to_owned(),
                    }));
                }
            };

            let enable_value = Value::Bool(!invert);
            let disable_value = Value::Bool(invert);

            on_enable.insert(variable, enable_value);
            on_disable.insert(variable, disable_value);
        }

        let on_activate = on_enable
            .into_iter()
            .map(|(variable, value)| SetVariable {
                variable,
                condition: enable.clone(),
                value: ValueExpression::Const(value),
            })
            .collect();

        let on_deactivate = on_disable
            .into_iter()
            .map(|(variable, value)| SetVariable {
                variable,
                condition: enable.clone(),
                value: ValueExpression::Const(value),
            })
            .collect();

        Ok(Some(Node {
            name: format!("[BSIsActiveModifier] {}", self.parent.node.name),
            inner: NodeInner::SetVariables(SetVariables {
                on_activate,
                on_deactivate,
            }),
        }))
    }
}
