use openfo4_hkx::behavior::BSCyclicBlendTransitionGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::{Blend, BlendNode, Node, NodeInner, Value, ValueExpression};

impl BuildNode for BSCyclicBlendTransitionGenerator {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        if let Some(generator) = &self.p_blender_generator {
            generator.dependencies(ctx)
        } else {
            Ok(Vec::new())
        }
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let Some(blender_generator) = &self.p_blender_generator else {
            return Ok(None);
        };

        let mut blend_parameter = ValueExpression::Const(Value::F32(self.f_blend_parameter));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "fBlendParameter" => blend_parameter = ValueExpression::Variable(variable),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSCyclicBlendTransitionGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let freeze_event = ctx
            .get_event(self.event_to_freeze_blend_value.base.id)
            .unwrap();
        let cross_blend_event = ctx.get_event(self.event_to_cross_blend.base.id).unwrap();

        if let Some(set) = &blender_generator.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &*binding.member_path {
                    // blendParamter comes from the BSCyclicBlendTransitionGenerator fBlendParameter value.
                    // The value on the hkbBlenderGenerator is ignored.
                    "blendParameter" => (),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbBlenderGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut children = Vec::new();
        for child in &*blender_generator.children {
            let Some(generator) = &child.generator else {
                continue;
            };

            let mut weight = ValueExpression::Const(Value::F32(child.weight));
            let mut bone_weights = None;

            if let Some(weights) = &child.bone_weights {
                if let Some(set) = &weights.bindable.variable_binding_set {
                    for binding in &*set.bindings {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbBoneWeightArray",
                            name: binding.member_path.clone(),
                        }));
                    }
                }

                bone_weights = Some(ValueExpression::Const(Value::BoneWeights(
                    weights.bone_weights.iter().copied().collect(),
                )))
            }

            if let Some(set) = &child.parent.variable_binding_set {
                for binding in &*set.bindings {
                    let variable = ctx
                        .get_variable(binding.binding_type, binding.variable_index)
                        .unwrap();

                    match &*binding.member_path {
                        "weight" => weight = ValueExpression::Variable(variable),
                        "boneWeights" => bone_weights = Some(ValueExpression::Variable(variable)),
                        name => {
                            return Err(Error::UnknownVariableName(UnknownVariableName {
                                class: "hkbBlenderGeneratorChild",
                                name: name.to_owned(),
                            }));
                        }
                    }
                }
            }

            if let Some(id) = ctx.get_node(generator.ptr) {
                children.push(BlendNode {
                    weight,
                    bone_weights,
                    node: id,
                });
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Blend(Blend {
                blend_factor: blend_parameter,
                children,
                is_parametric: true,
            }),
            ..Default::default()
        }))
    }
}
