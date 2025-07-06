//! hkxBlenderGenerator

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::{BlenderGenerator, BlenderGeneratorFlags};

use crate::builder::{BuildNode, Error, GraphBuilderState, UnknownVariableName};
use crate::{Blend, BlendNode, Node, NodeInner, Value, ValueExpression};

impl BuildNode for BlenderGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(self
            .children
            .iter()
            .filter_map(|c| c.generator.as_ref().map(|g| g.ptr))
            .collect())
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut children = Vec::new();

        let mut blend_parameter = ValueExpression::Const(Value::F32(self.blend_parameter));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = state
                    .get_variable_id(binding.binding_type, binding.variable_index)
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

            if let Some(set) = &child.parent.variable_binding_set {
                for binding in &*set.bindings {
                    let variable = state
                        .get_variable_id(binding.binding_type, binding.variable_index)
                        .unwrap();

                    match &*binding.member_path {
                        "weight" => weight = ValueExpression::Variable(variable),
                        "boneWeights" => {}
                        name => {
                            return Err(Error::UnknownVariableName(UnknownVariableName {
                                class: "hkbBlenderGeneratorChild",
                                name: name.to_owned(),
                            }));
                        }
                    }
                }
            }

            if let Some(id) = state.ptr_to_id.get(&generator.ptr) {
                children.push(BlendNode { weight, node: *id });
            }
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
