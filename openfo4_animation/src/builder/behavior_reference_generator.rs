//! hkbBehaviorReferenceGenerator

use openfo4_hkx::behavior::BehaviorReferenceGenerator;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error};
use crate::{Node, NodeInner};

impl BuildNode for BehaviorReferenceGenerator {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        if !self.behavior_name.is_empty() {
            Ok(vec![Dependency::File(self.behavior_name.clone())])
        } else {
            Ok(Vec::new())
        }
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut children = Vec::new();

        if !self.behavior_name.is_empty() {
            if let Some(node) = ctx.get_file_root_node(&self.behavior_name) {
                children.push(node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(children),
            ..Default::default()
        }))
    }
}
