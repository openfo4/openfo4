//! BSEventOnDeactivateModifier

use openfo4_hkx::behavior::BSEventOnDeactivateModifier;

use crate::builder::{BuildNode, BuilderContext, Error};
use crate::{Node, parse_event};

impl BuildNode for BSEventOnDeactivateModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let event = parse_event(ctx.hkx_file(), &self.event);

        Ok(None)
    }
}
