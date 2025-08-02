//! hkbBehaviorReferenceGenerator

use openfo4_hkx::behavior::BehaviorReferenceGenerator;

use crate::Node;
use crate::builder::{BuildNode, BuilderContext, Error};

impl BuildNode for BehaviorReferenceGenerator {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        Ok(None)
    }
}
