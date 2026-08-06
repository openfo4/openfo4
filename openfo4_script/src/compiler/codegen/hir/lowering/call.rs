use std::mem::offset_of;

use openfo4_jit::IntCompareOp;
use openfo4_jit::Type as JitType;
use openfo4_jit::Value as JitValue;

use crate::compiler::codegen::FunctionDef;
use crate::compiler::codegen::function::translate_function_signature;
use crate::compiler::codegen::hir::Literal;
use crate::compiler::codegen::hir::Null;
use crate::compiler::codegen::hir::Value;
use crate::compiler::codegen::hir::lowering::ISIZE_TYPE;
use crate::compiler::codegen::hir::lowering::Translator;
use crate::compiler::codegen::hir::lowering::stack::StackFrameInfo;
use crate::compiler::codegen::hir::lowering::stack::SuspensionPoint;
use crate::compiler::codegen::pex::registry::ClassId;
use crate::compiler::codegen::pex::registry::FunctionId;
use crate::compiler::codegen::pex::registry::FunctionSet;
use crate::compiler::codegen::pex::registry::Type;
use crate::runtime::RESULT_OK;
use crate::runtime::RawContext;
use crate::runtime::RawStack;

pub fn perform_checked_call(
    translator: &mut Translator<'_>,
    callee: &FunctionDef,
    args: &[Value<Literal>],
    dst: Option<&Value<Literal>>,
    object: Option<&Value<Null>>,
    suspension_point: &SuspensionPoint,
    stack_frame: &StackFrameInfo,
) {
    let callee_returns_value = callee.returns != Type::None;

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

    // The tag must be usize aligned.
    let size_padded = stack_frame.size_aligned_to(align_of::<usize>());
    let padding = size_padded - stack_frame.size;
    translator.stack_alloc(size_of::<usize>() + padding);

    let stack_ptr = translator.load_stack_ptr();
    let zero_tag = translator.builder.iconst(ISIZE_TYPE, 0);
    translator.builder.store(zero_tag, stack_ptr, size_padded);

    translator.builder.jump(resume_call_block, &[]);

    translator.builder.switch_to_block(resume_call_block);

    // Move the stack pointer over our current (caller) stack space.
    let new_stack_len = translator
        .builder
        .iadd_imm(translator.stack_start_offset, size_padded);
    translator.builder.store(
        new_stack_len,
        translator.ctx_ptr,
        (offset_of!(RawContext, stack_ptr) + offset_of!(RawStack, len)),
    );

    let mut args: Vec<_> = args.iter().map(|arg| translator.load(arg)).collect();
    args.insert(0, translator.ctx_ptr);

    let res = if let Some(object) = object {
        let signature = translate_function_signature(&callee);

        let object = translator.load(object);

        let fn_ptr = load_function_addr(translator, callee, object);
        translator.builder.call_indirect(signature, fn_ptr, &args)
    } else {
        let func_id = *translator.data.functions.get(&callee.id).unwrap();

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

    let is_ok = translator
        .builder
        .icmp_imm(*result, RESULT_OK, IntCompareOp::Equal);

    translator
        .builder
        .brif(is_ok, continue_block, &[], suspend_block, &[]);

    translator.builder.switch_to_block(suspend_block);
    translator.suspend(suspension_point, stack_ptr, *result);

    translator.builder.switch_to_block(continue_block);

    // Release the tag for the callee frame.
    translator.stack_free(size_of::<usize>() + padding);

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
    let class = translator.data.registry.class(callee.class).unwrap();
    let (class, func) = get_declaring_class_of_func(&class.functions, callee.class, callee.id);

    let class = translator.data.registry.class(class).unwrap();
    let func_index = class
        .functions
        .new
        .iter()
        .position(|(_, f)| *f == func)
        .unwrap();

    let mut class_index = 0;
    let mut next_parent = class.parent;
    while let Some(parent) = next_parent {
        class_index += 1;
        let class = translator.data.registry.class(parent).unwrap();
        next_parent = class.parent;
    }

    let header_ptr = translator.builder.load(JitType::Pointer, object_ptr, 0);

    // 1 usize for Class Count
    // 2 usize for each skipped Class (TypeId, *VTable)
    // 1 usize for the TypeId of our class
    let byte_class_offset =
        size_of::<usize>() + size_of::<usize>() * 2 * class_index + size_of::<usize>();

    let vtable_ptr = translator
        .builder
        .load(JitType::Pointer, header_ptr, byte_class_offset);

    let byte_func_offset = size_of::<*const u8>() * func_index;
    let fn_ptr = translator
        .builder
        .load(JitType::Pointer, vtable_ptr, byte_func_offset);

    fn_ptr
}

fn get_declaring_class_of_func(
    funcs: &FunctionSet,
    callee: ClassId,
    id: FunctionId,
) -> (ClassId, FunctionId) {
    // FIXME: Restructure the FunctionSet internals so that we don't need to loop here.
    for (_, f) in &funcs.new {
        if *f == id {
            return (callee, id);
        }
    }

    for (_, f) in &funcs.inherited {
        if f.next_parent_impl == id {
            return (f.def_class, f.def_function);
        }
    }

    for (_, f) in &funcs.overriden {
        if f.func == id {
            return (f.def_class, f.def_function);
        }
    }

    unreachable!("error resolving function vtable offset")
}
