use openfo4_hkx::behavior::TwistModifier;

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName};
use crate::nodes::twist::Twist;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for TwistModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let axis = self.axis_of_rotation.truncate();
        let mut angle = ValueExpression::Const(Value::F32(self.twist_angle));
        let start_bone = self.start_bone_index;
        let end_bone = self.end_bone_index;

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "twistAngle" => angle = ValueExpression::Variable(variable),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbTwistModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Twist(Twist {
                axis,
                angle,
                start_bone: start_bone as usize,
                end_bone: end_bone as usize,
            }),
        }))
    }
}
