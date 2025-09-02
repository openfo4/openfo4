//! hkxBlenderGenerator

use openfo4_hkx::behavior::{BlenderGenerator, BlenderGeneratorFlags};

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::nodes::blend::{Blend, BlendChildren};
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for BlenderGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(self
            .children
            .iter()
            .filter_map(|c| c.generator.as_ref().map(|g| Dependency::Node(g.ptr)))
            .collect())
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut children = Vec::new();

        let mut blend_parameter = ValueExpression::Const(Value::F32(self.blend_parameter));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "blendParameter" => blend_parameter = ValueExpression::Variable(variable),
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbBlenderGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        for child in &*self.children {
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
                    weights.bone_weights.iter().map(|v| *v).collect(),
                )));
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

            let node = ctx.get_node(generator.ptr);
            children.push(BlendChildren {
                weight,
                bone_weights,
                node,
            });
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Blend(Blend {
                blend_factor: blend_parameter,
                children,
                is_parametric: self.flags.contains(BlenderGeneratorFlags::PARAMETRIC_BLEND),
            }),
        }))
    }
}
