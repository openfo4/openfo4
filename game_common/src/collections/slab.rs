//! A slab, similar to the [`slab`] crate.
//!
//! The main difference is that this [`Slab`] implementation stores entries and residency
//! information separately.
//!
//! This reduces memory usage and cache pressure for unchecked accesses.
//!
//! [`slab`]: https://docs.rs/slab/latest/slab/

use std::fmt::{self, Debug, Formatter};
use std::iter::FusedIterator;
use std::mem::MaybeUninit;

use allocator_api2::vec::Vec;

pub struct Slab<T> {
    // Note: We store data and residency separately and also don't use a union { T, usize }
    // here as that would each data entry to usize.
    // Skipping this allows for less memory accesses when using unchecked the methods.
    // FIXME: We could try to store both Vecs in one allocation.
    data: Vec<MaybeUninit<T>>,
    // A value of usize::MAX indicates the presence of a value.
    residency: Vec<usize>,
    len: usize,
    next_free: usize,
}

impl<T> Slab<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            residency: Vec::new(),
            next_free: 0,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if a value with the give `key` exists in the `Slab`.
    pub fn contains(&self, key: usize) -> bool {
        self.residency
            .get(key)
            .is_some_and(|residency| *residency == usize::MAX)
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        if self.contains(key) {
            // SAFETY: We just checked that the key exists.
            Some(unsafe { self.get_unchecked(key) })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if self.contains(key) {
            // SAFETY: We just checked that the key exists.
            Some(unsafe { self.get_unchecked_mut(key) })
        } else {
            None
        }
    }

    /// Returns a reference to the value at the given `key` without checking if it exists.
    ///
    /// # Safety
    ///
    /// The given `key` must currently exist in the `Slab`.
    pub unsafe fn get_unchecked(&self, key: usize) -> &T {
        // SAFEY: The caller guarantees that the index is valid
        // and a initialized value exists and the given index.
        unsafe { self.data.get_unchecked(key).assume_init_ref() }
    }

    /// Returns a mutable reference to the value at the given `key` without checking if it exists.
    ///
    /// # Safety
    ///
    /// The given `key` must currently exist in the `Slab`.
    pub unsafe fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        // SAFEY: The caller guarantees that the index is valid
        // and a initialized value exists and the given index.
        unsafe { self.data.get_unchecked_mut(key).assume_init_mut() }
    }

    /// Inserts a new value into the [`Slab`] and returns the key to access it.
    pub fn insert(&mut self, value: T) -> usize {
        let index = self.next_free;

        if index == self.data.len() {
            self.data.push(MaybeUninit::new(value));
            self.residency.push(usize::MAX);
            self.next_free = index + 1;
        } else {
            // SAFETY: We guarantee that `next_free` is in bounds at
            // all times.
            unsafe {
                let residency_slot = self.residency.get_unchecked_mut(index);
                debug_assert_ne!(*residency_slot, usize::MAX);
                self.next_free = *residency_slot;

                let slot = self.data.get_unchecked_mut(index);
                slot.write(value);

                // Mark as occupied.
                *residency_slot = usize::MAX;
            }
        }

        self.len += 1;

        index
    }

    pub fn remove(&mut self, key: usize) -> Option<T> {
        if self.contains(key) {
            // SAFETY: We just checked that the key exists.
            Some(unsafe { self.remove_unchecked(key) })
        } else {
            None
        }
    }

    /// Removes the entry with the given `key` from the `Slab` without checking if it exists.
    ///
    /// # Safety
    ///
    /// A entry with the given `key` must currently exist in the `Slab`.
    pub unsafe fn remove_unchecked(&mut self, key: usize) -> T {
        let index = key as usize;

        // SAFETY: The caller guarantees that the given index is in bounds.
        let residency_slot = unsafe { self.residency.get_unchecked_mut(index) };
        let data_slot = unsafe { self.data.get_unchecked_mut(index) };

        debug_assert_eq!(*residency_slot, usize::MAX);
        *residency_slot = self.next_free;
        self.next_free = index;

        // SAFETY: The caller guarantees that a value exists at the given index.
        let value = unsafe { data_slot.assume_init_read() };

        self.len -= 1;

        value
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            data: self.data.iter(),
            residency: self.residency.iter(),
            index: 0,
            len: self.len,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            data: self.data.iter_mut(),
            residency: self.residency.iter_mut(),
            index: 0,
            len: self.len,
        }
    }

    pub fn values(&self) -> Values<'_, T> {
        Values { iter: self.iter() }
    }
}

impl<T> Clone for Slab<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut data = Vec::with_capacity(self.data.len());
        data.resize_with(self.data.len(), || MaybeUninit::uninit());

        for index in 0..self.data.len() {
            if let Some(value) = self.get(index) {
                data[index].write(value.clone());
            }
        }

        Self {
            data,
            residency: self.residency.clone(),
            len: self.len,
            next_free: self.next_free,
        }
    }
}

impl<T> Debug for Slab<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_map().entries(self.iter()).finish()
        } else {
            f.debug_struct("Slab")
                .field("len", &self.len)
                .field("cap", &self.data.capacity())
                .finish_non_exhaustive()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    data: core::slice::Iter<'a, MaybeUninit<T>>,
    residency: core::slice::Iter<'a, usize>,
    index: usize,
    len: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.index;
            let data = self.data.next()?;
            let residency = self.residency.next()?;
            self.index += 1;

            if *residency == usize::MAX {
                self.len -= 1;

                // SAFETY: residency == usize::MAX indicates a value exists
                // at this index.
                let data = unsafe { data.assume_init_ref() };
                return Some((index, data));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    data: core::slice::IterMut<'a, MaybeUninit<T>>,
    residency: core::slice::IterMut<'a, usize>,
    index: usize,
    len: usize,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.index;
            let data = self.data.next()?;
            let residency = self.residency.next()?;
            self.index += 1;

            if *residency == usize::MAX {
                self.len -= 1;

                // SAFETY: residency == usize::MAX indicates a value exists
                // at this index.
                let data = unsafe { data.assume_init_mut() };
                return Some((index, data));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}

/// An iterator visiting all values of a [`Slab`].
///
/// Returned by [`values`].
///
/// [`values`]: Slab::values
#[derive(Clone, Debug)]
pub struct Values<'a, T> {
    iter: Iter<'a, T>,
}

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.len(), Some(self.iter.len()))
    }
}

impl<'a, T> ExactSizeIterator for Values<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, T> FusedIterator for Values<'a, T> {}

#[cfg(test)]
mod tests {
    use crate::collections::slab::Slab;

    #[test]
    fn slab_basic() {
        let mut slab = Slab::new();

        let a = slab.insert(0);
        let b = slab.insert(23);
        let c = slab.insert(53);
        let d = slab.insert(47);

        assert_eq!(*slab.get(a).unwrap(), 0);
        assert_eq!(*slab.get(b).unwrap(), 23);
        assert_eq!(*slab.get(c).unwrap(), 53);
        assert_eq!(*slab.get(d).unwrap(), 47);

        slab.remove(b);
        slab.remove(d);

        assert!(slab.contains(a));
        assert!(!slab.contains(b));
        assert!(slab.contains(c));
        assert!(!slab.contains(d));

        let e = slab.insert(100);
        assert_eq!(*slab.get(e).unwrap(), 100);
    }
}
