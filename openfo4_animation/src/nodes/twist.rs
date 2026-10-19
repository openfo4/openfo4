use glam::{Quat, Vec3};
use openfo4_common::components::Transform;

use crate::ValueExpression;
use crate::nodes::{Context, GraphNode, Modifier, OffsetModifier};

#[derive(Clone, Debug)]
pub struct Twist {
    pub enable: ValueExpression,
    pub axis: Vec3,
    pub angle: ValueExpression,
    pub start_bone: usize,
    pub end_bone: usize,
}

impl GraphNode for Twist {
    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if self.enable.eval(ctx.variables()).to_bool() != Some(true) {
            return;
        }

        let angle = self.angle.eval(ctx.variables()).to_f32().unwrap();

        let mut bones = Vec::new();
        let mut bone = self.end_bone;
        loop {
            bones.push(bone);

            if bone == self.start_bone {
                break;
            }

            let Some(next_bone) = ctx
                .skeleton()
                .bones
                .iter()
                .find(|v| v.children.contains(&bone))
            else {
                break;
            };

            bone = next_bone.index;
        }

        if bones.is_empty() {
            return;
        }

        let rotation = Quat::from_axis_angle(self.axis, angle);

        let per_bone_factor = 1.0 / bones.len() as f32;
        for bone in bones {
            let rotation = Quat::IDENTITY.slerp(rotation, per_bone_factor);

            ctx.add_modifier(Modifier::Offset(OffsetModifier {
                bone,
                transform: Transform {
                    rotation,
                    ..Default::default()
                },
            }));
        }
    }
}
