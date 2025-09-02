use openfo4_hkx::behavior::ModifierGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error};
use crate::nodes::list::List;
use crate::nodes::{Node, NodeInner};

impl BuildNode for ModifierGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(vec![
            Dependency::Node(self.modifier.ptr),
            Dependency::Node(self.generator.ptr),
        ])
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut nodes = Vec::new();

        if let Some(node) = ctx.get_node(self.modifier.ptr) {
            nodes.push(node);
        }

        if let Some(node) = ctx.get_node(self.generator.ptr) {
            nodes.push(node);
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(List { nodes }),
        }))
    }
}
