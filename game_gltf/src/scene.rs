use game_common::collections::hierarchy::Hierarchy;

use crate::GltfNode;

#[derive(Clone, Debug)]
pub struct GltfScene {
    pub nodes: Hierarchy<GltfNode>,
}
