use slab::Slab;

use crate::{Instruction, Value};

#[cfg(feature = "cranelift")]
pub mod cranelift;

#[derive(Clone, Debug)]
pub struct FunctionDefinitionDesc {
    pub instructions: Vec<Instruction>,
    pub num_values: usize,
    pub num_blocks: usize,
    pub block_params: Vec<Vec<Value>>,
    pub stack_slots: Slab<StackSlotDesc>,
}

#[derive(Copy, Clone, Debug)]
pub struct StackSlotDesc {
    pub size: usize,
    pub align: usize,
}
