use std::iter::FusedIterator;
use std::time::Duration;

use hashbrown::{HashMap, HashSet};
use indexmap::IndexSet;
use openfo4_core::record::RecordId;
use openfo4_esp::records::TypedValue;
use openfo4_esp::{RecordData, RecordKind};
use parking_lot::Mutex;

use crate::state::esp::{self, EspLoader};

/// Globals valid across the entire game state.
#[derive(Debug)]
pub struct GlobalProperties {
    pub game_rules: GameRules,
    /// Global variables / Globals / `GLOB`
    pub variables: Mutex<HashMap<RecordId, f32>>,
    pub form_lists: Mutex<HashMap<RecordId, FormList>>,
    pub factions: Mutex<HashMap<RecordId, Faction>>,
    pub mods: RecordModifications,
    pub playtime: PlayTime,
    pub settings: GameSettings,
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

#[derive(Debug, Default)]
pub struct PlayTime(Mutex<Duration>);

impl PlayTime {
    pub fn get(&self) -> Duration {
        *self.0.lock()
    }

    /// Increases the time by the given [`Duration`].
    pub fn increase(&self, delta: Duration) {
        let mut dur = self.0.lock();
        *dur = dur.saturating_add(delta);
    }
}

#[derive(Debug)]
pub struct GameSettings {
    values: Mutex<HashMap<RecordId, GameSettingValue>>,
}

impl GameSettings {
    pub async fn load(loader: &EspLoader) -> Result<Self, esp::Error> {
        let mut values = HashMap::new();
        for record in loader.lookup_all(Some(RecordKind::GameSetting)).await? {
            let RecordData::GameSetting(data) = record.local.data else {
                continue;
            };

            let value = match data.data {
                TypedValue::Int(v) => GameSettingValue::I32(v),
                TypedValue::Float(v) => GameSettingValue::F32(v),
                // TODO
                _ => continue,
            };

            values.insert(record.id, value);
        }

        Ok(Self {
            values: Mutex::new(values),
        })
    }

    pub fn get(&self, id: RecordId) -> Option<GameSettingValue> {
        self.values.lock().get(&id).copied()
    }

    pub fn set(&self, id: RecordId, value: GameSettingValue) {
        // Don't insert the value if the key does not exist.
        self.values.lock().entry(id).and_modify(|v| {
            *v = value;
        });
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GameSettingValue {
    I32(i32),
    F32(f32),
}

impl GameSettingValue {
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(v) => Some(*v),
            _ => None,
        }
    }
}
