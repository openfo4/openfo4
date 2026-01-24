use std::alloc::Layout;
use std::ffi::c_void;
use std::os::fd::OwnedFd;

use hashbrown::HashMap;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use rustix::mm::{MapFlags, ProtFlags, mmap, munmap};
use slab::Slab;

#[derive(Debug)]
pub struct MemoryAllocator {
    bins: HashMap<usize, BinList>,
}

impl MemoryAllocator {
    pub fn new() -> Self {
        Self {
            bins: HashMap::new(),
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> Allocation {
        let layout = layout.pad_to_align();
        let bin_size = layout.size().next_power_of_two();

        let bins = self
            .bins
            .entry(bin_size)
            .or_insert_with(|| BinList::new(bin_size));

        let (bin_index, memory) = bins.alloc(layout);

        Allocation {
            memory,
            bin_size,
            bin_index,
        }
    }
}

#[derive(Debug)]
pub struct Allocation {
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

    fn alloc(&mut self, layout: Layout) -> (usize, AllocationMemory) {
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
                        size: layout.size(),
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
                size: layout.size(),
            },
        )
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
    size: usize,
}

unsafe impl Send for AllocationMemory {}
unsafe impl Sync for AllocationMemory {}

#[derive(Debug)]
struct Page {
    fd: OwnedFd,
    rw_map: *mut c_void,
    rx_map: *mut c_void,
    size: usize,
}

impl Page {
    pub fn new(size: usize) -> Self {
        debug_assert!(size.is_power_of_two());

        let fd = memfd_create("", MemfdFlags::CLOEXEC).unwrap();
        ftruncate(&fd, size.try_into().unwrap()).unwrap();

        let rw_map = unsafe {
            mmap(
                core::ptr::null_mut(),
                size,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::SHARED,
                &fd,
                0,
            )
            .unwrap()
        };
        let rx_map = unsafe {
            mmap(
                core::ptr::null_mut(),
                size,
                ProtFlags::READ | ProtFlags::EXEC,
                MapFlags::SHARED,
                &fd,
                0,
            )
            .unwrap()
        };

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
            munmap(self.rw_map, self.size).unwrap();
            munmap(self.rx_map, self.size).unwrap();
        }
    }
}

unsafe impl Send for Page {}
unsafe impl Sync for Page {}
