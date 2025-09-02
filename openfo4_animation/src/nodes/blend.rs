use crate::nodes::{Context, GraphNode};
use crate::{BoneWeights, NodeId, Value, ValueExpression};

#[derive(Clone, Debug)]
pub struct Blend {
    pub blend_factor: ValueExpression,
    pub children: Vec<BlendChildren>,
    pub is_parametric: bool,
}

impl GraphNode for Blend {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for children in &self.children {
            if let Some(node) = children.node {
                ctx.activate_node(node);
            }
        }
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        for children in &self.children {
            if let Some(node) = children.node {
                ctx.deactivate_node(node);
            }
        }
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if self.is_parametric {
            let blend_parameter = self.blend_factor.eval(ctx.variables()).to_f32().unwrap();

            let weights = self
                .children
                .iter()
                .map(|node| node.weight.eval(ctx.variables()).to_f32().unwrap())
                .collect::<Vec<_>>();

            let Some((mut min, mut min_weight)) = weights
                .iter()
                .copied()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.total_cmp(&b))
            else {
                return;
            };

            let Some((mut max, mut max_weight)) = weights
                .iter()
                .copied()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(&b))
            else {
                return;
            };

            // Select the two children so that min < blend_parameter < max.
            // If blend_parameter < min we will choose the children min and min+1.
            // If blend_parameter > max we will choose the children max-1 and max.
            // If only one children exists we always clamp blend_parameter to that
            // exact weight.

            for (index, &weight) in weights.iter().enumerate() {
                if weight <= blend_parameter && weight > min_weight {
                    min = index;
                    min_weight = weight;
                }

                if weight >= blend_parameter && weight < max_weight {
                    max = index;
                    max_weight = weight;
                }
            }

            let blend_parameter = blend_parameter.clamp(min_weight, max_weight);

            for (index, child) in self.children.iter().enumerate() {
                let Some(node) = child.node else {
                    continue;
                };

                let weight = weights[index];

                let s = if min == index {
                    debug_assert_eq!(min_weight, weight);

                    f32::min(
                        1.0,
                        1.0 - (blend_parameter - min_weight) / (max_weight - min_weight),
                    )
                } else if max == index {
                    debug_assert_eq!(max_weight, weight);

                    f32::max(
                        0.0,
                        (blend_parameter - min_weight) / (max_weight - min_weight),
                    )
                } else {
                    0.0
                };

                let mut final_weights = BoneWeights::Shared(s);
                if let Some(bone_weights) = &child.bone_weights {
                    let bone_weights = match bone_weights.eval(ctx.variables()) {
                        Value::BoneWeights(v) => v,
                        _ => unreachable!(),
                    };

                    final_weights *= &BoneWeights::Unique(bone_weights);
                }

                ctx.set_bone_weights(node, final_weights);
            }
        } else {
            let weights = self
                .children
                .iter()
                .map(|child| child.weight.eval(ctx.variables()).to_f32().unwrap())
                .collect::<Vec<_>>();

            let sum = weights.iter().sum::<f32>();

            for (child, weight) in self.children.iter().zip(weights) {
                let Some(node) = child.node else {
                    continue;
                };

                let weight = if sum.abs() == 0.0 { 0.0 } else { weight / sum };

                let mut final_weights = BoneWeights::Shared(weight);
                if let Some(bone_weights) = &child.bone_weights {
                    let bone_weights = match bone_weights.eval(ctx.variables()) {
                        Value::BoneWeights(v) => v,
                        _ => unreachable!(),
                    };

                    final_weights *= &BoneWeights::Unique(bone_weights);
                }

                ctx.set_bone_weights(node, final_weights);
            }
        }
    }

    fn children(&self) -> Vec<NodeId> {
        self.children.iter().filter_map(|c| c.node).collect()
    }
}

#[derive(Clone, Debug)]
pub struct BlendChildren {
    pub node: Option<NodeId>,
    pub weight: ValueExpression,
    pub bone_weights: Option<ValueExpression>,
}
