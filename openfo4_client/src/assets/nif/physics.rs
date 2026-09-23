use std::collections::VecDeque;

use game_common::components::Transform;
use glam::Vec3;
use hashbrown::HashSet;
use openfo4_hkx::{HkObject, HkxFile};
use openfo4_nif::{BhkNPCollisionObject, Block, BlockRef, Nif};
use openfo4_physics::components::{Collider, ColliderShape, CollisionLayer, TriMesh};

use crate::assets::nif::{
    ColliderNode, LoadColliderError, LoadNifFlags, NIF_TO_METERS, Node, Visit, Visitor,
};

impl Visit for BhkNPCollisionObject {
    async fn visit(&self, visitor: &mut Visitor<'_>, id: BlockRef) -> Option<Node> {
        if !visitor.key.flags.contains(LoadNifFlags::COLLIDERS) {
            return None;
        }

        match load_collider(&visitor.nif, id) {
            Ok(Some(collider)) => Some(Node {
                transform: Transform::from_scale(Vec3::splat(1.0 / NIF_TO_METERS))
                    * collider.transform,
                collider: Some(collider.collider),
                ..Default::default()
            }),
            Ok(None) => None,
            Err(err) => {
                tracing::error!("failed to load collider from {}: {}", visitor.key.path, err);
                None
            }
        }
    }
}

fn load_collider(
    nif: &Nif,
    collision_object: BlockRef,
) -> Result<Option<ColliderNode>, LoadColliderError> {
    // The collision object is a `bhkNPCollisionObject` node, which
    // contains a reference to a `bhkPhysicsSystem` node that contains
    // a HKX file.
    // The `BodyID` of the `bhkNPCollisionObject` node contains the index
    // of the body that should be selected from the HKX file.
    let collision_object = match nif.get(collision_object) {
        Some(Block::BhkNPCollisionObject(object)) => object,
        _ => todo!(),
    };

    let data = match nif.get(collision_object.data) {
        Some(Block::BhkPhysicsSystem(data)) => &data.data,
        Some(Block::BhkRagdollSystem(data)) => &data.data,
        _ => todo!(),
    };

    let hkx = HkxFile::decode(data).map_err(LoadColliderError::Hkx)?;

    let system = hkx
        .iter()
        .find_map(|obj| match obj {
            HkObject::PhysicsSystemData(data) => Some(data),
            HkObject::RagdollData(data) => Some(&data.physics_system),
            _ => None,
        })
        .unwrap();

    let body = system
        .body_cinfos
        .get(collision_object.body_id as usize)
        .unwrap();

    let Some(material) = system.materials.get(usize::from(body.material_id)) else {
        return Err(LoadColliderError::InvalidMaterialId(body.material_id));
    };

    let friction = material.static_friction.to_f32();
    let restitution = material.restitution.to_f32();

    // Note that unlike everything else in a NIF file,
    // the hkx transforms/collider meshes use meters.
    let transform = Transform {
        // There are position and orientation attributes on the body,
        // but they seem to not be applied?
        ..Default::default()
    };

    let mut meshes = Vec::new();
    let mut all_meshes_are_convex = true;

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((body.shape, Transform::default()));

    while let Some((ptr, mut transform)) = queue.pop_front() {
        // Since composite shapes can point to any other shape
        // we need to catch cycles we may get stuck here forever.
        if !visited.insert(ptr) {
            return Err(LoadColliderError::PointerCylce);
        }

        match hkx.get(ptr) {
            Some(HkObject::CompressedMeshShape(shape)) => {
                let mesh = shape
                    .data
                    .mesh()
                    .map_err(LoadColliderError::InvalidCompressedMeshShape)?;

                meshes.push((mesh, transform));
                all_meshes_are_convex = false;
            }
            Some(HkObject::ConvexPolytopeShape(shape)) => {
                let mesh = shape.to_tri_mesh();
                meshes.push((mesh, transform));
            }
            Some(HkObject::CapsuleShape(shape)) => {
                let mesh = shape.to_tri_mesh();
                meshes.push((mesh, transform));
            }
            Some(HkObject::ConvexShape(shape)) => {
                let mesh = shape.mesh();
                meshes.push((mesh, transform));
            }
            Some(HkObject::ScaledConvexShape(shape)) => {
                // FIXME: Which of these two is applied first,
                // or rather the question is whether translation
                // should already be scaled by the new scale value.
                transform.translation += shape.translation.truncate();
                transform.scale *= shape.scale.truncate();

                queue.push_back((shape.core_shape, transform));
            }
            Some(HkObject::DynamicCompoundShape(shape)) => {
                for instance in &*shape.instances {
                    let Some(shape) = instance.shape else {
                        continue;
                    };

                    let (_, rotation, translation) =
                        instance.transform.to_scale_rotation_translation();
                    let scale = instance.scale.truncate();

                    let transform = transform
                        * Transform {
                            translation,
                            rotation,
                            scale,
                        };

                    queue.push_back((shape, transform));
                }
            }
            _ => return Err(LoadColliderError::UnknownMeshShape),
        }
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (mesh, transform) in meshes {
        let base_index = vertices.len() as u32;

        for vertex in mesh.vertices {
            let vertex = transform.transform_point(vertex);
            vertices.push(vertex);
        }

        for index in mesh.indices {
            let index = base_index + index;
            debug_assert!(index < vertices.len() as u32);
            indices.push(index);
        }
    }

    if indices.is_empty() {
        return Ok(None);
    }

    let mesh = if all_meshes_are_convex {
        TriMesh::new_convex(vertices, indices)
    } else {
        TriMesh::new(vertices, indices)
    };

    Ok(Some(ColliderNode {
        transform,
        collider: Collider {
            // TODO: Load values from material property.
            friction: 0.5,
            restitution: 0.5,
            shape: ColliderShape::TriMesh(mesh),
            layer: CollisionLayer::STATIC,
        },
    }))
}
