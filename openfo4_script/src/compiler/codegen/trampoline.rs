//! Trampline for host to guest transitions

use openfo4_jit::{Compiler, FuncId, Signature, Type};

use crate::compiler::NativeFunction;
use crate::compiler::codegen::CodegenData;
use crate::compiler::codegen::trace::Span;
use crate::runtime::RawVal;

pub fn generate_host_to_guest_trampoline(
    compiler: &mut Compiler,
    target: FuncId,
    target_signature: &Signature,
) -> FuncId {
    // fn(*const Context, usize, *mut RawVal)
    let func_id = compiler.declare_function(Signature {
        params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
        returns: Vec::new(),
    });

    let mut builder = compiler.define_function(func_id);

    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx = builder.block_params(entry_block)[0];
    let ptr = builder.block_params(entry_block)[2];

    let mut args = vec![ctx];
    let mut offset = 0;
    for ty in target_signature.params.iter().skip(1) {
        let value = builder.load(*ty, ptr, offset);
        args.push(value);

        offset += size_of::<RawVal>() as u32;
    }

    let res = builder.call(target, &args);

    debug_assert_eq!(res.len(), target_signature.returns.len());
    for value in res {
        builder.store(value, ptr, offset);
        offset += size_of::<RawVal>() as u32;
    }

    builder.return_(&[]);
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
    // fn(*const Context, usize, *mut RawVal)
    let trampoline_func_id = compiler.declare_function(Signature {
        params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
        returns: Vec::new(),
    });

    compiler.define_native_function(trampoline_func_id, native.ptr);

    let func_id = compiler.declare_function(target_signature.clone());

    let mut builder = compiler.define_function(func_id);
    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx = builder.block_params(entry_block)[0];
    let params = builder.block_params(entry_block)[1..].to_vec();

    let mut span = Span::new(data, class_name, func_name);
    span.enter(data, &mut builder, ctx);

    let stack_slots = params.len() + target_signature.returns.len();
    let stack = builder.create_stack_slot(size_of::<RawVal>() * stack_slots, align_of::<RawVal>());

    let mut offset = 0;
    for value in params {
        builder.stack_store(stack, value, offset);
        offset += size_of::<RawVal>() as u32;
    }

    let index = builder.iconst(Type::Pointer, i64::try_from(native.index).unwrap());
    let addr = builder.stack_addr(stack, Type::Pointer, 0);

    let args = vec![ctx, index, addr];

    builder.call(trampoline_func_id, &args);

    let mut returns = Vec::new();
    for ty in target_signature.returns.iter() {
        let value = builder.stack_load(stack, *ty, offset);
        returns.push(value);
        offset += size_of::<RawVal>() as u32;
    }

    span.exit(data, &mut builder, ctx);

    builder.return_(&returns);
    builder.finalize();

    func_id
}
