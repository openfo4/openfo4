use core::slice;
use std::fmt::Debug;
use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};

use allocator_api2::alloc::{Allocator, Global};
use allocator_api2::vec::Vec;

use crate::utils::vec_ext::VecExt;

#[derive(Clone, Debug)]
pub struct Slab<K, V, A = Global>
where
    A: Allocator,
{
    buckets: Vec<Bucket<V>, A>,
    len: usize,
    next_free: usize,
    _marker: PhantomData<K>,
}

impl<K, V> Slab<K, V> {
    /// Creates a new empty `Slab`.
    pub fn new() -> Self {
        Self {
            buckets: Vec::new(),
            len: 0,
            next_free: 0,
            _marker: PhantomData,
        }
    }

    /// Returns the total number of items this `Slab` can store wihout reallocating.
    pub fn capacity(&self) -> usize {
        self.buckets.capacity()
    }

    /// Returns the number of items in this `Slab`.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if this `Slab` contains no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reserve capacity for at least `additional` more values to be inserted without allocating.
    ///
    /// Does nothing if capacity is already sufficient.
    pub fn reserve(&mut self, additional: usize) {
        let free_cap = self.buckets.capacity() - self.len;
        if free_cap < additional {
            self.buckets.reserve(additional - free_cap);
        }
    }
}

impl<K, V> Slab<K, V>
where
    K: SlabKey,
{
    /// Returns `true` if a value with the give `key` exists in the `Slab`.
    pub fn contains(&self, key: K) -> bool {
        self.buckets
            .get(key.to_index())
            .is_some_and(|bucket| matches!(bucket, Bucket::Occupied(_)))
    }

    /// Returns a reference to the value at the given `key`.
    #[inline]
    pub fn get(&self, key: K) -> Option<&V> {
        match self.buckets.get(key.to_index()) {
            Some(Bucket::Occupied(value)) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value at the given `key`.
    #[inline]
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        match self.buckets.get_mut(key.to_index()) {
            Some(Bucket::Occupied(value)) => Some(value),
            _ => None,
        }
    }

    /// Returns a reference to the value at the given `key` without checking if it exists.
    ///
    /// # Safety
    ///
    /// The given `key` must currently exist in the `Slab`.
    #[inline]
    pub unsafe fn get_unchecked(&self, key: K) -> &V {
        // SAFEY: The caller guarantees that the index is valid the bucket is occupied.
        unsafe {
            self.buckets
                .get_unchecked(key.to_index())
                .assume_occupied_ref()
        }
    }

    /// Returns a mutable reference to the value at the given `key` without checking if it exists.
    ///
    /// # Safety
    ///
    /// The given `key` must currently exist in the `Slab`.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, key: K) -> &mut V {
        // SAFEY: The caller guarantees that the index is valid the bucket is occupied.
        unsafe {
            self.buckets
                .get_unchecked_mut(key.to_index())
                .assume_occupied_mut()
        }
    }

    /// Inserts a new value into the [`Slab`] and returns the key to access it.
    pub fn insert(&mut self, value: V) -> K {
        self.try_insert(value).expect("Slab is full")
    }

    pub fn try_insert(&mut self, value: V) -> Option<K> {
        let index = self.next_free;
        let key = K::from_index(index)?;

        if index == self.buckets.len() {
            // We must not wrap around here, otherwise next_free would point
            // to a bucket that is occupied.
            self.next_free = index.checked_add(1)?;

            self.buckets.push(Bucket::Occupied(value));
        } else {
            // SAFETY: The next free index always points to a bucket in bounds.
            let bucket = unsafe { self.buckets.get_unchecked_mut(index) };
            debug_assert!(matches!(bucket, Bucket::Free(_)));

            let bucket = mem::replace(bucket, Bucket::Occupied(value));
            // SAFETY: The next free index always points to a free bucket.
            self.next_free = unsafe { bucket.assume_free() };
        }

        self.len += 1;

        Some(key)
    }

    /// Inserts a new value into the [`Slab`] assuming that there is free capacity for the new
    /// item.
    ///
    /// # Safety
    ///
    /// The spare capacity must be at least 1.
    ///
    pub unsafe fn insert_within_capacity_unchecked(&mut self, value: V) -> K {
        debug_assert!(self.capacity() > self.len());

        let index = self.next_free;
        let key = K::from_index_unchecked(index);

        if index == self.buckets.len() {
            self.next_free = index + 1;

            // SAFETY: The caller guarantees that there is spare capacity for a push.
            unsafe {
                self.buckets.push_unchecked(Bucket::Occupied(value));
            }
        } else {
            // SAFETY: The next free index always points to a bucket in bounds.
            let bucket = unsafe { self.buckets.get_unchecked_mut(index) };
            debug_assert!(matches!(bucket, Bucket::Free(_)));

            let bucket = mem::replace(bucket, Bucket::Occupied(value));
            // SAFETY: The next free index always points to a free bucket.
            self.next_free = unsafe { bucket.assume_free() };
        }

        self.len += 1;

        key
    }

    /// Remove the value at the given `key`.
    pub fn remove(&mut self, key: K) -> Option<V> {
        let index = key.to_index();

        match self.buckets.get_mut(index) {
            Some(bucket) if matches!(bucket, Bucket::Occupied(_)) => {
                let bucket = mem::replace(bucket, Bucket::Free(self.next_free));
                self.next_free = index;
                self.len -= 1;

                // SAFETY: We just checked that the slot is occupied.
                Some(unsafe { bucket.assume_occupied() })
            }
            _ => None,
        }
    }

    /// Removes the entry with the given `key` from the `Slab` without checking if it exists.
    ///
    /// # Safety
    ///
    /// A entry with the given `key` must currently exist in the `Slab`.
    pub unsafe fn remove_unchecked(&mut self, key: K) -> V {
        let index = key.to_index();

        // SAFETY: The caller guarantees that the given index is in bounds.
        let bucket = unsafe { self.buckets.get_unchecked_mut(index) };

        let bucket = mem::replace(bucket, Bucket::Free(self.next_free));
        self.next_free = index;
        self.len -= 1;

        // SAFETY: The caller guarantees that the bucket is occupied.
        unsafe { bucket.assume_occupied() }
    }

    /// Returns an iterator over all the items in this `Slab`.
    ///
    /// The order of returned items is not specified.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            buckets: self.buckets.iter(),
            index: 0,
            len: self.len,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over all the items in this `Slab`.
    ///
    /// The order of returned items is not specified.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            buckets: self.buckets.iter_mut(),
            index: 0,
            len: self.len,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over all the values in this `Slab`.
    ///
    /// The order of returned items is not specified.
    pub fn values(&self) -> Values<'_, K, V> {
        Values { iter: self.iter() }
    }
}

impl<'a, K, V> IntoIterator for &'a Slab<K, V>
where
    K: SlabKey,
{
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut Slab<K, V>
where
    K: SlabKey,
{
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V> Index<K> for Slab<K, V>
where
    K: SlabKey,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).expect("key does not exist")
    }
}

impl<K, V> IndexMut<K> for Slab<K, V>
where
    K: SlabKey,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(index).expect("key does not exist")
    }
}

/// A type that can be used as a key for a [`Slab`].
///
/// Implementations must satisfy the following guarantees:
///
/// - A key constructed from an index via [`from_index`] or [`from_index_unchecked`] must convert
///   back the same index via [`to_index`].
/// - A key that can be constructed via [`from_index`] must also be valid to construct via
///   [`from_index_unchecked`].
/// - If a key can be constructed from index A, another index B with `B < A` must also be
///   constructible.
///
/// [`from_index`]: Self::from_index
/// [`from_index_unchecked`]: Self::from_index_unchecked
/// [`to_index`]: Self::to_index
pub trait SlabKey: Copy + PartialEq + Eq {
    /// Creates a key from an index. Returns `None` if the index is too large.
    fn from_index(index: usize) -> Option<Self>;

    /// Creates a key from an index, assuming it is valid.
    ///
    /// An index is valid exactly if [`from_index`] returns `Some(..)`.
    ///
    /// [`from_index`]: Self::from_index
    fn from_index_unchecked(index: usize) -> Self;

    /// Extracts the index of this key.
    ///
    /// This returns the same index used to construct this key.
    fn to_index(&self) -> usize;
}

impl SlabKey for u32 {
    fn from_index(index: usize) -> Option<Self> {
        index.try_into().ok()
    }

    fn from_index_unchecked(index: usize) -> Self {
        index as u32
    }

    fn to_index(&self) -> usize {
        *self as usize
    }
}

impl SlabKey for usize {
    fn from_index(index: usize) -> Option<Self> {
        Some(index)
    }

    fn from_index_unchecked(index: usize) -> Self {
        index
    }

    #[inline]
    fn to_index(&self) -> usize {
        *self
    }
}

/// Defines a custom [`SlabKey`] type.
///
/// # Examples
///
/// ```
/// use game_common::collections::slab::{define_slab_key, Slab};
///
/// define_slab_key! {
///     struct MyKey: usize;
/// }
///
/// let mut slab = Slab::<MyKey, i32>::new();
/// let key: MyKey = slab.insert(0);
/// ```
#[macro_export]
macro_rules! define_slab_key {
    (
        $(#[$attrs:meta])*
        $vis:vis struct $name:ident : $base:ty;
    ) => {
        $(#[$attrs])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        $vis struct $name($base);

        impl $crate::collections::slab::SlabKey for $name {
            #[inline]
            fn from_index(index: usize) -> Option<Self> {
                <$base as $crate::collections::slab::SlabKey>::from_index(index).map(Self)
            }

            #[inline]
            fn from_index_unchecked(index: usize) -> Self {
                Self(<$base as $crate::collections::slab::SlabKey>::from_index_unchecked(index))
            }

            #[inline]
            fn to_index(&self) -> usize {
                <$base as $crate::collections::slab::SlabKey>::to_index(&self.0)
            }
        }
    };
}

pub use define_slab_key;

#[derive(Clone, Debug)]
enum Bucket<T> {
    Occupied(T),
    Free(usize),
}

impl<T> Bucket<T> {
    #[inline]
    unsafe fn assume_occupied_ref(&self) -> &T {
        match self {
            Self::Occupied(v) => v,
            // SAFETY: The caller guarantees that the bucket is occupied.
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    unsafe fn assume_occupied_mut(&mut self) -> &mut T {
        match self {
            Self::Occupied(v) => v,
            // SAFETY: The caller guarantees that the bucket is occupied.
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    unsafe fn assume_occupied(self) -> T {
        match self {
            Self::Occupied(v) => v,
            // SAFETY: The caller guarantees that the bucket is occupied.
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    unsafe fn assume_free(self) -> usize {
        match self {
            Self::Free(v) => v,
            // SAFETY: The caller guarantees that the bucket is free.
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, K, V> {
    buckets: slice::Iter<'a, Bucket<V>>,
    index: usize,
    len: usize,
    _marker: PhantomData<K>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: SlabKey,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.index;
            self.index += 1;

            match self.buckets.next()? {
                Bucket::Occupied(value) => {
                    let key = K::from_index_unchecked(index);
                    self.len -= 1;
                    return Some((key, value));
                }
                Bucket::Free(_) => (),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V>
where
    K: SlabKey,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, K, V> FusedIterator for Iter<'a, K, V> where K: SlabKey {}

#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    buckets: slice::IterMut<'a, Bucket<V>>,
    index: usize,
    len: usize,
    _marker: PhantomData<K>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: SlabKey,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.index;
            self.index += 1;

            match self.buckets.next()? {
                Bucket::Occupied(value) => {
                    let key = K::from_index_unchecked(index);
                    self.len -= 1;
                    return Some((key, value));
                }
                Bucket::Free(_) => (),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V>
where
    K: SlabKey,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, K, V> FusedIterator for IterMut<'a, K, V> where K: SlabKey {}

/// An iterator visiting all values of a [`Slab`].
///
/// Returned by [`values`].
///
/// [`values`]: Slab::values
#[derive(Clone, Debug)]
pub struct Values<'a, K, V> {
    iter: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: SlabKey,
{
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K, V> ExactSizeIterator for Values<'a, K, V>
where
    K: SlabKey,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> FusedIterator for Values<'a, K, V> where K: SlabKey {}

#[cfg(test)]
mod tests {
    use crate::collections::slab::Slab;

    #[test]
    fn slab_basic() {
        let mut slab = Slab::<usize, i32>::new();

        let a = slab.insert(0);
        let b = slab.insert(23);
        let c = slab.insert(53);
        let d = slab.insert(47);

        assert_eq!(slab.len(), 4);
        assert_eq!(*slab.get(a).unwrap(), 0);
        assert_eq!(*slab.get(b).unwrap(), 23);
        assert_eq!(*slab.get(c).unwrap(), 53);
        assert_eq!(*slab.get(d).unwrap(), 47);

        slab.remove(b);
        slab.remove(d);

        assert_eq!(slab.len(), 2);
        assert!(slab.contains(a));
        assert!(!slab.contains(b));
        assert!(slab.contains(c));
        assert!(!slab.contains(d));

        let e = slab.insert(100);
        assert_eq!(*slab.get(e).unwrap(), 100);
    }

    #[test]
    fn slab_insert_within_capacity_unchecked() {
        let mut slab = Slab::<usize, i32>::new();

        slab.reserve(2);
        let k0 = unsafe { slab.insert_within_capacity_unchecked(0) };
        let k1 = unsafe { slab.insert_within_capacity_unchecked(1) };

        assert_eq!(slab.remove(k1).unwrap(), 1);
        assert_eq!(slab.remove(k0).unwrap(), 0);

        let k2 = unsafe { slab.insert_within_capacity_unchecked(2) };
        let k3 = unsafe { slab.insert_within_capacity_unchecked(3) };

        slab.reserve(1);
        let k4 = unsafe { slab.insert_within_capacity_unchecked(4) };

        assert_eq!(slab.remove(k2).unwrap(), 2);
        assert_eq!(slab.remove(k3).unwrap(), 3);
        assert_eq!(slab.remove(k4).unwrap(), 4);
    }
}
