use std::sync::Arc;

use crate::engine::EngineShared;

#[derive(Clone, Debug)]
pub struct Stack {
    pub(crate) engine: Arc<EngineShared>,
    /// The top-level guest function that created the callstack.
    ///
    /// If `None` the callstack is empty.
    pub(crate) func: Option<*const u8>,
    memory: Vec<u8>,
    len: usize,
}

impl Stack {
    pub(crate) fn new_with_engine(engine: Arc<EngineShared>) -> Self {
        Self {
            engine,
            func: None,
            memory: Vec::new(),
            len: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.len
    }

    pub fn as_ptr(&mut self) -> *mut u8 {
        self.memory.as_mut_ptr()
    }

    pub(crate) fn alloc(&mut self, size: usize) -> *mut u8 {
        let offset = self.memory.len();
        self.memory.resize(offset + size, 0);
        self.len += size;

        self.memory.as_mut_ptr()
    }

    pub(crate) fn free(&mut self, size: usize) -> *mut u8 {
        self.len -= size;
        self.memory.as_mut_ptr()
    }
}

// SAFETY: All stack contents are plain-old-data types or pointers
// to objects that are allowed to be accessed from any thread.
unsafe impl Send for Stack {}

// SAFETY: Any access that reads or writes the Stack requires mutable access.
unsafe impl Sync for Stack {}
