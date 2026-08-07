//! Trampline for host to guest transitions

use openfo4_jit::{Compiler, FuncId, Signature, Type};

use crate::runtime::RawVal;

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

    let result = res[0];
    let ret_value = (res.len() > 1).then(|| res[1]);

    debug_assert_eq!(res.len(), target_signature.returns.len());

    if let Some(ret_value) = ret_value {
        builder.store(ret_value, returns_ptr, 0);
    }

    builder.return_(&[result]);
    builder.finalize();

    func_id
}
