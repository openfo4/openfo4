#[derive(Clone, Debug)]
pub struct Stack {
    memory: Vec<u8>,
}

impl Stack {
    pub fn new(size: usize) -> Self {
        Self {
            memory: vec![0; size],
        }
    }

    pub fn as_ptr(&mut self) -> *mut u8 {
        self.memory.as_mut_ptr()
    }
}
