use openfo4_hkx::behavior::ModifierList;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error};
use crate::nodes::list::List;
use crate::nodes::{Node, NodeInner};

impl BuildNode for ModifierList {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        Ok(self
            .modifiers
            .iter()
            .map(|m| Dependency::Node(m.ptr))
            .collect())
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut nodes = Vec::new();

        for modifier in &*self.modifiers {
            if let Some(node) = ctx.get_node(modifier.ptr) {
                nodes.push(node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(List { nodes }),
        }))
    }
}
