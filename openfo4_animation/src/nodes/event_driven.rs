use crate::nodes::{Context, GraphNode, NodeEvent};
use crate::{Event, NodeId};

#[derive(Clone, Debug)]
pub struct EventDriven {
    pub node: NodeId,
    pub on_event: i64,
    pub off_event: i64,
    pub on_by_default: bool,
    pub is_active: bool,
}

impl GraphNode for EventDriven {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if self.on_by_default {
            ctx.activate_node(self.node);
            self.is_active = true;
        }
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if self.is_active {
            ctx.deactivate_node(self.node);
            self.is_active = false;
        }
    }

    fn handle_event<T>(&mut self, mut ctx: T, event: &Event)
    where
        T: Context,
    {
        if event.id == self.on_event && !self.is_active {
            ctx.activate_node(self.node);
            self.is_active = true;
        }

        if event.id == self.off_event && self.is_active {
            ctx.deactivate_node(self.node);
            self.is_active = false;
        }
    }

    fn children(&self) -> Vec<NodeId> {
        vec![self.node]
    }
}
