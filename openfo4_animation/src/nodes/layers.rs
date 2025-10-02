use crate::events::EventId;
use crate::nodes::{Context, GraphNode};
use crate::{BoneWeights, Event, NodeId, Value, ValueExpression};

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
            if layer.on_by_default.eval(ctx.variables()).to_bool().unwrap() {
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

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for layer in &self.layers {
            if !layer.is_active {
                continue;
            }

            let weight = layer.weight.eval(ctx.variables()).to_f32().unwrap();

            let mut final_weights = BoneWeights::Shared(weight);
            if let Some(bone_weights) = &layer.bone_weights {
                let bone_weights = match bone_weights.eval(ctx.variables()) {
                    Value::BoneWeights(v) => v,
                    _ => unreachable!(),
                };

                final_weights *= &BoneWeights::Unique(bone_weights);
            }

            ctx.set_bone_weights(layer.node, final_weights);
        }
    }

    fn handle_event<T>(&mut self, mut ctx: T, event: &Event)
    where
        T: Context,
    {
        for layer in &mut self.layers {
            if layer.on_event.is_some_and(|id| id == event.id) && !layer.is_active {
                ctx.activate_node(layer.node);
                layer.is_active = true;
            }

            if layer.off_event.is_some_and(|id| id == event.id) && layer.is_active {
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
    pub on_by_default: ValueExpression,
    pub on_event: Option<EventId>,
    pub off_event: Option<EventId>,
    pub weight: ValueExpression,
    pub bone_weights: Option<ValueExpression>,
    pub is_active: bool,
}
