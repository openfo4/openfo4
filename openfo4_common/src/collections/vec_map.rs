use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;

pub trait Index: Copy + Eq {
    fn index(&self) -> usize;
}

impl Index for super::arena::Key {
    fn index(&self) -> usize {
        self.index()
    }
}

impl Index for usize {
    fn index(&self) -> usize {
        *self
    }
}

impl Index for u32 {
    fn index(&self) -> usize {
        *self as usize
    }
}

/// A key-value map backed by a [`Vec`].
///
/// This is more efficient alternative to a [`HashMap`] when the keys follow a linear pattern.
#[derive(Clone, Debug)]
pub struct VecMap<K, V> {
    inner: Vec<Option<(K, V)>>,
    len: usize,
}

impl<K, V> VecMap<K, V> {
    /// Creates a new, empty `VecMap`.
    pub const fn new() -> Self {
        Self {
            inner: Vec::new(),
            len: 0,
        }
    }
}

impl<K, V> VecMap<K, V>
where
    K: Index,
{
    /// Inserts a new value into this `VecMap`. Returns the previous value at this `key` or `None`
    /// if no value at the key exists.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let min_cap = key.index() + 1;
        if self.inner.len() < min_cap {
            self.inner.resize_with(min_cap, || None);
        }

        let slot = &mut self.inner[key.index()];

        let old_value = slot.take();
        *slot = Some((key, value));

        // Don't increment len if we swapped a value.
        if old_value.is_none() {
            self.len += 1;
        }

        old_value.map(|(_, v)| v)
    }

    /// Returns the number of elements in the `VecMap`.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the `VecMap` contains no elements.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the `VecMap` contains a value for the given `key`.
    pub fn contains_key(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    pub fn get(&self, key: K) -> Option<&V> {
        match self.inner.get(key.index()) {
            Some(Some((k, v))) if *k == key => Some(v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        match self.inner.get_mut(key.index()) {
            Some(Some((k, v))) if *k == key => Some(v),
            _ => None,
        }
    }

    unsafe fn get_unchecked_by_index(&self, index: usize) -> &V {
        // SAFETY: The caller guarantees the index is in bounds.
        let slot = unsafe { self.inner.get_unchecked(index) };

        match slot {
            Some((_, v)) => v,
            // SAFETY: The caller guarantees that the key exists.
            None => unsafe { unreachable_unchecked() },
        }
    }

    unsafe fn get_unchecked_mut_by_index(&mut self, index: usize) -> &mut V {
        // SAFETY: The caller guarantees the index is in bounds.
        let slot = unsafe { self.inner.get_unchecked_mut(index) };

        match slot {
            Some((_, v)) => v,
            // SAFETY: The caller guarantees that the key exists.
            None => unsafe { unreachable_unchecked() },
        }
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let slot = self.inner.get_mut(key.index())?;
        let (_, value) = slot.take_if(|(k, _)| *k == key)?;

        self.len -= 1;
        Some(value)
    }

    /// Returns an iterator visiting all elements in the `VecMap`.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            iter: self.inner.iter(),
            len: self.len,
        }
    }

    /// Returns an iterator visiting all elements in the `VecMap` mutably.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            iter: self.inner.iter_mut(),
            len: self.len,
        }
    }

    /// Returns an iterator visiting all values in the `VecMap`.
    pub fn values(&self) -> Values<'_, K, V> {
        Values { iter: self.iter() }
    }

    /// Returns an iterator visiting all values in the `VecMap` mutably.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            iter: self.iter_mut(),
        }
    }

    /// Returns an iterator yielding all values in the `VecMap`.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues {
            iter: self.into_iter(),
        }
    }

    /// Removes all elements from the `VecMap`.
    pub fn clear(&mut self) {
        self.inner.clear();
        self.len = 0;
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let index = key.index();

        // Note that the `Index` trait is not unsafe and we can't rely on `key.index()`
        // to always return the same index.
        // For this reason we store the fix index value and use that for all future indexing
        // operations.
        if self.contains_key(key) && self.inner.get(index).is_some_and(|v| v.is_some()) {
            Entry::Occupied(OccupiedEntry {
                map: self,
                key,
                index,
            })
        } else {
            Entry::Vacant(VacantEntry { map: self, key })
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs (k, v) for which f(&k, &mut v) returns false.
    /// The elements are visited in unsorted (and unspecified) order.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        for slot in &mut self.inner {
            if let Some((key, value)) = slot
                && !f(key, value)
            {
                *slot = None
            }
        }
    }
}

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K, V> IntoIterator for &'a VecMap<K, V>
where
    K: Index,
{
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut VecMap<K, V>
where
    K: Index,
{
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.inner.into_iter(),
            len: self.len,
        }
    }
}

#[derive(Debug)]
pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: Index,
{
    pub fn or_insert(self, default: V) -> &'a mut V {
        self.or_insert_with(|| default)
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(Default::default)
    }
}

#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V> {
    map: &'a mut VecMap<K, V>,
    key: K,
    index: usize,
}

impl<'a, K, V> OccupiedEntry<'a, K, V>
where
    K: Index,
{
    /// Returns the key in the entry.
    pub fn key(&self) -> K {
        self.key
    }

    /// Returns a reference to the value in the entry.
    pub fn get(&self) -> &V {
        // SAFETY: We hold a unique reference to the map. The key
        // cannot be removed from the map until we are dropped.
        unsafe { self.map.get_unchecked_by_index(self.index) }
    }

    /// Returns a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        // SAFETY: We hold a unique reference to the map. The key
        // cannot be removed from the map until we are dropped.
        unsafe { self.map.get_unchecked_mut_by_index(self.index) }
    }

    pub fn into_mut(self) -> &'a mut V {
        // SAFETY: We hold a unique reference to the map. The key
        // cannot be removed from the map until we are dropped.
        unsafe { self.map.get_unchecked_mut_by_index(self.index) }
    }

    /// Sets the value of the entry, returning the current value.
    pub fn insert(&mut self, value: V) -> V {
        self.map.insert(self.key, value).unwrap()
    }

    /// Remove the entry from the map and returns the current value.
    pub fn remove(self) -> V {
        self.map.remove(self.key).unwrap()
    }

    /// Remove the entry from the map and returns the current key and value.
    pub fn remove_entry(self) -> (K, V) {
        let value = self.map.remove(self.key).unwrap();
        (self.key, value)
    }
}

#[derive(Debug)]
pub struct VacantEntry<'a, K, V> {
    map: &'a mut VecMap<K, V>,
    key: K,
}

impl<'a, K, V> VacantEntry<'a, K, V>
where
    K: Index,
{
    /// Returns the key in the entry.
    pub fn key(&self) -> K {
        self.key
    }

    /// Sets the value of entry.
    pub fn insert(self, value: V) -> &'a mut V {
        self.map.insert(self.key, value);
        self.map.get_mut(self.key).unwrap()
    }
}

/// An iterator visiting all elements inside a [`VecMap`].
///
/// Returned by [`iter`].
///
/// [`iter`]: VecMap::iter
#[derive(Clone, Debug)]
pub struct Iter<'a, K, V> {
    iter: std::slice::Iter<'a, Option<(K, V)>>,
    len: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Index,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slot = self.iter.next()?;

            if let Some((key, value)) = slot {
                self.len -= 1;
                return Some((*key, value));
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.len
    }
}

/// An iterator visiting all elements inside a [`VecMap`] mutably.
///
/// Returned by [`iter_mut`].
///
/// [`iter_mut`]: VecMap::iter_mut
#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    iter: std::slice::IterMut<'a, Option<(K, V)>>,
    len: usize,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: Index,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slot = self.iter.next()?;

            if let Some((key, value)) = slot {
                self.len -= 1;
                return Some((*key, value));
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, K, V> FusedIterator for IterMut<'a, K, V> where K: Index {}

/// An iterator yielding all elements inside a [`VecMap`].
///
/// Returned by [`into_iter`].
///
/// [`into_iter`]: VecMap::into_iter
#[derive(Clone, Debug)]
pub struct IntoIter<K, V> {
    iter: std::vec::IntoIter<Option<(K, V)>>,
    len: usize,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slot = self.iter.next()?;

            if let Some((key, value)) = slot {
                self.len -= 1;
                return Some((key, value));
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

/// An iterator visiting all values inside a [`VecMap`].
///
/// Returned by [`values`].
///
/// [`values`]: VecMap::values
#[derive(Clone, Debug)]
pub struct Values<'a, K, V>
where
    K: Index,
{
    iter: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: Index,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K, V> ExactSizeIterator for Values<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> FusedIterator for Values<'a, K, V> where K: Index {}

/// An iterator visiting all values inside a [`VecMap`] mutably.
///
/// Returned by [`values_mut`].
///
/// [`values_mut`]: VecMap::values_mut
#[derive(Debug)]
pub struct ValuesMut<'a, K, V>
where
    K: Index,
{
    iter: IterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V>
where
    K: Index,
{
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V>
where
    K: Index,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> FusedIterator for ValuesMut<'a, K, V> where K: Index {}

/// An iterator yielding all values inside a [`VecMap`].
///
/// Returned by [`into_values`].
///
/// [`into_values`]: VecMap::into_values
#[derive(Clone, Debug)]
pub struct IntoValues<K, V> {
    iter: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, value)| value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IntoValues<K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<K, V> FusedIterator for IntoValues<K, V> {}

impl<K, V> FromIterator<(K, V)> for VecMap<K, V>
where
    K: Index,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        let mut map = Self::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::vec_map::VecMap;

    #[test]
    fn vec_map() {
        let mut map = VecMap::<usize, i32>::new();

        map.insert(0_usize, 42_i32);
        assert_eq!(*map.get(0_usize).unwrap(), 42_i32);
        map.insert(42_usize, 21_i32);
        assert_eq!(*map.get(42_usize).unwrap(), 21_i32);

        assert_eq!(map.get(41), None);
        assert_eq!(map.get(43), None);

        assert_eq!(map.remove(42).unwrap(), 21);
        assert_eq!(map.get(42), None);

        map.insert(42, 22);
        assert_eq!(*map.get(42).unwrap(), 22);
    }

    #[test]
    fn vec_map_retain() {
        let mut map = VecMap::<usize, ()>::new();

        map.insert(2, ());
        map.insert(4, ());
        map.insert(5, ());
        map.insert(7, ());
        map.insert(8, ());

        map.retain(|key, _| *key % 2 == 0);

        assert!(map.contains_key(2));
        assert!(map.contains_key(4));
        assert!(map.contains_key(8));
        assert!(!map.contains_key(5));
        assert!(!map.contains_key(7));
    }
}
