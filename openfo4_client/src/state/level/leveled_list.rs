use openfo4_esp::records::{LeveledActorFlags, LeveledListEntry};
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;

pub fn select_from_leveled_list(
    flags: LeveledActorFlags,
    entries: &[LeveledListEntry],
    player_level: u16,
) -> Option<usize> {
    let mut weights = Vec::new();
    let mut indices = Vec::new();

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
        return None;
    }

    let distribution = WeightedIndex::new(weights).unwrap();
    let index = indices[distribution.sample(&mut rand::rng())];
    Some(index)
}
