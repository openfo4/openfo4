use std::collections::{HashMap, VecDeque};

use game_common::components::Transform;
use hecs::Entity;

#[derive(Clone, Debug, Default)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn bind_pose_bone_transforms(&self) -> Vec<Transform> {
        let mut parents = HashMap::new();
        for bone in &self.bones {
            for child in &bone.children {
                parents.insert(child, bone.index);
            }
        }

        let mut transforms = HashMap::new();
        let mut queue = VecDeque::new();

        for bone in &self.bones {
            if !parents.contains_key(&bone.index) {
                queue.push_back(bone.index);
                transforms.insert(bone.index, bone.transform);
            }
        }

        while let Some(index) = queue.pop_front() {
            let bone = &self.bones[index];
            let mut transform = bone.transform;

            if let Some(parent) = parents.get(&index) {
                let parent_transform = *transforms.get(parent).unwrap();
                transform = parent_transform * transform;
            }

            transforms.insert(index, transform);
            queue.extend(&bone.children);
        }

        self.bones
            .iter()
            .map(|bone| *transforms.get(&bone.index).unwrap())
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Bone {
    pub index: usize,
    pub name: String,
    pub children: Vec<usize>,
    /// Transformation of the bone relative to the parent.
    pub transform: Transform,
}

#[derive(Copy, Clone, Debug)]
pub struct SkeletonMarker;

#[derive(Copy, Clone, Debug)]
pub struct BoneMarker;

#[derive(Clone, Debug)]
pub struct SkeletonInstance {
    pub skeleton: Skeleton,
    pub bones: HashMap<usize, Entity>,
    pub bones_bind_pose: HashMap<usize, Transform>,
    pub root_bones: Vec<Entity>,
}
