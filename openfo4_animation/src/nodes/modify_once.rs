use crate::NodeId;
use crate::nodes::{Context, GraphNode};

#[derive(Clone, Debug)]
pub struct ModifyOnce {
    pub on_activate: Vec<NodeId>,
    pub on_deactivate: Vec<NodeId>,
}

impl GraphNode for ModifyOnce {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for node in &self.on_activate {
            ctx.activate_node(*node);
        }

        for node in &self.on_activate {
            ctx.deactivate_node(*node);
        }
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for node in &self.on_deactivate {
            ctx.activate_node(*node);
        }

        for node in &self.on_deactivate {
            ctx.deactivate_node(*node);
        }
    }

    fn children(&self) -> Vec<NodeId> {
        self.on_activate
            .iter()
            .chain(&self.on_deactivate)
            .copied()
            .collect()
    }
}
