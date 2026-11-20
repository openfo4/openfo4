use std::hash::Hash;
use std::num::NonZeroUsize;

use indexmap::IndexMap;
use openfo4_common::collections::vec_map::VecMap;
use openfo4_jit::{Block, FunctionBuilder, IntCompareOp};

use crate::compiler::codegen::hir::lowering::{ISIZE_TYPE, Local, Translator};
use crate::compiler::codegen::hir::{Function, Instruction, Variable};

#[derive(Clone, Debug)]
pub struct StackFrameInfo {
    /// Size of the entire stack for this function.
    pub size: usize,
    /// Suspension points after function calls.
    pub call_points: IndexMap<InstructionPosition, SuspensionPoint>,
    /// Offset from stack base where the variable is stored.
    variable_offsets: VecMap<Variable, usize>,
}

impl StackFrameInfo {
    pub fn has_no_suspension_points(&self) -> bool {
        self.call_points.is_empty()
    }

    /// Returns the size of the entire stack frame, aligned up to `align`.
    pub fn size_aligned_to(&self, align: usize) -> usize {
        debug_assert_ne!(align, 0);
        debug_assert!(align.is_power_of_two());
        (self.size + align - 1) & !(align - 1)
    }

    pub fn variable_offset(&self, var: Variable) -> usize {
        *self.variable_offsets.get(var).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct SuspensionPoint {
    /// The unique tag identifying our code position.
    pub tag: NonZeroUsize,
    /// The block to skip towards when resuming.
    pub block: Block,
}

pub fn compute_suspension_points(
    builder: &mut FunctionBuilder<'_>,
    func: &Function,
    locals: &[Local],
) -> StackFrameInfo {
    let mut next_tag = NonZeroUsize::MIN;
    let mut call_points = IndexMap::new();

    for (block_index, block) in func.blocks.iter().enumerate() {
        for (inst_index, instruction) in block.iter().enumerate() {
            match instruction {
                Instruction::Call(_)
                | Instruction::CallIndirect(_)
                | Instruction::Lock { object: _, ty: _ } => {
                    let tag = next_tag;
                    next_tag = next_tag.checked_add(1).unwrap();

                    // A call resumtpion semantically happens in the middle of an instruction
                    // and we cannot just reuse the block at the start of the instruction, if
                    // one exists.
                    let block = builder.create_block(&[]);

                    call_points.insert(
                        InstructionPosition {
                            block: block_index,
                            instruction: inst_index,
                        },
                        SuspensionPoint { tag, block },
                    );
                }
                _ => {}
            }
        }
    }

    // The first slot is reserved for the tag.
    let mut size = size_of::<usize>();

    let mut variable_offsets = VecMap::new();

    let mut variables: Vec<_> = func.variables().map(|(var, _)| var).collect();
    // Sort by alignment descending to minimize stack space.
    variables.sort_unstable_by(|a, b| {
        let local_a = &locals[a.0];
        let local_b = &locals[b.0];
        local_b.ty.align().cmp(&local_a.ty.align())
    });

    for (var, _) in func.variables() {
        let local = &locals[var.0];

        size = (size + local.ty.align() - 1) & !(local.ty.align() - 1);
        variable_offsets.insert(var, size);
        size += local.ty.size();
    }

    StackFrameInfo {
        size,
        call_points,
        variable_offsets,
    }
}

pub fn resume(translator: &mut Translator<'_>, frame: &StackFrameInfo, start_block: Block) {
    if frame.has_no_suspension_points() {
        translator.builder.jump(start_block, &[]);
        return;
    }

    let stack_ptr = translator.load_stack_ptr();

    let tag = translator.builder.load(ISIZE_TYPE, stack_ptr, 0);

    let resume_block = translator.builder.create_block(&[]);

    // If tag == 0 we don't resume.
    translator
        .builder
        .brif(tag, resume_block, &[], start_block, &[]);

    let mut block = resume_block;

    for (index, point) in frame.call_points.values().enumerate() {
        translator.builder.switch_to_block(block);

        if index + 1 == frame.call_points.len() {
            translator.builder.jump(point.block, &[]);
            continue;
        }

        let next_block = translator.builder.create_block(&[]);
        let is_eq = translator
            .builder
            .icmp_imm(tag, point.tag.get(), IntCompareOp::Equal);
        translator
            .builder
            .brif(is_eq, point.block, &[], next_block, &[]);
        block = next_block;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstructionPosition {
    pub block: usize,
    pub instruction: usize,
}
