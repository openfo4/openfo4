use openfo4_hkx::behavior::BSDirectAtModifier;

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName};
use crate::nodes::direct_at::DirectAt;
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for BSDirectAtModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));
        let mut direct_at_on = ValueExpression::Const(Value::Bool(self.direct_at_on));
        let mut active = ValueExpression::Const(Value::Bool(self.active));

        let mut start_bone_index = ValueExpression::Const(Value::I16(self.start_bone_index));
        let mut end_bone_index = ValueExpression::Const(Value::I16(self.end_bone_index));
        let mut source_bone_index = ValueExpression::Const(Value::I16(self.source_bone_index));

        let mut camera_x = ValueExpression::Const(Value::F32(self.direct_at_camera_x));
        let mut camera_y = ValueExpression::Const(Value::F32(self.direct_at_camera_y));
        let mut camera_z = ValueExpression::Const(Value::F32(self.direct_at_camera_z));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => enable = ValueExpression::Variable(variable),
                    "directAtOn" => direct_at_on = ValueExpression::Variable(variable),
                    "offsetsCharacterRelative" => {}
                    "limitHeadingDegreesCCW" => {}
                    "limitHeadingDegreesCW" => {}
                    "limitPitchDegreesUp" => {}
                    "limitPitchDegreesDown" => {}
                    "initialHeadingOffset" => {}
                    "initialPitchOffset" => {}
                    "initialGain" => {}
                    "allowSourceBonePoseCapture" => {}
                    "directAtCameraX" => camera_x = ValueExpression::Variable(variable),
                    "directAtCameraY" => camera_y = ValueExpression::Variable(variable),
                    "directAtCameraZ" => camera_z = ValueExpression::Variable(variable),
                    "active" => active = ValueExpression::Variable(variable),
                    "currentHeadingOffset" => {}
                    "currentPitchOffset" => {}
                    "currentGain" => {}
                    "sourceBoneIndex" => source_bone_index = ValueExpression::Variable(variable),
                    "startBoneIndex" => start_bone_index = ValueExpression::Variable(variable),
                    "endBoneIndex" => end_bone_index = ValueExpression::Variable(variable),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSDirectAtModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::DirectAt(DirectAt {
                enable: ValueExpression::And(
                    Box::new(ValueExpression::And(
                        Box::new(enable),
                        Box::new(direct_at_on),
                    )),
                    Box::new(active),
                ),
                start_bone_index,
                end_bone_index,
                source_bone_index,
                camera_x,
                camera_y,
                camera_z,
            }),
        }))
    }
}
