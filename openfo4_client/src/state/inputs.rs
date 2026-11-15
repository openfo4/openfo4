use glam::{Quat, Vec3};

#[derive(Copy, Clone, Debug)]
pub enum ActorInputEvent {
    /// Move the actor in the given [`Direction`].
    ///
    /// Speed is a factor between 0.0 (no movement) and 1.0 (max movement speed).
    Move {
        direction: Direction,
        speed: f32,
    },
    /// Turn the actor around the given rotation.
    // FIXME: Merge these two
    TurnYaw(Quat),
    TurnPitch(Quat),
    /// Toggle sneaking.
    ToggleSneak,
    /// Toggle sprinting.
    ToggleSprint,
    /// Toggle walking.
    ToggleWalk,
    /// Attack with the equipped weapon.
    WeaponAttack,
    /// Draw the currently equipped weapon.
    WeaponDraw,
    /// Reload the currently equipped weapon.
    WeaponReload,
    /// Activate the currently facing entity.
    Activate,
    ToggleFreeCamera,
}

#[derive(Copy, Clone, Debug)]
pub struct Direction(Vec3);

impl Direction {
    /// The direction associated with forwards.
    pub const FRONT: Self = Self(Vec3::NEG_Z);

    /// The direction associated with backwards.
    pub const BACK: Self = Self(Vec3::Z);

    /// The direction associated with left.
    pub const LEFT: Self = Self(Vec3::NEG_X);

    /// The direction associated with right.
    pub const RIGHT: Self = Self(Vec3::X);

    pub(crate) fn to_vec3(self) -> Vec3 {
        self.0
    }
}
