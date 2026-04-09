use crate::runtime::Func;

#[derive(Clone, Debug)]
pub struct Stack {
    memory: Vec<u8>,
    len: usize,
}

impl Stack {
    pub fn new(size: usize) -> Self {
        Self {
            memory: vec![0; size],
            len: 0,
        }
    }

    pub fn for_func(func: &Func) -> Self {
        Self::new(func.stack_size())
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

        unsafe { self.memory.as_mut_ptr().add(offset) }
    }

    pub(crate) fn free(&mut self, size: usize) {
        self.len -= size;
    }
}
