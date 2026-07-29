use cranelift_codegen::ir::{
    self, ExtFuncData, ExternalName, InstBuilder, StackSlotData, StackSlotKind, UserExternalName,
    UserExternalNameRef,
};
use cranelift_frontend::FunctionBuilder;
use hashbrown::HashMap;

use crate::backend::cranelift::{Callee, FuncState, HostFunctionDefinition, map_type};
use crate::func::{HostFunc, MAX_ALIGN};

pub unsafe extern "C" fn host_call_trampoline(
    func: *const HostFunc,
    params: *const u8,
    returns: *mut u8,
) {
    unsafe {
        let func = &*func;
        func.call(params, returns);
    }
}

/// Generates the code to call the host trampoline.
pub fn call_host_trampoline(
    builder: &mut FunctionBuilder<'_>,
    ptr_type: ir::Type,
    shim_signature: &ir::Signature,
    func: &FuncState,
    def: &HostFunctionDefinition,
    params: &[ir::Value],
    callees: &mut HashMap<UserExternalNameRef, Callee>,
) -> Vec<ir::Value> {
    let mut params_size = 0;
    for ty in &func.signature.params {
        params_size = (params_size + ty.align() - 1) & !(ty.align() - 1);
        params_size += ty.size();
    }

    let mut returns_size = 0;
    for ty in &func.signature.returns {
        returns_size = (returns_size + ty.align() - 1) & !(ty.align() - 1);
        returns_size += ty.size();
    }

    let params_slot = builder.create_sized_stack_slot(StackSlotData {
        kind: StackSlotKind::ExplicitSlot,
        size: params_size as u32,
        align_shift: MAX_ALIGN.ilog2() as u8,
        key: None,
    });
    let returns_slot = builder.create_sized_stack_slot(StackSlotData {
        kind: StackSlotKind::ExplicitSlot,
        size: returns_size as u32,
        align_shift: MAX_ALIGN.ilog2() as u8,
        key: None,
    });

    {
        let mut offset = 0;
        for (ty, value) in func.signature.params.iter().zip(params) {
            offset = (offset + ty.align() - 1) & !(ty.align() - 1);
            builder
                .ins()
                .stack_store(*value, params_slot, offset as i32);
            offset += ty.size();
        }
    }

    let name = builder
        .func
        .declare_imported_user_function(UserExternalName::new(1, 0));
    let sig_ref = builder.func.import_signature(shim_signature.clone());
    let func_ref = builder.import_function(ExtFuncData {
        name: ExternalName::User(name),
        signature: sig_ref,
        colocated: false,
        patchable: false,
    });

    callees.insert(name, Callee::HostCallTrampoline);

    let ptr = builder.ins().iconst(ptr_type, def.data.addr() as i64);
    let params = builder.ins().stack_addr(ptr_type, params_slot, 0);
    let returns = builder.ins().stack_addr(ptr_type, returns_slot, 0);

    builder.ins().call(func_ref, &[ptr, params, returns]);

    let mut offset = 0;
    let mut return_values = Vec::new();
    for ty in &func.signature.returns {
        offset = (offset + ty.align() - 1) & !(ty.align() - 1);

        let ir_ty = map_type(ptr_type, *ty);
        let value = builder.ins().stack_load(ir_ty, returns_slot, offset as i32);
        return_values.push(value);

        offset += ty.size();
    }

    return_values
}
