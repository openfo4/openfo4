use std::collections::HashMap;

use glam::Vec3;

use crate::ValueExpression;
use crate::ik::Constraint;
use crate::nodes::{Context, DirectAtModifier, GraphNode, Modifier};

#[derive(Clone, Debug)]
pub struct DirectAt {
    pub enable: ValueExpression,
    pub start_bone_index: ValueExpression,
    pub end_bone_index: ValueExpression,
    pub source_bone_index: ValueExpression,
    pub camera_x: ValueExpression,
    pub camera_y: ValueExpression,
    pub camera_z: ValueExpression,
}

impl GraphNode for DirectAt {
    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if !self.enable.eval(ctx.variables()).to_bool().unwrap() {
            return;
        }

        let start_bone_index = self
            .start_bone_index
            .eval(ctx.variables())
            .to_usize()
            .unwrap();
        let end_bone_index = self
            .end_bone_index
            .eval(ctx.variables())
            .to_usize()
            .unwrap();
        let source_bone_index = self
            .source_bone_index
            .eval(ctx.variables())
            .to_usize()
            .unwrap();

        let camera_x = self.camera_x.eval(ctx.variables()).to_f32().unwrap();
        let camera_y = self.camera_y.eval(ctx.variables()).to_f32().unwrap();
        let camera_z = self.camera_z.eval(ctx.variables()).to_f32().unwrap();
        let camera = Vec3::new(camera_x, camera_y, camera_z) * 10000.0;

        let mut constraints = HashMap::new();
        let mut bone = ctx.skeleton().bones.get(source_bone_index).unwrap();

        let mut is_in_chain = false;
        let mut chain = Vec::new();

        loop {
            constraints.insert(bone.index, Constraint::Fixed);

            if bone.index == end_bone_index {
                is_in_chain = true;
            }

            if is_in_chain {
                chain.push(bone.index);
            }

            if bone.index == start_bone_index {
                break;
            }

            let parent = ctx
                .skeleton()
                .bones
                .iter()
                .find(|b| b.children.contains(&bone.index))
                .unwrap();
            bone = parent;
        }

        for bone in chain {
            constraints.insert(bone, Constraint::None);
        }

        // ctx.add_modifier(Modifier::InverseKinematics(InverseKinematicsModifier {
        //     start_bone: start_bone_index,
        //     end_bone: source_bone_index,
        //     target: camera,
        //     constraints,
        // }));
        ctx.add_modifier(Modifier::DirectAt(DirectAtModifier {
            detector_bone: source_bone_index,
            chain_start_bone: start_bone_index,
            chain_end_bone: end_bone_index,
            target: camera,
        }));
    }
}
