use std::collections::{HashMap, HashSet, VecDeque};

use game_common::collections::arena::{self, Arena};
use game_common::collections::vec_map::VecMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(arena::Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity(Key);

#[derive(Clone, Debug)]
pub struct Hierarchy<T> {
    nodes: Arena<T>,
    children: HashMap<Key, Vec<Key>>,
    parents: VecMap<arena::Key, arena::Key>,
    root: HashSet<Key>,
    /// Hint that the nodes are currently topologically sorted in memory, i.e. the parent comes
    /// before all children is true for all nodes.
    ///
    /// This hint can improve performance of some operations, but note that this hint is
    /// conservative and may be set to false, even if the nodes are sorted.
    is_sorted: bool,
}

impl<T> Hierarchy<T> {
    pub fn new() -> Self {
        Self {
            nodes: Arena::new(),
            children: HashMap::new(),
            parents: VecMap::new(),
            root: HashSet::new(),
            is_sorted: true,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn append(&mut self, parent: Option<Key>, node: T) -> Key {
        let key = Key(self.nodes.insert(node));

        if let Some(parent) = parent {
            debug_assert!(self.nodes.contains_key(parent.0));

            self.parents.insert(key.0, parent.0);
            self.children.entry(parent).or_default().push(key);
        } else {
            self.root.insert(key);
        }

        key
    }

    pub fn remove(&mut self, key: Key) {
        self.nodes.remove(key.0);
        self.root.remove(&key);

        if let Some(parent) = self.parents.remove(key.0) {
            if let Some(children) = self.children.get_mut(&Key(parent)) {
                children.retain(|id| *id != key);
            }
        }

        if let Some(children) = self.children.remove(&key) {
            for c in children {
                self.remove(c);
            }
        }

        self.is_sorted = false;
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        self.nodes.get(key.0)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        self.nodes.get_mut(key.0)
    }

    /// Removes all entities.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.children.clear();
        self.parents.clear();
        self.is_sorted = true;
    }

    pub fn contains_key(&self, key: Key) -> bool {
        self.nodes.contains_key(key.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &T)> + '_ {
        self.nodes.iter().map(|(k, v)| (Key(k), v))
    }

    /// Returns an [`Iterator`] over all nodes in this `Hierarchy` that yields all nodes
    /// topologically sorted, i.e. such that parents are always yielded before their respective
    /// children.
    pub fn iter_topo_sorted(&self) -> IterTopoSorted<'_, T> {
        // If the nodes are sorted in memory they are already topologically sorted.
        if self.is_sorted {
            IterTopoSorted {
                inner: IterTopoSortedInner::Fast(self.nodes.iter()),
            }
        } else {
            IterTopoSorted {
                inner: IterTopoSortedInner::Slow {
                    hierarchy: self,
                    len: self.nodes.len(),
                    queue: self.root.iter().copied().collect(),
                },
            }
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> + '_ {
        self.nodes.values()
    }

    pub fn parent_key(&self, key: Key) -> Option<Key> {
        self.parents.get(key.0).map(|parent| Key(*parent))
    }

    pub fn parent(&self, key: Key) -> Option<&T> {
        let parent = self.parents.get(key.0)?;
        Some(self.nodes.get(*parent).unwrap())
    }

    pub fn children(&self, parent: Key) -> Option<impl Iterator<Item = (Key, &T)> + '_> {
        let children = self.children.get(&parent)?;
        Some(children.iter().map(|key| {
            let node = self.nodes.get(key.0).unwrap();
            (*key, node)
        }))
    }

    pub fn root_keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.root.iter().copied()
    }

    /// Converts a `Hierarchy<T>` into a `Hierarchy<U>`, retaining the existing hierarchy.
    pub fn convert<U, F>(&self, mut f: F) -> Hierarchy<U>
    where
        F: FnMut(&T) -> U,
    {
        // FIXME: Actually we only need to create a new arena in the same
        // state, apply `F` on all elements and then reuse the existing keys
        // as `self.nodes`, but `SlotMap` doesn't allow us to do these things.
        // We can do this only we have our own `Arena` type.
        // For now we must manually recreate the parents/children maps.
        let mut nodes = Arena::with_capacity(self.nodes.len());
        let mut parents = VecMap::new();
        let mut children = HashMap::with_capacity(self.children.len());
        let mut root = HashSet::with_capacity(self.root.len());

        let mut old_to_new_keys = HashMap::new();

        for (old_key, node) in &self.nodes {
            let new_key = nodes.insert(f(node));
            old_to_new_keys.insert(old_key, new_key);
        }

        for (old_key, old_parent) in self.parents.iter() {
            let new_key = old_to_new_keys.get(&old_key).unwrap();
            let new_parent = old_to_new_keys.get(&old_parent).unwrap();
            parents.insert(*new_key, *new_parent);
        }

        for (old_key, old_children) in &self.children {
            let new_key = old_to_new_keys.get(&old_key.0).unwrap();
            let new_children = old_children
                .iter()
                .map(|k| Key(*old_to_new_keys.get(&k.0).unwrap()))
                .collect();

            children.insert(Key(*new_key), new_children);
        }

        for old_key in &self.root {
            let new_key = *old_to_new_keys.get(&old_key.0).unwrap();
            root.insert(Key(new_key));
        }

        Hierarchy {
            nodes,
            children,
            parents,
            root,
            is_sorted: self.is_sorted,
        }
    }
}

impl<T> Default for Hierarchy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for Hierarchy<T> {
    fn from(value: T) -> Self {
        let mut hierarchy = Self::new();
        hierarchy.append(None, value);
        hierarchy
    }
}

/// An [`Iterator`] that yields all nodes of a [`Hierarchy`] topologically sorted.
///
/// Returned by [`iter_topo_sorted`].
///
/// [`iter_topo_sorted`]: Hierarchy::iter_topo_sorted
#[derive(Clone, Debug)]
pub struct IterTopoSorted<'a, T> {
    inner: IterTopoSortedInner<'a, T>,
}

impl<'a, T> Iterator for IterTopoSorted<'a, T> {
    type Item = (Key, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            IterTopoSortedInner::Fast(iter) => iter.next().map(|(k, v)| (Key(k), v)),
            IterTopoSortedInner::Slow {
                hierarchy,
                len,
                queue,
            } => {
                let key = queue.pop_front()?;

                if let Some(children) = hierarchy.children.get(&key) {
                    queue.extend(children);
                }

                *len -= 1;
                Some((key, hierarchy.nodes.get(key.0).unwrap()))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, T> ExactSizeIterator for IterTopoSorted<'a, T> {
    fn len(&self) -> usize {
        match &self.inner {
            IterTopoSortedInner::Fast(iter) => iter.len(),
            IterTopoSortedInner::Slow { len, .. } => *len,
        }
    }
}

#[derive(Clone, Debug)]
enum IterTopoSortedInner<'a, T> {
    Fast(arena::Iter<'a, T>),
    Slow {
        hierarchy: &'a Hierarchy<T>,
        len: usize,
        queue: VecDeque<Key>,
    },
}
