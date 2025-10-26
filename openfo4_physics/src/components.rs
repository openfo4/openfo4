use game_common::components::Axis;
use glam::Vec3;
use rapier3d::prelude::{Group, InteractionGroups};

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
    pub layer: CollisionLayer,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CollisionLayer(Group);

impl CollisionLayer {
    /// Static objects.
    pub const STATIC: Self = Self(Group::GROUP_1);
    /// Character controllers.
    pub const CHARACTER_CONTROLLER: Self = Self(Group::GROUP_2);
    /// Weapon projectiles.
    pub const PROJECTILE: Self = Self(Group::GROUP_3);

    pub(crate) fn to_interaction_groups(self) -> InteractionGroups {
        match self {
            Self::STATIC => InteractionGroups {
                memberships: self.0,
                filter: self.0 | Self::CHARACTER_CONTROLLER.0 | Self::PROJECTILE.0,
            },
            Self::CHARACTER_CONTROLLER => InteractionGroups {
                memberships: self.0,
                filter: self.0 | Self::STATIC.0,
            },
            Self::PROJECTILE => InteractionGroups {
                memberships: self.0,
                filter: self.0 | Self::STATIC.0,
            },
            _ => unreachable!(),
        }
    }
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
