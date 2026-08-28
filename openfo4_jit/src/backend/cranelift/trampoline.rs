use cranelift_codegen::ir::{
    self, ExtFuncData, ExternalName, InstBuilder, MemFlags, StackSlotData, StackSlotKind,
    UserExternalName, UserExternalNameRef,
};
use cranelift_frontend::FunctionBuilder;
use hashbrown::HashMap;

use crate::FuncId;
use crate::backend::cranelift::{Callee, FuncState, HostFunctionDefinition, Translator, map_type};
use crate::func::HostFunc;

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
    translator: &mut Translator,
) -> Vec<ir::Value> {
    let mut params_size = 0;
    let mut params_align = 1;
    for ty in &func.signature.params {
        params_size = (params_size + ty.align() - 1) & !(ty.align() - 1);
        params_size += ty.size();
        params_align = params_align.max(ty.align());
    }

    let mut returns_size = 0;
    let mut returns_align = 1;
    for ty in &func.signature.returns {
        returns_size = (returns_size + ty.align() - 1) & !(ty.align() - 1);
        returns_size += ty.size();
        returns_align = returns_align.max(ty.align());
    }

    debug_assert!(params_align.is_power_of_two());
    debug_assert!(returns_align.is_power_of_two());
    let params_slot = builder.create_sized_stack_slot(StackSlotData {
        kind: StackSlotKind::ExplicitSlot,
        size: params_size as u32,
        // Alignment always is a power of two, so this is the same as ilog2.
        align_shift: params_align.trailing_zeros() as u8,
        key: None,
    });
    let returns_slot = builder.create_sized_stack_slot(StackSlotData {
        kind: StackSlotKind::ExplicitSlot,
        size: returns_size as u32,
        // Alignment always is a power of two, so this is the same as ilog2.
        align_shift: returns_align.trailing_zeros() as u8,
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

    let func_ref = *translator.host_call_trampoline.get_or_insert_with(|| {
        let name = builder
            .func
            .declare_imported_user_function(UserExternalName::new(1, 0));

        callees.insert(name, Callee::HostCallTrampoline);

        let sig_ref = *translator
            .signatures
            .entry(shim_signature.clone())
            .or_insert_with(|| builder.func.import_signature(shim_signature.clone()));
        builder.import_function(ExtFuncData {
            name: ExternalName::User(name),
            signature: sig_ref,
            colocated: false,
            patchable: false,
        })
    });

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

pub fn call_guest_trampoline(
    builder: &mut FunctionBuilder<'_>,
    ptr_type: ir::Type,
    func: &FuncState,
    func_id: FuncId,
    callee_signature: &ir::Signature,
    callees: &mut HashMap<UserExternalNameRef, Callee>,
) {
    let block = builder.create_block();
    builder.switch_to_block(block);
    builder.append_block_params_for_function_params(block);
    let args = builder.block_params(block);

    let params_ptr = args[0];
    let returns_ptr = args[1];

    let mut offset = 0;
    let mut params = Vec::with_capacity(func.signature.params.len());
    for ty in &func.signature.params {
        offset = (offset + ty.align() - 1) & !(ty.align() - 1);
        let value = builder.ins().load(
            map_type(ptr_type, *ty),
            MemFlags::trusted(),
            params_ptr,
            offset as i32,
        );
        params.push(value);
        offset += ty.size();
    }

    let name = builder
        .func
        .declare_imported_user_function(UserExternalName::new(0, func_id.0.try_into().unwrap()));
    callees.insert(name, Callee::Id(func_id));

    let sig_ref = builder.import_signature(callee_signature.clone());
    let func_ref = builder.import_function(ExtFuncData {
        name: ExternalName::User(name),
        signature: sig_ref,
        colocated: false,
        patchable: false,
    });

    let inst = builder.ins().call(func_ref, &params);
    let returns = builder.inst_results(inst).to_vec();

    debug_assert_eq!(func.signature.returns.len(), returns.len());
    let mut offset = 0;
    for (ty, value) in func.signature.returns.iter().zip(returns) {
        offset = (offset + ty.align() - 1) & !(ty.align() - 1);
        builder
            .ins()
            .store(MemFlags::trusted(), value, returns_ptr, offset as i32);
        offset += ty.size();
    }

    builder.ins().return_(&[]);
}
