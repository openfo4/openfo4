use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::BSOffsetAnimationGenerator;

use crate::builder::{BuildNode, Error, GraphBuilderState};
use crate::{Node, NodeInner};

impl BuildNode for BSOffsetAnimationGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        let mut children = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            children.push(generator.ptr);
        }

        if let Some(generator) = &self.p_offset_clip_generator {
            children.push(generator.ptr);
        }

        Ok(children)
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut nodes = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            if let Some(node) = state.ptr_to_id.get(&generator.ptr) {
                nodes.push(*node);
            }
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            inner: NodeInner::List(nodes),
            ..Default::default()
        }))
    }
}
