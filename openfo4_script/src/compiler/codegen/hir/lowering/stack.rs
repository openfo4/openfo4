use std::num::NonZeroUsize;

use indexmap::IndexMap;
use openfo4_jit::{Block, FunctionBuilder, IntCompareOp, Value};

use crate::compiler::codegen::hir::lowering::{Local, PTR_TYPE, Translator};
use crate::compiler::codegen::hir::{Function, Instruction, Variable};

#[derive(Clone, Debug)]
pub struct StackFrameInfo {
    /// Size of the entire stack for this function.
    pub size: usize,
    /// Suspension points after function calls.
    pub call_points: IndexMap<usize, SuspensionPoint>,
    pub points: IndexMap<usize, SuspensionPoint>,
}

impl StackFrameInfo {
    pub fn has_no_suspension_points(&self) -> bool {
        self.call_points.is_empty() && self.points.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct SuspensionPoint {
    /// The unique tag identifying our code position.
    pub tag: NonZeroUsize,
    /// The set of variables that this suspension point saves.
    pub variables: Vec<Variable>,
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

    for (index, instruction) in func.instructions.iter().enumerate() {
        match instruction {
            Instruction::Call(_) | Instruction::CallIndirect(_) => {
                let variables = func.variables().map(|v| v.0).collect();

                let tag = next_tag;
                next_tag = next_tag.checked_add(1).unwrap();

                // A call resumtpion semantically happens in the middle of an instruction
                // and we cannot just reuse the block at the start of the instruction, if
                // one exists.
                let block = builder.create_block(&[]);

                call_points.insert(
                    index,
                    SuspensionPoint {
                        tag,
                        variables,
                        block,
                    },
                );
            }
            _ => {}
        }
    }

    let mut max_size = 0;

    for point in call_points.values() {
        let mut size = PTR_TYPE.size();

        for var in &point.variables {
            let local = &locals[var.0];
            size = (size + local.ty.align() - 1) & !(local.ty.align() - 1);
            size += local.ty.size();
        }

        max_size = max_size.max(size);
    }

    StackFrameInfo {
        size: max_size,
        points: IndexMap::new(),
        call_points,
    }
}

/// Suspends execution of the function at the [`SuspensionPoint`].
pub fn suspend(translator: &mut Translator<'_>, stack_ptr: Value, point: &SuspensionPoint) {
    let tag = translator.builder.iconst(PTR_TYPE, point.tag.get() as i64);
    translator.builder.store(tag, stack_ptr, 0);

    let mut offset = PTR_TYPE.size();

    for var in &point.variables {
        let local = &translator.locals[var.0];
        let value = translator.builder.get_var(local.var);

        offset = (offset + local.ty.align() - 1) & !(local.ty.align() - 1);
        translator.builder.store(value, stack_ptr, offset as u32);

        offset += local.ty.size();
    }
}

pub fn resume(translator: &mut Translator<'_>, frame: &StackFrameInfo, start_block: Block) {
    if frame.has_no_suspension_points() {
        translator.builder.jump(start_block, &[]);
        return;
    }

    let stack_ptr = translator.load_stack_ptr();

    let tag = translator.builder.load(PTR_TYPE, stack_ptr, 0);

    let resume_block = translator.builder.create_block(&[]);

    // If tag == 0 we don't resume.
    translator
        .builder
        .brif(tag, resume_block, &[], start_block, &[]);

    translator.builder.switch_to_block(resume_block);

    let mut next_block = translator.builder.create_block(&[]);

    let num_points = frame.points.len() + frame.call_points.len();

    for (index, point) in frame
        .points
        .values()
        .chain(frame.call_points.values())
        .enumerate()
    {
        // If we reach the last point, the tag must match.
        if index + 1 != num_points {
            let wanted_tag = translator.builder.iconst(PTR_TYPE, point.tag.get() as i64);
            let tag_matches = translator
                .builder
                .icmp(tag, wanted_tag, IntCompareOp::Equal);

            let tag_matches_block = translator.builder.create_block(&[]);

            translator
                .builder
                .brif(tag_matches, tag_matches_block, &[], next_block, &[]);

            translator.builder.switch_to_block(tag_matches_block);
        }

        resume_point_load_vars(translator, stack_ptr, point);
        translator.builder.jump(point.block, &[]);

        if index + 1 != num_points {
            translator.builder.switch_to_block(next_block);
            next_block = translator.builder.create_block(&[]);
        }
    }
}

fn resume_point_load_vars(
    translator: &mut Translator<'_>,
    stack_ptr: Value,
    point: &SuspensionPoint,
) {
    // Tag was already read.
    let mut offset = PTR_TYPE.size();

    for var in &point.variables {
        let local = &translator.locals[var.0];

        offset = (offset + local.ty.align() - 1) & !(local.ty.align() - 1);

        let value = translator.builder.load(local.ty, stack_ptr, offset as u32);
        translator.builder.set_var(local.var, value);

        offset += local.ty.size();
    }
}
