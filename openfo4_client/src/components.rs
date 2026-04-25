use std::ops::{Mul, MulAssign};

use game_common::components::{Color, Transform};
use glam::{IVec3, Quat, Vec3};
use hashbrown::HashSet;
use openfo4_core::ecs::Entity;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;

use crate::systems::camera::CameraMode;
use crate::systems::inventory::ItemStack;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub mode: CameraMode,
    pub fov: f32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct NiTransform(Transform);

impl NiTransform {
    pub fn from_ni_transform(transform: Transform) -> Self {
        Self(transform)
    }

    pub fn to_ni_transform(self) -> Transform {
        self.0
    }

    pub fn from_translation(translation: Vec3) -> Self {
        Self(Transform::from_translation(translation))
    }

    pub fn translation(&self) -> Vec3 {
        self.0.translation
    }

    pub fn rotation(&self) -> Quat {
        self.0.rotation
    }

    pub fn scale(&self) -> Vec3 {
        self.0.scale
    }
}

impl From<Transform> for NiTransform {
    fn from(value: Transform) -> Self {
        Self(value.to_nif())
    }
}

impl From<NiTransform> for Transform {
    fn from(value: NiTransform) -> Self {
        value.0.to_opengl()
    }
}

impl Mul for NiTransform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl MulAssign for NiTransform {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

/// A root entity.
///
/// A root entity represents a gameplay entity (such as Actor, Object, etc.) and has no
/// [`Children`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Root;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RootParent(pub Entity);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Player;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Persistent;

/// Indicates the worldspace that the entity is contained in.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorldSpace {
    pub record: RecordId,
    pub is_interior: bool,
}

/// Marker for all actor entities (including player).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ActorMarker;

/// Marker for dead actors.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeadActor;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LookingAtTarget(pub Entity);

/// An item placed in the world.
#[derive(Clone, Debug)]
pub struct PlacedItem(pub ItemStack);

#[derive(Clone, Debug)]
pub struct TriggerBox {
    pub half_extents: Vec3,
    pub marker_color: Color,
    pub entities_inside: HashSet<Entity>,
}

/// Assorted actor properties.
#[derive(Clone, Debug)]
pub struct ActorProperties {
    pub protection: ActorProtection,
    /// Whether bleedout recovery is enabled.
    ///
    /// If `true` the actor will automatically recover when `Protected` or `Essential` and reach
    /// zero health.
    pub bleedout_recovery: bool,
    pub level: u32,
    /// The number of available perk points to spend.
    pub perk_points: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActorProtection {
    /// No protection.
    None,
    /// Protected
    ///
    /// Protected actors ignore lethal damage from any non-player actors.
    Protected,
    /// Essential
    ///
    /// Essential actors ignore lethal damage from any actor.
    Essential,
}

/// Object bounds as reported in the form in NIF space.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ObjectBounds {
    pub min: IVec3,
    pub max: IVec3,
}

#[derive(Copy, Clone, Debug)]
pub struct BaseObject {
    pub record: RecordId,
}

/// Assorted object reference properties.
#[derive(Clone, Debug)]
pub struct ObjectProperties {
    pub lock: Option<ObjectLockData>,
}

#[derive(Copy, Clone, Debug)]
pub struct ObjectLockData {
    pub level: Option<ObjectLockLevel>,
    pub key: Option<RecordId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectLockLevel {
    Noivce,
    Advanced,
    Expert,
    Master,
    Barred,
    Chained,
    Inaccessible,
    RequiresTerminal,
    RequiresKey,
}
