//! hkbLayerGenerator
//!
//! Layer is a blendable generator with multiple layers that can be activated/deactivated on demand
//! with events.

use openfo4_hkx::behavior::LayerGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::nodes::layers::{Layer, Layers};
use crate::nodes::{Node, NodeInner};
use crate::{Value, ValueExpression};

impl BuildNode for LayerGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(self
            .layers
            .iter()
            .map(|l| Dependency::Node(l.generator.ptr))
            .collect())
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &*binding.member_path {
                    "userData" => continue,
                    "name" => continue,
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbLayerGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut layers = Vec::new();
        for layer in &*self.layers {
            let Some(node) = ctx.get_node(layer.generator.ptr) else {
                continue;
            };

            let mut weight = ValueExpression::Const(Value::F32(layer.weight));
            let mut bone_weights = layer.bone_weights.as_ref().map(|weights| {
                ValueExpression::Const(Value::BoneWeights(
                    weights.bone_weights.iter().copied().collect(),
                ))
            });
            let mut on_by_default = ValueExpression::Const(Value::Bool(layer.on_by_default));

            if let Some(set) = &layer.parent.variable_binding_set {
                for binding in &*set.bindings {
                    let variable = ctx
                        .get_variable(binding.binding_type, binding.variable_index)
                        .unwrap();

                    match &*binding.member_path {
                        "weight" => weight = ValueExpression::Variable(variable),
                        "boneWeights" => bone_weights = Some(ValueExpression::Variable(variable)),
                        "onByDefault" => on_by_default = ValueExpression::Variable(variable),
                        name => {
                            return Err(Error::UnknownVariableName(UnknownVariableName {
                                class: "hkbLayer",
                                name: name.to_owned(),
                            }));
                        }
                    }
                }
            }

            let on_event = ctx.get_event(layer.on_event_id);
            let off_event = ctx.get_event(layer.off_event_id);

            layers.push(Layer {
                on_by_default,
                node,
                on_event,
                off_event,
                is_active: false,
                weight,
                bone_weights,
            });
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Layers(Layers { layers }),
        }))
    }
}
