//! BSBehaviorGraphSwapGenerator

use openfo4_hkx::behavior::BSBehaviorGraphSwapGenerator;

use crate::{Node, NodeInner};

use super::{BuildNode, BuilderContext, Dependency, Error};

impl BuildNode for BSBehaviorGraphSwapGenerator {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(self
            .p_default_generator
            .iter()
            .map(|v| Dependency::Node(v.ptr))
            .collect())
    }

    fn build(&self, state: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut nodes = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            if let Some(node) = state.get_node(generator.ptr) {
                nodes.push(node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(nodes),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        }))
    }
}
