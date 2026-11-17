use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartialEntityFlags {
    /// Graphics related components.
    pub graphics: bool,
    /// Physics related components.
    pub physics: bool,
    pub actor: bool,
    pub scene: bool,
    pub script: bool,
    pub lod: LodLevel,
}

impl PartialEntityFlags {
    pub const NONE: Self = Self {
        graphics: false,
        physics: false,
        actor: false,
        scene: false,
        script: false,
        lod: LodLevel::Normal,
    };

    /// Returns `true` if `other` is a subset of `self`.
    pub fn is_subset(&self, other: &Self) -> bool {
        let mut res = true;
        res &= self.graphics == other.graphics || !other.graphics;
        res &= self.physics == other.physics || !other.physics;
        res &= self.actor == other.actor || !other.actor;
        res &= self.scene == other.scene || !other.scene;
        res &= self.script == other.script || !other.script;
        res &= self.lod <= other.lod;
        res
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LodLevel {
    Normal,
    Distant0,
}

#[derive(Copy, Clone, Debug, Default)]
pub(super) struct PartialEntityFlagCounts {
    pub graphics: usize,
    pub physics: usize,
    pub actor: usize,
    pub scene: usize,
    pub script: usize,
    pub lod_normal: usize,
    pub lod_distant0: usize,
}

impl PartialEntityFlagCounts {
    pub fn flags(&self) -> PartialEntityFlags {
        PartialEntityFlags {
            graphics: self.graphics > 0,
            physics: self.physics > 0,
            actor: self.actor > 0,
            scene: self.scene > 0,
            script: self.script > 0,
            lod: if self.lod_normal > 0 {
                LodLevel::Normal
            } else if self.lod_distant0 > 0 {
                LodLevel::Distant0
            } else {
                // Does't matter
                LodLevel::Normal
            },
        }
    }
}

impl AddAssign<PartialEntityFlags> for PartialEntityFlagCounts {
    fn add_assign(&mut self, flags: PartialEntityFlags) {
        self.graphics += usize::from(flags.graphics);
        self.physics += usize::from(flags.physics);
        self.actor += usize::from(flags.actor);
        self.scene += usize::from(flags.scene);
        self.scene += usize::from(flags.script);
        self.lod_normal += usize::from(flags.lod == LodLevel::Normal);
        self.lod_distant0 += usize::from(flags.lod == LodLevel::Distant0);
    }
}

impl Add<PartialEntityFlags> for PartialEntityFlagCounts {
    type Output = Self;

    fn add(mut self, rhs: PartialEntityFlags) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<PartialEntityFlags> for PartialEntityFlagCounts {
    fn sub_assign(&mut self, flags: PartialEntityFlags) {
        self.graphics -= usize::from(flags.graphics);
        self.physics -= usize::from(flags.physics);
        self.actor -= usize::from(flags.actor);
        self.scene -= usize::from(flags.scene);
        self.scene -= usize::from(flags.script);
        self.lod_normal -= usize::from(flags.lod == LodLevel::Normal);
        self.lod_distant0 -= usize::from(flags.lod == LodLevel::Distant0);
    }
}

impl Sub<PartialEntityFlags> for PartialEntityFlagCounts {
    type Output = Self;

    fn sub(mut self, rhs: PartialEntityFlags) -> Self::Output {
        self -= rhs;
        self
    }
}
