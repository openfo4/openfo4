use openfo4_jit::Type as JitType;
use openfo4_jit::Value as JitValue;

use crate::compiler::codegen::function::FunctionTranslator;

/// Allocates a new frame on the stack capable of storing the given [`StackFrame`].
///
/// Returns the pointer to the memory.
pub fn push_stack_frame(translator: &mut FunctionTranslator<'_>) -> JitValue {
    // This function is called in the function prologue.
    // Our ABI guarantees that the caller already allocated a `usize` for us
    // that we should reuse.
    let size = translator.stack_frame.size - size_of::<usize>();
    let size = translator.builder.iconst(JitType::Pointer, size as i64);

    // The callee will ensure that at least `size` bytes are available starting
    // from `stack_base`.
    // The callee MAY invalidate the current `stack_base` pointer.
    let stack_base = translator.builder.call(
        translator.data.runtime_funcs.stack_frame_alloc,
        &[translator.ctx_ptr, size],
    )[0];

    translator.builder.store(stack_base, translator.ctx_ptr, 0);
    stack_base
}

pub fn pop_stack_frame(translator: &mut FunctionTranslator<'_>) {
    let size = translator
        .builder
        .iconst(JitType::Pointer, translator.stack_frame.size as i64);

    translator.builder.call(
        translator.data.runtime_funcs.stack_frame_free,
        &[translator.ctx_ptr, size],
    );
}

pub fn prepare_stack_for_call(translator: &mut FunctionTranslator<'_>) -> JitValue {
    let size = translator
        .builder
        .iconst(JitType::Pointer, size_of::<usize>() as i64);

    // The callee will ensure that at least `size` bytes are available starting
    // from `stack_base`.
    // The callee MAY invalidate the current `stack_base` pointer.
    let stack_base = translator.builder.call(
        translator.data.runtime_funcs.stack_frame_alloc,
        &[translator.ctx_ptr, size],
    )[0];

    translator.builder.store(stack_base, translator.ctx_ptr, 0);
    stack_base
}
