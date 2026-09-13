use std::alloc::Layout;
use std::collections::BTreeMap;
use std::marker::PhantomData;

use bytemuck::NoUninit;

use crate::api::{Buffer, BufferDescriptor, CommandQueue};
use crate::backend::BufferUsage;
use crate::backend::allocator::{BuddyAllocator, GrowableAllocator, Region, UsageFlags};

/// A [`Buffer`] that supports insertion of a dynamic number of elements.
#[derive(Debug)]
pub struct SubAllocatedGrowableBuffer<T, A = BuddyAllocator> {
    buffer: Buffer,
    allocator: A,
    allocations: BTreeMap<usize, Region>,
    usage: BufferUsage,
    _marker: PhantomData<T>,
}

impl<T, A> SubAllocatedGrowableBuffer<T, A>
where
    T: NoUninit,
    A: GrowableAllocator,
{
    pub fn new(queue: &CommandQueue<'_>, usage: BufferUsage) -> Self {
        let buffer = queue.create_buffer(&BufferDescriptor {
            size: 1,
            usage: usage | BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
            flags: UsageFlags::empty(),
        });

        Self {
            buffer,
            allocator: A::new(Region::new(0, 1)),
            allocations: BTreeMap::new(),
            usage: usage | BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
            _marker: PhantomData,
        }
    }

    /// Inserts a new array of values into the buffer.
    ///
    /// Returns an index to the first value. The values are densly packed in the buffer.
    pub fn insert(&mut self, queue: &CommandQueue<'_>, value: &[T]) -> usize {
        let bytes = bytemuck::must_cast_slice(value);

        let layout = Layout::array::<T>(value.len()).unwrap();
        // Since we return an index into the buffer that represents offset in
        // number of `T`s we must also align our new element to a multiple of `T`.
        let layout = layout.align_to(size_of::<T>()).unwrap();
        if layout.size() == 0 {
            return 0;
        }

        if let Some(region) = self.allocator.alloc(layout) {
            let start = region.start();
            let end = region.start() + bytes.len();

            debug_assert!(start <= self.buffer.size());
            debug_assert!(end <= self.buffer.size());

            queue.write_buffer(self.buffer.slice(start..end), bytes);

            debug_assert_eq!(start % size_of::<T>(), 0);
            let index = start / size_of::<T>();
            self.allocations.insert(index, region);
            return index;
        }

        self.grow(queue, layout.pad_to_align().size());
        let region = self.allocator.alloc(layout).unwrap();

        let start = region.start();
        let end = region.start() + bytes.len();

        debug_assert!(start <= self.buffer.size());
        debug_assert!(end <= self.buffer.size());

        queue.write_buffer(self.buffer.slice(start..end), bytes);

        debug_assert_eq!(start % size_of::<T>(), 0);
        let index = start / size_of::<T>();
        self.allocations.insert(index, region);
        index
    }

    pub fn alloc(&mut self, queue: &CommandQueue<'_>, len: usize) -> usize {
        let layout = Layout::array::<T>(len).unwrap();
        // Since we return an index into the buffer that represents offset in
        // number of `T`s we must also align our new element to a multiple of `T`.
        let layout = layout.align_to(size_of::<T>()).unwrap();
        if layout.size() == 0 {
            return 0;
        }

        if let Some(region) = self.allocator.alloc(layout) {
            let start = region.start();
            let end = region.start() + layout.size();

            debug_assert!(start <= self.buffer.size());
            debug_assert!(end <= self.buffer.size());

            debug_assert_eq!(start % size_of::<T>(), 0);
            let index = start / size_of::<T>();
            self.allocations.insert(index, region);
            return index;
        }

        self.grow(queue, layout.pad_to_align().size());
        let region = self.allocator.alloc(layout).unwrap();

        let start = region.start();
        let end = region.start() + layout.size();

        debug_assert!(start <= self.buffer.size());
        debug_assert!(end <= self.buffer.size());

        debug_assert_eq!(start % size_of::<T>(), 0);
        let index = start / size_of::<T>();
        self.allocations.insert(index, region);
        index
    }

    pub fn write(&mut self, queue: &CommandQueue<'_>, index: usize, values: &[T]) {
        if values.is_empty() {
            return;
        }

        let bytes = bytemuck::must_cast_slice(values);

        let region = self
            .allocations
            .get(&index)
            .expect("invalid allocation index");

        let start = region.start();
        let end = region.start() + bytes.len();

        queue.write_buffer(self.buffer.slice(start..end), bytes);
    }

    /// Removes the allocation starting at the given offset.
    pub fn remove(&mut self, index: usize) {
        if let Some(region) = self.allocations.remove(&index) {
            unsafe {
                self.allocator.dealloc(region);
            }
        }
    }

    fn grow(&mut self, queue: &CommandQueue<'_>, size: usize) {
        let old_size = self.buffer.size();
        let new_size =
            usize::max(old_size << 1, old_size + size.next_power_of_two()).next_power_of_two();

        let new_buffer = queue.create_buffer(&BufferDescriptor {
            size: new_size,
            usage: self.usage,
            flags: UsageFlags::empty(),
        });

        queue.copy_buffer_to_buffer(self.buffer.slice(..), new_buffer.slice(..old_size));

        unsafe {
            self.allocator.grow(Region::new(0, new_size as usize));
        }

        self.buffer = new_buffer;
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
