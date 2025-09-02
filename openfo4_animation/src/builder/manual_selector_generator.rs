//! hkbManualSelectorGenerator

use std::collections::HashMap;

use openfo4_hkx::behavior::ManualSelectorGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::nodes::manual_selector::ManualSelector;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for ManualSelectorGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(self
            .generators
            .iter()
            .map(|v| Dependency::Node(v.ptr))
            .collect())
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut start_state = ValueExpression::Const(Value::I16(self.selected_generator_index));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "selectedGeneratorIndex" => start_state = ValueExpression::Variable(variable),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbManualSelectorGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut states = HashMap::new();
        for (index, generator) in self.generators.iter().enumerate() {
            let Some(node) = ctx.get_node(generator.ptr) else {
                continue;
            };

            states.insert(index, node);
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::ManualSelector(ManualSelector {
                states,
                index_value: start_state,
                can_change_after_activation: self.selected_index_can_change_after_active,
                active_index: None,
            }),
        }))
    }
}
