//! hkbEventDrivenModifier
//!

use openfo4_hkx::behavior::EventDrivenModifier;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::nodes::event_driven::EventDriven;
use crate::nodes::{Node, NodeInner};

impl BuildNode for EventDrivenModifier {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(vec![Dependency::Node(self.parent.modifer.ptr)])
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        // let mut enable = ValueExpression::Const(Value::Bool(self.parent.parent.enable));

        if let Some(set) = &self.parent.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => {
                        // enable = ValueExpression::Variable(variable);
                    }
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbEventDrivenModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let Some(node) = ctx.get_node(self.parent.modifer.ptr) else {
            return Ok(None);
        };

        let on_event = ctx.get_event(self.activate_event_id);
        let off_event = ctx.get_event(self.deactivate_event_id);

        Ok(Some(Node {
            name: self.parent.parent.node.name.clone(),
            inner: NodeInner::EventDriven(EventDriven {
                node,
                on_by_default: self.active_by_default,
                on_event,
                off_event,
                is_active: false,
            }),
        }))
    }
}
