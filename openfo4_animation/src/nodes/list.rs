use crate::NodeId;
use crate::nodes::{Context, GraphNode};

#[derive(Clone, Debug)]
pub struct List {
    pub nodes: Vec<NodeId>,
}

impl GraphNode for List {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for node in &self.nodes {
            ctx.activate_node(*node);
        }
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for node in &self.nodes {
            ctx.deactivate_node(*node);
        }
    }

    fn children(&self) -> Vec<NodeId> {
        self.nodes.clone()
    }
}
