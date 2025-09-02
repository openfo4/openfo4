//! BSEventOnDeactivateModifier

use openfo4_hkx::behavior::BSEventOnDeactivateModifier;

use crate::builder::{BuildNode, BuilderContext, Error, parse_event};
use crate::nodes::Node;

impl BuildNode for BSEventOnDeactivateModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let event = parse_event(ctx, &self.event);

        Ok(None)
    }
}
