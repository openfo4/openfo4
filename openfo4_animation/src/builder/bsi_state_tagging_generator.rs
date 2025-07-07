//! BSiStateTaggingGenerator

use openfo4_hkx::OpaquePtr;
use openfo4_hkx::behavior::BSiStateTaggingGenerator;

use crate::builder::{BuildNode, Error, GraphBuilderState};
use crate::{Node, NodeInner};

impl BuildNode for BSiStateTaggingGenerator {
    fn children(&self, _state: &mut GraphBuilderState<'_>) -> Result<Vec<OpaquePtr>, Error> {
        if let Some(generator) = &self.p_default_generator {
            Ok(vec![generator.ptr])
        } else {
            Ok(Vec::new())
        }
    }

    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let mut list = Vec::new();

        if let Some(generator) = &self.p_default_generator {
            let node = *state.ptr_to_id.get(&generator.ptr).unwrap();
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
