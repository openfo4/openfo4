use std::collections::{HashMap, HashSet, VecDeque};

use game_common::components::Transform;
use game_core::hierarchy::{Hierarchy, Key};
use openfo4_core::ecs::Entity;

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

    pub fn root_bones(&self) -> Vec<&Bone> {
        let mut parents = HashSet::new();

        for bone in &self.bones {
            for children in &bone.children {
                parents.insert(*children);
            }
        }

        let mut root_bones = Vec::new();
        for bone in &self.bones {
            if !parents.contains(&bone.index) {
                root_bones.push(bone);
            }
        }

        root_bones
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
    pub culled: HashSet<usize>,
    pub transforms: BoneTransformHierarchy,
}

impl SkeletonInstance {
    pub fn new(skeleton: Skeleton, bones: HashMap<usize, Entity>) -> Self {
        let root_bones = skeleton
            .root_bones()
            .iter()
            .map(|bone| *bones.get(&bone.index).unwrap())
            .collect();

        let mut bones_bind_pose = HashMap::new();
        for (bone, transform) in skeleton
            .bones
            .iter()
            .zip(skeleton.bind_pose_bone_transforms())
        {
            bones_bind_pose.insert(bone.index, transform);
        }

        let mut transforms = BoneTransformHierarchy::default();
        let mut queue: VecDeque<_> = skeleton
            .root_bones()
            .iter()
            .map(|bone| (bone.index, None))
            .collect();
        while let Some((id, parent)) = queue.pop_front() {
            let bone = &skeleton.bones[id];
            let key = transforms.hierarchy.append(parent, bone.transform);
            transforms.bones.insert(id, key);

            let global = bones_bind_pose.get(&id).unwrap();
            transforms.global.insert(key, *global);

            for child in &bone.children {
                queue.push_back((*child, Some(key)));
            }
        }

        for bone in &skeleton.bones {
            transforms.set_local(bone.index, bone.transform);
        }

        Self {
            skeleton,
            bones,
            bones_bind_pose,
            root_bones,
            culled: HashSet::new(),
            transforms,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Culled;

#[derive(Clone, Debug, Default)]
pub struct BoneTransformHierarchy {
    hierarchy: Hierarchy<Transform>,
    // FIXME: Use VecMap instead.
    global: HashMap<Key, Transform>,
    bones: HashMap<usize, Key>,
    updated: Vec<usize>,
}

impl BoneTransformHierarchy {
    pub fn local(&self, bone: usize) -> Transform {
        let key = *self.bones.get(&bone).unwrap();
        *self.hierarchy.get(key).unwrap()
    }

    /// Returns the global [`Transform`] of the given `bone` relative to the skeleton root.
    pub fn global(&self, bone: usize) -> Transform {
        let key = self.bones.get(&bone).unwrap();
        *self.global.get(key).unwrap()
    }

    pub(crate) fn set_local(&mut self, bone: usize, transform: Transform) {
        let key = *self.bones.get(&bone).unwrap();
        *self.hierarchy.get_mut(key).unwrap() = transform;
        self.update_global(key, transform);
        self.updated.push(bone);
    }

    fn update_global(&mut self, root: Key, local_root: Transform) {
        let mut global_root = local_root;
        if let Some(parent) = self.hierarchy.parent_key(root) {
            let parent = self.global[&parent];
            global_root = parent * local_root;
        }

        let mut queue = vec![(root, global_root)];
        while let Some((key, global)) = queue.pop() {
            self.global.insert(key, global);

            for (child, local) in self.hierarchy.children(key).into_iter().flatten() {
                queue.push((child, global * *local));
            }
        }
    }

    pub(crate) fn drain_updated_local(&mut self) -> impl Iterator<Item = (usize, Transform)> {
        self.updated.drain(..).map(|index| {
            let key = *self.bones.get(&index).unwrap();
            let transform = *self.hierarchy.get(key).unwrap();
            (index, transform)
        })
    }
}
