use openfo4_hkx::behavior::BSOffsetAnimationGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error};
use crate::nodes::list::List;
use crate::nodes::{Node, NodeInner};

impl BuildNode for BSOffsetAnimationGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        let mut children = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            children.push(Dependency::Node(generator.ptr));
        }

        if let Some(generator) = &self.p_offset_clip_generator {
            children.push(Dependency::Node(generator.ptr));
        }

        Ok(children)
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
