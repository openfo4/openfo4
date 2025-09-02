//! hkbLayerGenerator
//!
//! Layer is a blendable generator with multiple layers that can be activated/deactivated on demand
//! with events.

use openfo4_hkx::behavior::LayerGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::nodes::layers::{Layer, Layers};
use crate::nodes::{Node, NodeInner};

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

            // if let Some(set) = &layer.parent.variable_binding_set {
            //     for binding in &*set.bindings {
            //         match &*binding.member_path {
            //             name => {
            //                 return Err(Error::UnknownVariableName(UnknownVariableName {
            //                     class: "hkbLayer",
            //                     name: name.to_owned(),
            //                 }));
            //             }
            //         }
            //     }
            // }

            layers.push(Layer {
                on_by_default: layer.on_by_default,
                node,
                on_event: layer.on_event_id as i64,
                off_event: layer.off_event_id as i64,
                is_active: false,
            });
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::Layers(Layers { layers }),
        }))
    }
}
