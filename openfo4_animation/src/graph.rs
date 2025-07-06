use slab::Slab;

use crate::{Event, NodeId, ValueExpression, VariableId};

#[derive(Clone, Debug)]
pub struct AnimationGraph {
    pub nodes: Slab<Node>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub on_update: Vec<Command>,
    pub on_activate: Vec<Command>,
    pub on_deactivate: Vec<Command>,
}

#[derive(Clone, Debug)]
pub enum Command {
    /// Run a list of commands only if the condition is true.
    Conditional(ValueExpression, Vec<Command>),
    ActivateNode(NodeId),
    DeactivateNode(NodeId),
    SetVariable(SetVariable),
    SendEvent(Event),
}

#[derive(Clone, Debug)]
pub struct SetVariable {
    pub variable: VariableId,
    pub value: ValueExpression,
}
