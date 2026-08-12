use std::iter::FusedIterator;

use hashbrown::{HashMap, HashSet};
use indexmap::IndexSet;
use openfo4_core::record::RecordId;
use parking_lot::Mutex;

/// Globals valid across the entire game state.
#[derive(Debug)]
pub struct GlobalProperties {
    pub game_rules: GameRules,
    /// Global variables / Globals / `GLOB`
    pub variables: Mutex<HashMap<RecordId, f32>>,
    pub form_lists: Mutex<HashMap<RecordId, FormList>>,
    pub factions: Mutex<HashMap<RecordId, Faction>>,
    pub mods: RecordModifications,
}

/// Set of game rules.
#[derive(Clone, Debug)]
pub struct GameRules {
    pub difficulty: Difficulty,
    /// Whether the player is allowed to fast travel.
    pub fast_travel_enabled: bool,
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

    pub fn iter(&self) -> FormListIter<'_> {
        FormListIter {
            inner: self.entries.iter(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FormListIter<'a> {
    inner: indexmap::set::Iter<'a, RecordId>,
}

impl<'a> Iterator for FormListIter<'a> {
    type Item = RecordId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for FormListIter<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> FusedIterator for FormListIter<'a> {}

#[derive(Clone, Debug)]
pub struct Faction {
    pub relations: HashMap<RecordId, FactionReaction>,
    /// Whether the player is considered an temporary enemy of this faction.
    pub player_enemy: bool,
    /// Whether the player is considered expelled from this faction.
    pub player_expelled: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FactionReaction {
    Neutral,
    Ally,
    Friend,
    Enemy,
}

#[derive(Debug, Default)]
pub struct RecordModifications {
    /// Dynamically added leveled list additions.
    pub leveled_list: Mutex<HashMap<RecordId, Vec<LevelListAddition>>>,
}

#[derive(Copy, Clone, Debug)]
pub struct LevelListAddition {
    pub item: RecordId,
    pub level: u16,
    pub count: u32,
}
