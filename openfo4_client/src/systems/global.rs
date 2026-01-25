use hashbrown::HashMap;
use openfo4_core::record::RecordId;

/// Globals valid across the entire game state.
#[derive(Clone, Debug)]
pub struct GlobalProperties {
    /// Global variables / Globals / `GLOB`
    pub variables: HashMap<RecordId, f32>,
    pub difficulty: Difficulty,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Difficulty {
    VeryEasy,
    Easy,
    Normal,
    Hard,
    VeryHard,
    Survival,
}

impl Difficulty {
    pub const fn to_i32(&self) -> i32 {
        match self {
            Self::VeryEasy => 0,
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
            Self::VeryHard => 4,
            Self::Survival => 6,
        }
    }
}
