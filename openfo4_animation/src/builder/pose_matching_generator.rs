use openfo4_hkx::behavior::PoseMatchingGenerator;

use crate::Node;
use crate::builder::{BuildNode, BuilderContext, Error};

impl BuildNode for PoseMatchingGenerator {
    fn dependencies(&self, ctx: &mut BuilderContext<'_>) -> Result<Vec<super::Dependency>, Error> {
        self.parent.dependencies(ctx)
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        self.parent.build(ctx)
    }
}
