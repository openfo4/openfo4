use hashbrown::{HashMap, HashSet};
use indexmap::{IndexMap, IndexSet};
use openfo4_core::record::RecordId;

/// Globals valid across the entire game state.
#[derive(Clone, Debug)]
pub struct GlobalProperties {
    /// Global variables / Globals / `GLOB`
    pub variables: HashMap<RecordId, f32>,
    pub difficulty: Difficulty,
    pub form_lists: HashMap<RecordId, FormList>,
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

#[derive(Clone, Debug, Default)]
pub struct FormList {
    // Entries must be ordered.
    entries: IndexSet<RecordId>,
    added: HashSet<RecordId>,
}

impl FormList {
    pub fn new<T>(base: T) -> Self
    where
        T: IntoIterator<Item = RecordId>,
    {
        Self {
            entries: base.into_iter().collect(),
            added: HashSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, index: usize) -> Option<RecordId> {
        self.entries.get_index(index).copied()
    }

    pub fn index(&self, id: RecordId) -> Option<usize> {
        self.entries.get_index_of(&id)
    }

    pub fn add(&mut self, id: RecordId) {
        if self.added.contains(&id) {
            return;
        }

        self.entries.insert(id);
    }

    pub fn remove(&mut self, id: RecordId) {
        if !self.added.contains(&id) {
            return;
        }

        self.added.remove(&id);
        // Ensure we preserve order.
        self.entries.shift_remove(&id);
    }

    pub fn contains(&self, id: RecordId) -> bool {
        self.entries.contains(&id)
    }

    pub fn reset(&mut self) {
        for id in self.added.drain() {
            // Preserve order.
            self.entries.shift_remove(&id);
        }
    }
}
