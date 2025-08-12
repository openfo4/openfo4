use game_common::components::Axis;
use glam::Vec3;

#[derive(Copy, Clone, Debug)]
pub struct RigidBody {
    pub kind: RigidBodyKind,
    pub linvel: Vec3,
    pub angvel: Vec3,
}

impl RigidBody {
    pub const FIXED: Self = Self {
        kind: RigidBodyKind::Fixed,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    };

    pub const DYNAMIC: Self = Self {
        kind: RigidBodyKind::Dynamic,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RigidBodyKind {
    Dynamic,
    Fixed,
}

#[derive(Clone, Debug)]
pub struct Collider {
    pub friction: f32,
    pub restitution: f32,
    pub shape: ColliderShape,
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Capsule(Capsule),
    TriMesh(TriMesh),
}

#[derive(Clone, Debug)]
pub struct Capsule {
    pub axis: Axis,
    pub half_height: f32,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct TriMesh {
    vertices: Vec<Vec3>,
    indices: Vec<u32>,
    convex: bool,
}

impl TriMesh {
    pub fn new(vertices: Vec<Vec3>, indices: Vec<u32>) -> Self {
        assert!(!indices.is_empty());
        assert_eq!(indices.len() % 3, 0);
        Self {
            vertices,
            indices,
            convex: false,
        }
    }

    /// Creates a new `TriMesh` that is assumed to be convex.
    pub fn new_convex(vertices: Vec<Vec3>, indices: Vec<u32>) -> Self {
        assert!(!indices.is_empty());
        assert_eq!(indices.len() % 3, 0);
        Self {
            vertices,
            indices,
            convex: true,
        }
    }

    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn is_convex(&self) -> bool {
        self.convex
    }
}
