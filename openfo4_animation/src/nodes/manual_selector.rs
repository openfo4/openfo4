use std::collections::HashMap;

use crate::nodes::{Context, GraphNode};
use crate::{NodeId, ValueExpression};

#[derive(Clone, Debug)]
pub struct ManualSelector {
    pub states: HashMap<usize, NodeId>,
    pub index_value: ValueExpression,
    pub can_change_after_activation: bool,
    pub active_index: Option<usize>,
}

impl GraphNode for ManualSelector {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        let index = self.index_value.eval(ctx.variables()).to_usize().unwrap();

        let Some(node) = self.states.get(&index) else {
            self.active_index = None;
            return;
        };

        ctx.activate_node(*node);
        self.active_index = Some(index);
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if let Some(index) = self.active_index.take() {
            let node = self.states[&index];
            ctx.deactivate_node(node);
        }
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        let index = self.index_value.eval(ctx.variables()).to_usize().unwrap();

        if self.active_index == Some(index) || !self.can_change_after_activation {
            return;
        }

        if let Some(node) = self.active_index.take() {
            ctx.deactivate_node(node);
        }

        let Some(node) = self.states.get(&index) else {
            self.active_index = None;
            return;
        };

        ctx.activate_node(*node);
        self.active_index = Some(index);
    }

    fn children(&self) -> Vec<NodeId> {
        self.states.values().copied().collect()
    }
}
