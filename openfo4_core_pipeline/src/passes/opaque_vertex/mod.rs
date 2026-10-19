//! Main opaque geometry pass using the vertex shader based pipeline

use std::sync::Arc;

use forward::OpaqueVertexForwardPass;
use openfo4_render::api::CommandQueue;
use openfo4_render::graph::{NodeLabel, RenderGraph, SlotFlags, SlotKind, SlotLabel};
use parking_lot::Mutex;

use crate::passes::{HDR_TEXTURE, OPAQUE_DEPTH, SKINNING_PASS};

use super::State;

mod forward;
mod pipelines;
mod shadow;

const FORWARD_PASS: NodeLabel = NodeLabel::new("opaque_vertex_forward_pass");

pub fn init(graph: &mut RenderGraph, queue: &CommandQueue<'_>, state: Arc<Mutex<State>>) {
    let forward_pass = OpaqueVertexForwardPass::new(queue, state);
    graph.add_node(FORWARD_PASS, forward_pass);

    graph.add_node_dependency(FORWARD_PASS, SKINNING_PASS);

    graph.add_slot_dependency(
        FORWARD_PASS,
        HDR_TEXTURE,
        SlotKind::Texture,
        SlotFlags::WRITE,
    );
    graph.add_slot_dependency(
        FORWARD_PASS,
        SlotLabel::SURFACE,
        SlotKind::Texture,
        SlotFlags::READ,
    );

    graph.add_slot_dependency(
        FORWARD_PASS,
        OPAQUE_DEPTH,
        SlotKind::Texture,
        SlotFlags::WRITE,
    );
}
