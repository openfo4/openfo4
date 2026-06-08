use std::mem::offset_of;

use openfo4_jit::IntCompareOp;
use openfo4_jit::Type as JitType;
use openfo4_jit::Value as JitValue;

use crate::compiler::codegen::FunctionDef;
use crate::compiler::codegen::function::translate_function_signature;
use crate::compiler::codegen::hir::Function;
use crate::compiler::codegen::hir::Literal;
use crate::compiler::codegen::hir::Null;
use crate::compiler::codegen::hir::Value;
use crate::compiler::codegen::hir::lowering::RESULT_TYPE;
use crate::compiler::codegen::hir::lowering::Translator;
use crate::compiler::codegen::hir::lowering::stack;
use crate::compiler::codegen::hir::lowering::stack::StackFrameInfo;
use crate::compiler::codegen::hir::lowering::stack::SuspensionPoint;
use crate::runtime::RESULT_OK;
use crate::runtime::RawContext;
use crate::runtime::RawStack;

pub fn perform_checked_call(
    translator: &mut Translator<'_>,
    caller: &Function,
    callee: &FunctionDef,
    args: &[Value<Literal>],
    dst: Option<&Value<Literal>>,
    object: Option<&Value<Null>>,
    suspension_point: &SuspensionPoint,
    stack_frame: &StackFrameInfo,
) {
    let callee_returns_value = !callee.function.return_ty.is_empty()
        && !callee.function.return_ty.eq_ignore_ascii_case("none");

    let resume_call_block = suspension_point.block;

    // Abort if object == null.
    // We only need to check this on the first call.
    // If we suspend all future resumptions will reuse the same non-null
    // value.
    if let Some(object) = object {
        let object = translator.load(object);
        translator.assert_true(object);
    }

    // We need to allocate a single usize tag frame at the stack top.
    // We need to set the tag to 0, which indicates to the callee
    // that the function is being called for the first time.
    translator.stack_alloc(size_of::<usize>());

    translator.builder.jump(resume_call_block, &[]);

    translator.builder.switch_to_block(resume_call_block);

    // Move the stack pointer over our current (caller) stack space.
    let new_stack_len = translator
        .builder
        .iadd_imm(translator.stack_start_offset, stack_frame.size as i64);
    translator.builder.store(
        new_stack_len,
        translator.ctx_ptr,
        (offset_of!(RawContext, stack_ptr) + offset_of!(RawStack, len)) as u32,
    );

    let mut args: Vec<_> = args.iter().map(|arg| translator.load(arg)).collect();
    args.insert(0, translator.ctx_ptr);

    let res = if let Some(object) = object {
        let signature = translate_function_signature(translator.data, &callee.function);

        let object = translator.load(object);

        let fn_ptr = load_function_addr(translator, callee, object);
        translator.builder.call_indirect(signature, fn_ptr, &args)
    } else {
        let func_id = callee.func_id.unwrap();

        translator.builder.call(func_id, &args)
    };

    // The callee will leave the stack len at the same value as when we called it.
    // The callee may have invalidated the previous stack pointer and we must
    // reload our stack pointer.
    // We don't need to restore stack len because only two cases may happen:
    // 1. The callee wants to suspend: In this case each function in the callstack
    //    will suspend using its currently loaded stack offset (which remains valid).
    //    When the function is resumed the host will always start with a stack len of 0.
    // 2. The callee returns and our function continues: In this case we reuse our local
    //    currently loaded stack offset whenever we call another function, overwriting
    //    the stack len written by the callee.
    let stack_ptr = translator.load_stack_ptr();

    let result_pos = if callee_returns_value { 1 } else { 0 };

    let result = res.get(result_pos).unwrap();

    let continue_block = translator.builder.create_block(&[]);
    let suspend_block = translator.builder.create_block(&[]);

    let result_ok = translator.builder.iconst(RESULT_TYPE, RESULT_OK);
    let is_ok = translator
        .builder
        .icmp(*result, result_ok, IntCompareOp::Equal);

    translator
        .builder
        .brif(is_ok, continue_block, &[], suspend_block, &[]);

    translator.builder.switch_to_block(suspend_block);
    stack::suspend(translator, stack_ptr, suspension_point);

    let mut rvals = Vec::new();
    if let Some(ty) = caller.returns {
        let ty = translator.lowered_ty_from_id(ty);
        rvals.push(translator.builder.uninit(ty));
    }
    rvals.push(*result);

    translator.builder.jump(translator.return_block, &rvals);

    translator.builder.switch_to_block(continue_block);

    // Release the tag for the callee frame.
    translator.stack_free(size_of::<usize>());

    translator.current_block = continue_block;

    if let Some(dst) = dst
        && callee_returns_value
    {
        let res = res[0];
        translator.store(dst, res);
    }
}

pub fn load_function_addr(
    translator: &mut Translator<'_>,
    callee: &FunctionDef,
    object_ptr: JitValue,
) -> JitValue {
    let class = translator.data.classes.get(&callee.class).unwrap();
    let func_index = class.functions.get_index_of(&callee.name).unwrap();

    let mut class_index = 0;
    let mut next_parent = class.parent.as_ref();
    while let Some(parent) = next_parent {
        class_index += 1;
        let class = translator.data.classes.get(parent).unwrap();
        next_parent = class.parent.as_ref();
    }

    let header_ptr = translator.builder.load(JitType::Pointer, object_ptr, 0);

    // 1 usize for Class Count
    // 2 usize for each skipped Class (TypeId, *VTable)
    // 1 usize for the TypeId of our class
    let byte_class_offset =
        size_of::<usize>() + size_of::<usize>() * 2 * class_index + size_of::<usize>();

    let vtable_ptr =
        translator
            .builder
            .load(JitType::Pointer, header_ptr, byte_class_offset as u32);

    let byte_func_offset = size_of::<*const u8>() * func_index;
    let fn_ptr = translator
        .builder
        .load(JitType::Pointer, vtable_ptr, byte_func_offset as u32);

    fn_ptr
}
