use crate::events::EventId;
use crate::nodes::{Context, GraphNode};
use crate::{Event, NodeId};

#[derive(Clone, Debug)]
pub struct EventDriven {
    pub node: NodeId,
    pub on_event: Option<EventId>,
    pub off_event: Option<EventId>,
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
        if self.on_event.is_some_and(|id| id == event.id) && !self.is_active {
            ctx.activate_node(self.node);
            self.is_active = true;
        }

        if self.off_event.is_some_and(|id| id == event.id) && self.is_active {
            ctx.deactivate_node(self.node);
            self.is_active = false;
        }
    }

    fn children(&self) -> Vec<NodeId> {
        vec![self.node]
    }
}
