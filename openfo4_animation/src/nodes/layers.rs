use crate::nodes::{Context, GraphNode, NodeEvent};
use crate::{Event, NodeId};

#[derive(Clone, Debug)]
pub struct Layers {
    pub layers: Vec<Layer>,
}

impl GraphNode for Layers {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for layer in &mut self.layers {
            if layer.on_by_default {
                ctx.activate_node(layer.node);
                layer.is_active = true;
            }
        }
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for layer in &mut self.layers {
            if layer.is_active {
                ctx.deactivate_node(layer.node);
                layer.is_active = false;
            }
        }
    }

    fn handle_event<T>(&mut self, mut ctx: T, event: &Event)
    where
        T: Context,
    {
        for layer in &mut self.layers {
            if layer.on_event == event.id && !layer.is_active {
                ctx.activate_node(layer.node);
                layer.is_active = true;
            }

            if layer.off_event == event.id && layer.is_active {
                ctx.deactivate_node(layer.node);
                layer.is_active = false;
            }
        }
    }

    fn children(&self) -> Vec<NodeId> {
        self.layers.iter().map(|v| v.node).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub node: NodeId,
    pub on_by_default: bool,
    pub on_event: i64,
    pub off_event: i64,
    pub is_active: bool,
}
