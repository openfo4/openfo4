use openfo4_esp::records::{LeveledActorFlags, LeveledListEntry};
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;

pub fn select_from_leveled_list(
    flags: LeveledActorFlags,
    entries: &[LeveledListEntry],
    count: u64,
    player_level: u16,
) -> Vec<SelectedLeveledEntry> {
    let mut weights = Vec::new();
    let mut indices = Vec::new();

    if flags.contains(LeveledActorFlags::USE_ALL) {
        return (0..entries.len())
            .map(|index| SelectedLeveledEntry { index, count })
            .collect();
    }

    if flags.contains(LeveledActorFlags::ALL_LEVELS) {
        for (index, entry) in entries.iter().enumerate() {
            if entry.level > player_level {
                continue;
            }

            let weight = 100 - u8::min(100, entry.chance_none);
            if weight == 0 {
                continue;
            }

            indices.push(index);
            weights.push(u64::from(weight));
        }
    } else {
        let mut max_level = 0;

        for (index, entry) in entries.iter().enumerate() {
            if entry.level > player_level || entry.level < max_level {
                continue;
            }

            // We found an entry with a higher level.
            // All previous entries become invalid.
            if entry.level != max_level {
                weights.clear();
                indices.clear();
                max_level = entry.level;
            }

            let weight = 100 - u8::min(100, entry.chance_none);
            if weight == 0 {
                continue;
            }

            indices.push(index);
            weights.push(u64::from(weight));
        }
    }

    if weights.len() == 0 {
        return Vec::new();
    }

    let distribution = WeightedIndex::new(weights).unwrap();

    let mut entries = Vec::new();
    if flags.contains(LeveledActorFlags::EACH) {
        for _ in 0..count {
            let index = indices[distribution.sample(&mut rand::rng())];
            entries.push(SelectedLeveledEntry { index, count: 1 });
        }
    } else {
        let index = indices[distribution.sample(&mut rand::rng())];
        entries.push(SelectedLeveledEntry { index, count });
    }

    entries
}

#[derive(Copy, Clone, Debug)]
pub struct SelectedLeveledEntry {
    pub index: usize,
    pub count: u64,
}
