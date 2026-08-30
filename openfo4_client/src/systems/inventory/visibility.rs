use hashbrown::{HashMap, HashSet};

use crate::systems::inventory::{InventoryKey, ItemVisibilityFlags};

/// Tracks which equipment items are visible.
#[derive(Clone, Debug, Default)]
pub struct VisibilityTracker {
    items: HashMap<InventoryKey, State>,
    /// The item that is currently visible at a specific slot.
    ///
    /// Only one item can be visible at a slot at the same time.
    visible: HashMap<u32, InventoryKey>,
    /// Items that have changed visible slots since the last rescan.
    visible_changed: HashSet<InventoryKey>,
}

impl VisibilityTracker {
    /// Adds a new equipped item.
    pub fn add<F>(&mut self, item: ItemVisibilityFlags, key: InventoryKey, mut change_visible: F)
    where
        F: FnMut(InventoryKey, &HashSet<u32>),
    {
        let mut conflicts = HashSet::new();
        for slot in &item.slots {
            let Some(other) = self.visible.get(slot).copied() else {
                continue;
            };

            let item = self.items.get_mut(&other).unwrap();

            conflicts.insert(other);
            item.conflicts.insert(key);
        }

        let has_no_slots = item.slots.is_empty();

        self.items.insert(
            key,
            State {
                item,
                visible: HashSet::new(),
                conflicts,
            },
        );

        // An item with no slots conflicts with no other items and is always visible.
        if has_no_slots {
            change_visible(key, &HashSet::new());
            return;
        }

        self.rescan_visiblity(key, false);

        for key in self.visible_changed.drain() {
            let item = self.items.get(&key).unwrap();
            change_visible(key, &item.visible);
        }
    }

    /// Removes a currently equipped item.
    pub fn remove<F>(&mut self, key: InventoryKey, mut change_visible: F)
    where
        F: FnMut(InventoryKey, &HashSet<u32>),
    {
        if !self.items.contains_key(&key) {
            return;
        }

        self.rescan_visiblity(key, true);
        self.visible_changed.insert(key);

        for key in self.visible_changed.drain() {
            let item = self.items.get(&key).unwrap();
            change_visible(key, &item.visible);
        }

        let state = self.items.remove(&key).unwrap();
        for other in state.conflicts {
            let other = self.items.get_mut(&other).unwrap();
            debug_assert!(other.conflicts.contains(&key));
            other.conflicts.remove(&key);
        }
    }

    fn rescan_visiblity(&mut self, key: InventoryKey, exclude_item: bool) {
        let root = key;
        let item = self.items.get_mut(&root).unwrap();

        for slot in item.visible.drain() {
            self.visible.remove(&slot);
        }

        let mut queue = Vec::new();
        queue.extend(&item.conflicts);
        if !exclude_item {
            queue.push(key);
        }

        let mut visited = HashSet::new();
        while let Some(key) = queue.pop() {
            if !visited.insert(key) {
                continue;
            }

            // Temporarily remove the item from the map so we can borrow other
            // items mutably.
            // We will place it back after the iteration.
            let mut item = self.items.remove(&key).unwrap();

            for slot in &item.item.slots {
                let Some(other) = self.visible.get(slot).copied() else {
                    self.visible.insert(*slot, key);
                    item.visible.insert(*slot);
                    self.visible_changed.insert(key);
                    continue;
                };

                if other == key {
                    continue;
                }

                let other_item = self.items.get_mut(&other).unwrap();

                debug_assert!(item.conflicts.contains(&other));
                debug_assert!(other_item.conflicts.contains(&key));

                // The excluded item has no visible slots set and is excluded from
                // conflicts. We should never see it here.
                if exclude_item {
                    debug_assert_ne!(key, root);
                    debug_assert_ne!(other, root);
                }

                // The item with the higher priority wins.
                if other_item.item.priority >= item.item.priority {
                    continue;
                }

                other_item.visible.remove(slot);
                item.visible.insert(*slot);
                self.visible.insert(*slot, key);

                // We only emit one `change_visible` call for each item,
                // but it is possible that more than one slot conflicts.
                self.visible_changed.insert(other);
                self.visible_changed.insert(key);
            }

            for other in &item.conflicts {
                if exclude_item && *other == root {
                    continue;
                }

                queue.push(*other);
            }

            self.items.insert(key, item);
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    item: ItemVisibilityFlags,
    /// The slots of this item that are currently visible.
    visible: HashSet<u32>,
    /// Other items that the visibility slots conflict with.
    conflicts: HashSet<InventoryKey>,
}

#[cfg(test)]
mod tests {
    use hashbrown::{HashMap, HashSet};

    use crate::systems::inventory::visibility::VisibilityTracker;
    use crate::systems::inventory::{InventoryKey, ItemVisibilityFlags};

    fn expect(expect: &[(InventoryKey, HashSet<u32>)]) -> impl FnMut(InventoryKey, &HashSet<u32>) {
        let expect: HashMap<InventoryKey, HashSet<u32>> = expect.to_vec().into_iter().collect();
        move |key, slots| {
            assert_eq!(slots, expect.get(&key).unwrap());
        }
    }

    #[test]
    fn visibility_tracker_add_remove() {
        let mut tracker = VisibilityTracker::default();
        tracker.add(
            ItemVisibilityFlags {
                slots: [0, 1].into(),
                priority: 0,
            },
            InventoryKey(0),
            |key, slots| {
                assert_eq!(key, InventoryKey(0));
                assert_eq!(slots, &HashSet::from([0, 1]));
            },
        );

        tracker.remove(InventoryKey(0), |key, slots| {
            assert_eq!(key, InventoryKey(0));
            assert_eq!(slots, &HashSet::from([]));
        });
    }

    #[test]
    fn visibility_tracker_add_overlapping() {
        let mut tracker = VisibilityTracker::default();

        tracker.add(
            ItemVisibilityFlags {
                slots: [0, 1].into(),
                priority: 0,
            },
            InventoryKey(0),
            |key, slots| {
                assert_eq!(key, InventoryKey(0));
                assert_eq!(slots, &HashSet::from([0, 1]))
            },
        );
        tracker.add(
            ItemVisibilityFlags {
                slots: [2, 3].into(),
                priority: 0,
            },
            InventoryKey(1),
            |key, slots| {
                assert_eq!(key, InventoryKey(1));
                assert_eq!(slots, &HashSet::from([2, 3]));
            },
        );

        tracker.add(
            ItemVisibilityFlags {
                slots: [1, 2].into(),
                priority: 1,
            },
            InventoryKey(2),
            expect(&[
                (InventoryKey(0), HashSet::from([0])),
                (InventoryKey(1), HashSet::from([3])),
                (InventoryKey(2), HashSet::from([1, 2])),
            ]),
        );

        tracker.remove(
            InventoryKey(2),
            expect(&[
                (InventoryKey(0), HashSet::from([0, 1])),
                (InventoryKey(1), HashSet::from([2, 3])),
                (InventoryKey(2), HashSet::from([])),
            ]),
        );
    }

    #[test]
    fn visibility_tracker_empty_slots() {
        let mut tracker = VisibilityTracker::default();

        tracker.add(
            ItemVisibilityFlags {
                slots: [].into(),
                priority: 1,
            },
            InventoryKey(0),
            |key, slots| {
                assert_eq!(key, InventoryKey(0));
                assert_eq!(slots, &HashSet::from([]))
            },
        );

        tracker.remove(InventoryKey(0), |key, slots| {
            assert_eq!(key, InventoryKey(0));
            assert_eq!(slots, &HashSet::from([]))
        });
    }
}
