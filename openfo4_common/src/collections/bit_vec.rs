use allocator_api2::alloc::{Allocator, Global};
use allocator_api2::vec::Vec;

pub struct BitVec<A = Global>
where
    A: Allocator,
{
    bits: Vec<usize, A>,
    nbits: usize,
}

impl BitVec {
    /// Creates a new `BitVec` with `nbits` elements set to `bit`.
    pub fn from_elem(nbits: usize, bit: bool) -> Self {
        Self::from_elem_in(nbits, bit, Global)
    }
}

impl<A> BitVec<A>
where
    A: Allocator,
{
    /// Creates a new `BitVec` with `nbits` elements set to `bit`.
    pub fn from_elem_in(nbits: usize, bit: bool, alloc: A) -> Self {
        let len = nbits.div_ceil(usize::BITS as usize);

        let mut bits = Vec::with_capacity_in(len, alloc);

        if bit {
            bits.resize(len, usize::MAX);
        } else {
            bits.resize(len, 0);
        }

        Self { bits, nbits }
    }

    /// Returns the number of bits in this `BitVec`.
    pub fn len(&self) -> usize {
        self.nbits
    }

    /// Returns `true` if there are no bits in this `BitVec`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the value at the given `index`.
    ///
    /// # Panics
    ///
    /// Panics if the `index` is out of bounds.
    #[track_caller]
    pub fn get(&self, index: usize) -> bool {
        assert!(index < self.nbits, "index out of bounds");

        unsafe { self.get_unchecked(index) }
    }

    /// Returns the value at the given `index` without doing bounds checking.
    ///
    /// # Safety
    ///
    /// The given `index` must be in bounds, i.e. less than [`len`].
    ///
    /// [`len`]: Self::len
    pub unsafe fn get_unchecked(&self, index: usize) -> bool {
        let word = index / usize::BITS as usize;
        let bit = index % usize::BITS as usize;

        // SAFETY: The caller guarantees that this is in bounds.
        let word = *unsafe { self.bits.get_unchecked(word) };

        word & (1 << bit) != 0
    }

    /// Sets the value at the given `index`.
    ///
    /// # Panics
    ///
    /// Panics if the `index` is out of bounds.
    #[track_caller]
    pub fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.nbits, "index out of bounds");

        unsafe {
            self.set_unchecked(index, value);
        }
    }

    /// Sets the value at the given `index` without doing bounds checking.
    ///
    /// # Safety
    ///
    /// The given `index` must be in bounds, i.e. less than [`len`].
    ///
    /// [`len`]: Self::len
    pub unsafe fn set_unchecked(&mut self, index: usize, value: bool) {
        let word = index / usize::BITS as usize;
        let bit = index % usize::BITS as usize;

        // SAFETY: The caller guarantees that this is in bounds.
        let word = unsafe { self.bits.get_unchecked_mut(word) };

        if value {
            *word |= 1 << bit;
        } else {
            // If the bit is set, it will flip to false,
            // all other bits are unchanged.
            *word ^= 1 << bit;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::bit_vec::BitVec;

    #[test]
    fn bitvec_basic() {
        let mut bits = BitVec::from_elem(100, false);

        for i in 0..100 {
            assert_eq!(bits.get(i), false);
        }

        for i in 0..20 {
            bits.set(i * 5, true);
        }

        for i in 0..100 {
            if i % 5 == 0 {
                assert_eq!(bits.get(i), true);
            } else {
                assert_eq!(bits.get(i), false);
            }
        }
    }
}
