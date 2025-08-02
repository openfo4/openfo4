//! BSiStateTaggingGenerator

use openfo4_hkx::behavior::BSiStateTaggingGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error};
use crate::{Node, NodeInner};

impl BuildNode for BSiStateTaggingGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        if let Some(generator) = &self.p_default_generator {
            Ok(vec![Dependency::Node(generator.ptr)])
        } else {
            Ok(Vec::new())
        }
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut list = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            let node = ctx.get_node(generator.ptr).unwrap();
            list.push(node);
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(list),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
            listeners: Vec::new(),
        }))
    }
}
