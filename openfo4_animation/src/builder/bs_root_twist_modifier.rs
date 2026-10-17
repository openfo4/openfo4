use std::f32::consts::PI;

use glam::Vec3;
use openfo4_hkx::behavior::{BSRootTwistModifier, TwistType};

use crate::builder::{BuildNode, Error, UnknownVariableName};
use crate::nodes::twist::Twist;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for BSRootTwistModifier {
    fn build(&self, ctx: &mut super::BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));
        let mut angle = ValueExpression::Const(Value::F32(self.root_twist_angle));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => enable = ValueExpression::Variable(variable),
                    "rootTwistAngle" => angle = ValueExpression::Variable(variable),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSRootTwistModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let axis = Vec3::Z;

        let angle = match self.twist_type {
            // Convert angle from degrees to radians.
            TwistType::AngleInDegrees => ValueExpression::Mul(
                Box::new(angle),
                Box::new(ValueExpression::Const(Value::F32(PI / 180.0))),
            ),
            // Unit circle (0.25 = 90°, 0.5 = 180°, 0.75 = 360°)
            TwistType::Direction => ValueExpression::Mul(
                Box::new(angle),
                Box::new(ValueExpression::Const(Value::F32(PI / 0.5))),
            ),
        };

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Twist(Twist {
                enable,
                axis,
                angle,
                // FIXME: Don't assume that index 0 is the root bone.
                start_bone: 0,
                end_bone: 0,
            }),
        }))
    }
}
