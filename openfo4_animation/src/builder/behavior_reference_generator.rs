//! hkbBehaviorReferenceGenerator

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::BehaviorReferenceGenerator;

use crate::Node;
use crate::builder::{BuildNode, Error, GraphBuilderState};

impl BuildNode for BehaviorReferenceGenerator {
    fn children(&self, state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        Ok(Vec::new())
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        Ok(None)
    }
}
