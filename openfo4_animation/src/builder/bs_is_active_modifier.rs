//! BSIsActiveModifier

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName};
use crate::nodes::is_active::IsActive;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for openfo4_hkx::behavior::BSIsActiveModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let Some(set) = &self.parent.node.bindable.variable_binding_set else {
            return Ok(None);
        };

        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));

        let mut variables = Vec::new();
        let mut inverted_variables = Vec::new();

        for binding in &*set.bindings {
            let variable = ctx
                .get_variable(binding.binding_type, binding.variable_index)
                .unwrap();

            let invert = match binding.member_path.as_str() {
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

            if invert {
                inverted_variables.push(variable);
            } else {
                variables.push(variable);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::IsActive(IsActive {
                condition: enable,
                variables,
                inverted_variables,
                is_active: false,
            }),
        }))
    }
}
