use std::alloc::Layout;
use std::ffi::c_void;
use std::sync::Arc;

use hashbrown::HashMap;
use openfo4_io::memfd::{Memfd, ProtFlags};
use parking_lot::Mutex;
use slab::Slab;

#[derive(Clone, Debug)]
pub struct MemoryAllocator {
    bins: Arc<Mutex<HashMap<usize, BinList>>>,
}

impl MemoryAllocator {
    pub fn new() -> Self {
        Self {
            bins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn alloc(&self, layout: Layout) -> Allocation {
        let layout = layout.pad_to_align();
        let bin_size = layout.size().next_power_of_two();

        let mut bins = self.bins.lock();

        let bins = bins
            .entry(bin_size)
            .or_insert_with(|| BinList::new(bin_size));

        let (bin_index, memory) = bins.alloc();

        Allocation {
            allocator: self.clone(),
            memory,
            bin_size,
            bin_index,
        }
    }

    unsafe fn dealloc(&self, alloc: &Allocation) {
        let mut bins = self.bins.lock();

        let bins = bins.get_mut(&alloc.bin_size).unwrap();

        unsafe {
            bins.dealloc(alloc.bin_index, &alloc.memory);
        }
    }
}

#[derive(Debug)]
pub struct Allocation {
    allocator: MemoryAllocator,
    memory: AllocationMemory,
    bin_index: usize,
    bin_size: usize,
}

impl Allocation {
    pub fn as_writable_ptr(&self) -> *mut u8 {
        self.memory.rw_mem.cast()
    }

    pub fn as_exec_ptr(&self) -> *const u8 {
        self.memory.rx_mem.cast()
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            self.allocator.dealloc(self);
        }
    }
}

#[derive(Debug)]
struct BinList {
    bin_size: usize,
    bins: Slab<Bin>,
    next_bin_slot_count: usize,
}

impl BinList {
    fn new(bin_size: usize) -> Self {
        // Avoid allocating pages with less than 4KiB size.
        let next_bin_slot_count = usize::max(4096 / bin_size, 1);

        Self {
            bin_size,
            bins: Slab::new(),
            next_bin_slot_count,
        }
    }

    fn alloc(&mut self) -> (usize, AllocationMemory) {
        for (bin_index, bin) in &mut self.bins {
            if let Some(index) = bin.free_slots.pop() {
                bin.num_allocs += 1;

                let offset = self.bin_size * index;
                let rw_mem = unsafe { bin.page.rw_map.byte_add(offset) };
                let rx_mem = unsafe { bin.page.rx_map.byte_add(offset) };

                return (
                    bin_index,
                    AllocationMemory {
                        rw_mem,
                        rx_mem,
                        bin_slot: index,
                    },
                );
            }
        }

        let page = Page::new(self.next_bin_slot_count * self.bin_size);
        let mut free_slots = (0..self.next_bin_slot_count).collect::<Vec<_>>();
        self.next_bin_slot_count *= 2;

        let index = free_slots.pop().unwrap();
        let offset = self.bin_size * index;
        let rw_mem = unsafe { page.rw_map.byte_add(offset) };
        let rx_mem = unsafe { page.rx_map.byte_add(offset) };

        let bin_index = self.bins.insert(Bin {
            page,
            num_allocs: 1,
            free_slots,
        });

        (
            bin_index,
            AllocationMemory {
                rw_mem,
                rx_mem,
                bin_slot: index,
            },
        )
    }

    unsafe fn dealloc(&mut self, bin_index: usize, mem: &AllocationMemory) {
        let bin = &mut self.bins[bin_index];

        bin.free_slots.push(mem.bin_slot);
        bin.num_allocs -= 1;

        if bin.num_allocs == 0 {
            self.bins.remove(bin_index);
        }
    }
}

#[derive(Debug)]
struct Bin {
    page: Page,
    num_allocs: usize,
    free_slots: Vec<usize>,
}

#[derive(Copy, Clone, Debug)]
struct AllocationMemory {
    rw_mem: *mut c_void,
    rx_mem: *mut c_void,
    bin_slot: usize,
}

unsafe impl Send for AllocationMemory {}
unsafe impl Sync for AllocationMemory {}

#[derive(Debug)]
struct Page {
    fd: Memfd,
    rw_map: *mut c_void,
    rx_map: *mut c_void,
    size: usize,
}

impl Page {
    pub fn new(size: usize) -> Self {
        debug_assert!(size.is_power_of_two());

        let fd = Memfd::new(size).unwrap();

        let rw_map = unsafe { fd.map(size, ProtFlags::READ | ProtFlags::WRITE).unwrap() };
        let rx_map = unsafe { fd.map(size, ProtFlags::READ | ProtFlags::EXEC).unwrap() };

        Self {
            rw_map,
            rx_map,
            size,
            fd,
        }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        unsafe {
            self.fd.unmap(self.rw_map, self.size).ok();
            self.fd.unmap(self.rx_map, self.size).ok();
        }
    }
}

unsafe impl Send for Page {}
unsafe impl Sync for Page {}
