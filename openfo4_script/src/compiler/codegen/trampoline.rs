//! Trampline for host to guest transitions

use openfo4_jit::{Compiler, FuncId, IntCompareOp, Signature, Type};

use crate::compiler::NativeFunction;
use crate::compiler::codegen::CodegenData;
use crate::compiler::codegen::trace::Span;
use crate::runtime::{RESULT_OK, RawVal};

pub fn generate_host_to_guest_trampoline(
    compiler: &mut Compiler,
    target: FuncId,
    target_signature: &Signature,
) -> FuncId {
    // fn(ctx: *const Context, params: *const RawVal, returns: *mut RawVal) -> Result
    let func_id = compiler.declare_function(Signature {
        params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
        returns: vec![Type::I32],
    });

    let mut builder = compiler.define_function(func_id);

    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx = builder.block_params(entry_block)[0];
    let params_ptr = builder.block_params(entry_block)[1];
    let returns_ptr = builder.block_params(entry_block)[2];

    let mut args = vec![ctx];
    for (index, ty) in target_signature.params.iter().skip(1).enumerate() {
        let offset = (size_of::<RawVal>() * index) as u32;
        let value = builder.load(*ty, params_ptr, offset);
        args.push(value);
    }

    let res = builder.call(target, &args);

    let ret_value = (res.len() > 1).then(|| res[0]);
    let result = if res.len() == 1 { res[0] } else { res[1] };

    debug_assert_eq!(res.len(), target_signature.returns.len());

    if let Some(ret_value) = ret_value {
        builder.store(ret_value, returns_ptr, 0);
    }

    builder.return_(&[result]);
    builder.finalize();

    func_id
}

pub fn generate_guest_to_host_trampoline(
    compiler: &mut Compiler,
    data: &CodegenData,
    target_signature: &Signature,
    native: &NativeFunction,
    class_name: &str,
    func_name: &str,
) -> FuncId {
    // fn(ctx: *const Context, func_id: usize, params: *const RawVal, returns: *mut RawVal) -> Result
    let trampoline_func_id = compiler.declare_function(Signature {
        params: vec![Type::Pointer, Type::Pointer, Type::Pointer, Type::Pointer],
        returns: vec![Type::I32],
    });

    compiler.define_native_function(trampoline_func_id, native.ptr.clone());

    let func_id = compiler.declare_function(target_signature.clone());

    let mut builder = compiler.define_function(func_id);
    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx = builder.block_params(entry_block)[0];
    let params = builder.block_params(entry_block)[1..].to_vec();

    let mut span = Span::new(data, class_name, func_name);
    span.enter(data, &mut builder, ctx);

    let stack_params =
        builder.create_stack_slot(size_of::<RawVal>() * params.len(), align_of::<RawVal>());
    let stack_returns = builder.create_stack_slot(
        size_of::<RawVal>() * (target_signature.returns.len() - 1),
        align_of::<RawVal>(),
    );

    for (index, value) in params.into_iter().enumerate() {
        let offset = (size_of::<RawVal>() * index) as u32;
        builder.stack_store(stack_params, value, offset);
    }

    let index = builder.iconst(Type::Isize, i64::try_from(native.index).unwrap());

    let stack_params_addr = builder.stack_addr(stack_params, 0);
    let stack_returns_addr = builder.stack_addr(stack_returns, 0);

    let args = vec![ctx, index, stack_params_addr, stack_returns_addr];

    let result = builder.call(trampoline_func_id, &args)[0];

    let mut returns = Vec::new();
    if target_signature.returns.len() > 1 {
        let ty = target_signature.returns[0];

        let result_ok_block = builder.create_block(&[]);
        let result_err_block = builder.create_block(&[]);
        let merge_block = builder.create_block(&[ty]);

        let is_ok = builder.icmp_imm(result, RESULT_OK, IntCompareOp::Equal);
        builder.brif(is_ok, result_ok_block, &[], result_err_block, &[]);

        builder.switch_to_block(result_ok_block);
        let ty = target_signature.returns[0];
        let value = builder.stack_load(stack_returns, ty, 0);
        builder.jump(merge_block, &[value]);

        builder.switch_to_block(result_err_block);
        let value = builder.uninit(ty);
        builder.jump(merge_block, &[value]);

        builder.switch_to_block(merge_block);
        let value = builder.block_params(merge_block)[0];

        returns.push(value);
    }

    returns.push(result);

    span.exit(data, &mut builder, ctx);

    builder.return_(&returns);
    builder.finalize();

    func_id
}
