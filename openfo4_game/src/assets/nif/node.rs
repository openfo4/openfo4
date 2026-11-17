use glam::{Quat, Vec3};
use openfo4_animation::skeleton::Bone;
use openfo4_common::components::Transform;
use openfo4_nif::{Block, BlockRef, NiNode};

use crate::assets::nif::{
    AttachmentPoint, Node, Visit, Visitor, extract_attachment_name, is_marker,
};

impl Visit for NiNode {
    async fn visit(&self, visitor: &mut Visitor<'_>, id: BlockRef) -> Option<Node> {
        if is_marker(&self.name) {
            return None;
        }

        // For some scenes the transform of the root node breaks the
        // transform exactly by the root transform, so maybe the transform
        // of the root node is not applied?
        let transform = if visitor.parent(id).is_none() {
            Transform::default()
        } else {
            Transform {
                translation: self.translation,
                rotation: Quat::from_mat3(&self.rotation).normalize().conjugate(),
                scale: Vec3::splat(self.scale),
            }
        };

        for children in &self.children {
            visitor.enqueue(*children);
        }

        if let Some(collision_object) = self.collision_object {
            visitor.enqueue(collision_object);
        }

        let mut attachments = Vec::new();
        if let Some(points) = visitor.connect_points_parents {
            for point in &points.points {
                // point.parent contains the name of the node where the attachment
                // will be mounted, except for some "dummy" weapon base files with
                // no child nodes it is empty, so it should probably refer to the
                // root node then?
                if point.parent != self.name
                    && visitor.parent(id).is_some()
                    && !point.parent.is_empty()
                {
                    continue;
                }

                let name = extract_attachment_name(&point.name);

                let offset = Transform {
                    translation: point.translation,
                    rotation: point.rotation,
                    scale: Vec3::splat(point.scale),
                };

                attachments.push(AttachmentPoint {
                    name: name.to_owned(),
                    offset,
                });
            }
        }

        let mut attaches = self.name.clone();
        for block in &self.extra_data {
            match visitor.nif.get(*block) {
                Some(Block::NiStringExtraData(data)) if data.name.eq_ignore_ascii_case("Prn") => {
                    attaches = data.data.clone();
                }
                _ => {}
            }
        }

        let bone = visitor.build_skeleton.then(|| {
            let index = visitor.bones.len();
            visitor.bones.push(Bone {
                index,
                name: self.name.clone(),
                children: Vec::new(),
                transform,
            });

            if let Some(parent) = visitor.parent_node(id)
                && let Some(parent) = parent.bone
            {
                visitor.bones[parent].children.push(index);
            }

            index
        });

        Some(Node {
            name: self.name.clone(),
            transform,
            attaches,
            attachments,
            bone,
            ..Default::default()
        })
    }

    fn transform(&self) -> Option<Transform> {
        Some(Transform {
            translation: self.translation,
            rotation: Quat::from_mat3(&self.rotation).normalize().conjugate(),
            scale: Vec3::splat(self.scale),
        })
    }
}
