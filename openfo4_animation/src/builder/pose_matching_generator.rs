use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::PoseMatchingGenerator;

use crate::Node;
use crate::builder::{BuildNode, Error, GraphBuilderState};

impl BuildNode for PoseMatchingGenerator {
    fn children(&self, state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        self.parent.children(state)
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        self.parent.build(state)
    }
}
