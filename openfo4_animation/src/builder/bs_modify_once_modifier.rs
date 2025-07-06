use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::BSModifyOnceModifier;

use crate::Node;
use crate::builder::{BuildNode, Error, GraphBuilderState};

impl BuildNode for BSModifyOnceModifier {
    fn children(&self, state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        let mut children = Vec::new();

        if let Some(modifier) = &self.p_on_activate_modifier {
            children.push(modifier.ptr);
        }

        if let Some(modifier) = &self.p_on_deactivate_modifier {
            children.push(modifier.ptr);
        }

        Ok(children)
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        Ok(None)
    }
}
