use openfo4_hkx::behavior::BSModifyOnceModifier;

use crate::Node;
use crate::builder::{BuildNode, BuilderContext, Dependency, Error};

impl BuildNode for BSModifyOnceModifier {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        let mut children = Vec::new();

        if let Some(modifier) = &self.p_on_activate_modifier {
            children.push(Dependency::Node(modifier.ptr));
        }

        if let Some(modifier) = &self.p_on_deactivate_modifier {
            children.push(Dependency::Node(modifier.ptr));
        }

        Ok(children)
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        Ok(None)
    }
}
