//! BSiStateTaggingGenerator

use openfo4_hkx::behavior::BSiStateTaggingGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error};
use crate::nodes::list::List;
use crate::nodes::{Node, NodeInner};

impl BuildNode for BSiStateTaggingGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        if let Some(generator) = &self.p_default_generator {
            Ok(vec![Dependency::Node(generator.ptr)])
        } else {
            Ok(Vec::new())
        }
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut nodes = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            if let Some(node) = ctx.get_node(generator.ptr) {
                nodes.push(node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(List { nodes }),
        }))
    }
}
