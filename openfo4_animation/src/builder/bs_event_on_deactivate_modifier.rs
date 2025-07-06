//! BSEventOnDeactivateModifier

use openfo4_hkx::behavior::BSEventOnDeactivateModifier;

use crate::builder::{BuildNode, Error, GraphBuilderState};
use crate::{Node, parse_event};

impl BuildNode for BSEventOnDeactivateModifier {
    fn build(&self, state: &mut GraphBuilderState<'_>) -> Result<Option<Node>, Error> {
        let event = parse_event(&state.file, &self.event);

        Ok(None)
    }
}
