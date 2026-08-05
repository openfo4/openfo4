mod debug;
mod libcall;
mod memory;
mod relocs;
mod trampoline;

use std::alloc::Layout;
use std::mem::transmute;
use std::sync::Arc;

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    self, ArgumentExtension, ArgumentPurpose, BlockArg, ExtFuncData, ExternalName, Function,
    InstBuilder, MemFlags, StackSlotData, StackSlotKind, UserExternalName, UserExternalNameRef,
    UserFuncName,
};
use cranelift_codegen::isa::{CallConv, OwnedTargetIsa};
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{CompiledCode, Context, settings};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use slab::Slab;

use crate::backend::cranelift::memory::{Allocation, MemoryAllocator};
use crate::backend::cranelift::relocs::perform_relocations;
use crate::backend::{
    Backend, FunctionDefinitionDesc, SourceLocMap, StackSlotDesc, TextWithSrcLocs,
};
use crate::func::HostFunc;
use crate::{
    BlockProps, CompilerConfig, FloatCompareOp, FuncId, Instruction, IntCompareOp, Signature,
    SourceLoc, Type,
};

pub struct CraneliftModule {
    functions: Slab<FuncState>,
    compiled_functions: HashMap<usize, CompiledFunction>,
    allocator: MemoryAllocator,
    isa: OwnedTargetIsa,
    ctx: Context,
    func_ctx: FunctionBuilderContext,
    debug: bool,
    call_host_trampoline_signature: ir::Signature,
}

impl Backend for CraneliftModule {
    type CompiledFunc = CompiledFunction;

    fn new(config: CompilerConfig) -> Self {
        let mut flags = settings::builder();
        flags.set("use_colocated_libcalls", "false").unwrap();
        flags.set("is_pic", "false").unwrap();
        flags.set("opt_level", "speed_and_size").unwrap();
        flags.set("regalloc_algorithm", "backtracking").unwrap();

        if config.validation {
            flags.set("enable_verifier", "true").unwrap();
        } else {
            flags.set("enable_verifier", "false").unwrap();
        }

        let builder = cranelift_native::builder().unwrap();
        let isa = builder.finish(settings::Flags::new(flags)).unwrap();

        let call_host_trampoline_signature = ir::Signature {
            params: vec![
                ir::AbiParam {
                    value_type: isa.pointer_type(),
                    purpose: ArgumentPurpose::Normal,
                    extension: ArgumentExtension::None,
                },
                ir::AbiParam {
                    value_type: isa.pointer_type(),
                    purpose: ArgumentPurpose::Normal,
                    extension: ArgumentExtension::None,
                },
                ir::AbiParam {
                    value_type: isa.pointer_type(),
                    purpose: ArgumentPurpose::Normal,
                    extension: ArgumentExtension::None,
                },
            ],
            returns: Vec::new(),
            call_conv: isa.default_call_conv(),
        };

        Self {
            functions: Slab::new(),
            compiled_functions: HashMap::new(),
            allocator: MemoryAllocator::new(),
            ctx: Context::new(),
            isa,
            func_ctx: FunctionBuilderContext::new(),
            debug: config.debug,
            call_host_trampoline_signature,
        }
    }

    fn declare_function(&mut self, signature: &Signature) -> FuncId {
        let id = self.functions.insert(FuncState {
            signature: signature.clone(),
            definition: None,
            compiled: None,
            trampoline: None,
        });

        FuncId(id)
    }

    fn define_host_function(&mut self, id: FuncId, func: HostFunc) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(state.definition.is_none(), "cannot redefine function");

        let data = Box::into_raw(Box::new(func));

        state.definition = Some(FunctionDefinition::Host(HostFunctionDefinition { data }));
    }

    fn define_function(&mut self, id: FuncId, definition: FunctionDefinitionDesc) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(state.definition.is_none(), "cannot redefine function");

        state.definition = Some(FunctionDefinition::Guest(GuestFunctionDefinition {
            instructions: definition.instructions,
            num_values: definition.num_values,
            stack_slots: definition.stack_slots,
            num_variables: definition.num_variables,
            srcloc: definition.srcloc,
            blocks: definition.blocks,
        }));
    }

    fn compile_function(&mut self, id: FuncId) -> Self::CompiledFunc {
        let _span = trace_span!("CraneliftModule::compile_function").entered();

        if let Some(func) = self.compiled_functions.get(&id.0) {
            return func.clone();
        }

        let ptr_type = self.isa.pointer_type();
        let map_type = |ty: Type| -> ir::Type {
            match ty {
                Type::I8 => ir::types::I8,
                Type::I16 => ir::types::I16,
                Type::I32 => ir::types::I32,
                Type::I64 => ir::types::I64,
                Type::F32 => ir::types::F32,
                Type::F64 => ir::types::F64,
                Type::Pointer => ptr_type,
            }
        };

        let convert_signature = |sig: &Signature| -> ir::Signature {
            let map_param = |ty: &Type| -> ir::AbiParam {
                let ty = map_type(*ty);
                ir::AbiParam {
                    value_type: ty,
                    purpose: ArgumentPurpose::Normal,
                    extension: ArgumentExtension::None,
                }
            };

            let params = sig.params.iter().map(map_param).collect();
            let returns = sig.returns.iter().map(map_param).collect();

            ir::Signature {
                params,
                returns,
                call_conv: CallConv::Fast,
            }
        };

        let mut funcs_to_compile = vec![id];

        if self.functions[id.0].trampoline.is_none() {
            let trampoline_id = FuncId(self.functions.insert(FuncState {
                signature: Signature {
                    params: vec![Type::Pointer, Type::Pointer],
                    returns: Vec::new(),
                },
                definition: Some(FunctionDefinition::GuestTrampoline(id)),
                compiled: None,
                trampoline: None,
            }));

            self.functions[id.0].trampoline = Some(trampoline_id);
            funcs_to_compile.push(trampoline_id);
        }

        let mut compiled_funcs = HashSet::new();

        while let Some(id) = funcs_to_compile.pop() {
            let func_def = &self.functions[id.0];
            if func_def.compiled.is_some() {
                continue;
            }

            // Note that FuncRefs are only valid for a single function,
            // so we must recreate them for each function.
            let mut callees = HashMap::new();
            let mut refs_to_funcs = HashMap::new();

            self.ctx.clear();

            let signature = convert_signature(&func_def.signature);

            let mut func = Function::with_name_signature(
                UserFuncName::User(UserExternalName::new(0, id.0.try_into().unwrap())),
                signature,
            );

            let mut builder = FunctionBuilder::new(&mut func, &mut self.func_ctx);

            let mut translator = Translator::default();

            match func_def.definition.as_ref().unwrap() {
                FunctionDefinition::Host(def) => {
                    let block = builder.create_block();
                    builder.append_block_params_for_function_params(block);
                    let params = builder.block_params(block).to_vec();

                    let returns = trampoline::call_host_trampoline(
                        &mut builder,
                        ptr_type,
                        &self.call_host_trampoline_signature,
                        func_def,
                        def,
                        &params,
                        &mut refs_to_funcs,
                        &mut translator,
                    );

                    builder.ins().return_(&returns);
                }
                FunctionDefinition::GuestTrampoline(callee) => {
                    let f = &self.functions[callee.0];
                    let mut signature = convert_signature(&f.signature);
                    signature.call_conv = self.isa.default_call_conv();

                    trampoline::call_guest_trampoline(
                        &mut builder,
                        ptr_type,
                        f,
                        *callee,
                        &signature,
                        &mut refs_to_funcs,
                    );
                }
                FunctionDefinition::Guest(func_def) => {
                    let mut values = vec![const { None }; func_def.num_values];
                    let mut blocks = vec![const { None }; func_def.blocks.len()];
                    let mut stack_slots = vec![const { None }; func_def.stack_slots.len()];
                    let mut variables = vec![const { None }; func_def.num_variables];

                    // Create stack slots
                    for (id, slot) in &func_def.stack_slots {
                        let res = builder.create_sized_stack_slot(StackSlotData {
                            kind: StackSlotKind::ExplicitSlot,
                            size: slot.size.try_into().unwrap(),
                            // `slot.align` already must be a power of two.
                            // FIXME: we can do better than ilog2.
                            align_shift: slot.align.ilog2() as u8,
                            key: None,
                        });

                        stack_slots[id] = Some(res);
                    }

                    for block in &func_def.blocks {
                        let b = builder.create_block();

                        if block.is_entry {
                            builder.append_block_params_for_function_params(b);
                            builder.switch_to_block(b);
                            builder.seal_block(b);
                        } else {
                            for ty in &block.types {
                                builder.append_block_param(b, map_type(*ty));
                            }
                        }

                        let params = builder.block_params(b);
                        debug_assert_eq!(params.len(), block.values.len());
                        for (param, value) in params.iter().zip(&block.values) {
                            values[value.0] = Some(*param);
                        }

                        blocks[block.id.0] = Some(b);
                    }

                    let mut current_block = None;

                    for (index, instruction) in func_def.instructions.iter().enumerate() {
                        let current_block_on_entry = current_block;
                        let mut added_inst_count = 0;

                        match instruction {
                            Instruction::SwitchBlock(block) => {
                                let block = blocks[block.0].unwrap();
                                builder.switch_to_block(block);
                                current_block = Some(block);
                            }
                            Instruction::IConst(ty, value, dst) => {
                                let res = builder.ins().iconst(map_type(*ty), i64::from(*value));
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FConst(ty, value, dst) => {
                                let res = match ty {
                                    Type::F32 => builder.ins().f32const(*value as f32),
                                    Type::F64 => builder.ins().f64const(*value),
                                    _ => unreachable!(),
                                };

                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::IAdd(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().iadd(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::ISub(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().isub(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::IMul(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().imul(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::UDiv(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().udiv(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SDiv(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().sdiv(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::URem(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().urem(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SRem(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().srem(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FAdd(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().fadd(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FSub(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().fsub(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FMul(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().fmul(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FDiv(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().fdiv(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FMin(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().fmin(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FMax(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().fmax(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::INeg(op) => {
                                let src = values[op.src.0].unwrap();

                                let res = builder.ins().ineg(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FNeg(op) => {
                                let src = values[op.src.0].unwrap();

                                let res = builder.ins().fneg(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FAbs(op) => {
                                let src = values[op.src.0].unwrap();

                                let res = builder.ins().fabs(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::Floor(op) => {
                                let src = values[op.src.0].unwrap();

                                let res = builder.ins().floor(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::Ceil(op) => {
                                let src = values[op.src.0].unwrap();

                                let res = builder.ins().ceil(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::CallIndirect(signature, callee, params, returns) => {
                                let args = params
                                    .iter()
                                    .map(|v| values[v.0].unwrap())
                                    .collect::<Vec<_>>();

                                let signature = convert_signature(signature);
                                let sig_ref = *translator
                                    .signatures
                                    .entry(signature.clone())
                                    .or_insert_with(|| builder.import_signature(signature));

                                let callee = values[callee.0].unwrap();

                                let inst = builder.ins().call_indirect(sig_ref, callee, &args);
                                added_inst_count += 1;

                                let return_values = builder.inst_results(inst);
                                debug_assert_eq!(returns.len(), return_values.len());

                                for (lhs, rhs) in returns.iter().zip(return_values) {
                                    values[lhs.0] = Some(*rhs);
                                }
                            }
                            Instruction::Call(func_id, params, returns) => {
                                let args = params
                                    .iter()
                                    .map(|v| values[v.0].unwrap())
                                    .collect::<Vec<_>>();

                                let func = &self.functions[func_id.0];

                                let return_values = match &func.definition {
                                    Some(FunctionDefinition::Guest(_)) => {
                                        if func.compiled.is_none() {
                                            funcs_to_compile.push(*func_id);
                                        }

                                        let func_ref =
                                            *callees.entry(*func_id).or_insert_with(|| {
                                                let name =
                                                    builder.func.declare_imported_user_function(
                                                        UserExternalName::new(
                                                            0,
                                                            func_id.0.try_into().unwrap(),
                                                        ),
                                                    );

                                                refs_to_funcs.insert(name, Callee::Id(*func_id));

                                                let signature = convert_signature(&func.signature);
                                                let sig_ref = *translator
                                                    .signatures
                                                    .entry(signature.clone())
                                                    .or_insert_with(|| {
                                                        builder.import_signature(signature)
                                                    });

                                                builder.import_function(ExtFuncData {
                                                    name: ExternalName::User(name),
                                                    signature: sig_ref,
                                                    colocated: false,
                                                    patchable: false,
                                                })
                                            });

                                        let inst = builder.ins().call(func_ref, &args);
                                        let return_values = builder.inst_results(inst);
                                        return_values.to_vec()
                                    }
                                    Some(FunctionDefinition::Host(def)) => {
                                        // All host functions need to go through the host-call trampoline
                                        // first. We generally benefit from inlining this directly since
                                        // the trampoline only unpacks and repacks stack memory.
                                        trampoline::call_host_trampoline(
                                            &mut builder,
                                            ptr_type,
                                            &self.call_host_trampoline_signature,
                                            func,
                                            def,
                                            &args,
                                            &mut refs_to_funcs,
                                            &mut translator,
                                        )
                                    }
                                    _ => unreachable!(),
                                };

                                added_inst_count += 1;

                                debug_assert_eq!(returns.len(), return_values.len());

                                for (lhs, rhs) in returns.iter().zip(return_values) {
                                    values[lhs.0] = Some(rhs);
                                }
                            }
                            Instruction::Return(rvals) => {
                                let rvals = rvals
                                    .into_iter()
                                    .map(|v| values[v.0].unwrap())
                                    .collect::<Vec<_>>();

                                builder.ins().return_(&rvals);
                                added_inst_count += 1;
                            }
                            Instruction::Jump(block, block_args) => {
                                let block = blocks[block.0].unwrap();

                                let args = block_args
                                    .iter()
                                    .map(|v| BlockArg::Value(values[v.0].unwrap()))
                                    .collect::<Vec<_>>();

                                builder.ins().jump(block, &args);
                                added_inst_count += 1;
                            }
                            Instruction::Brif {
                                condition,
                                true_block,
                                true_block_args,
                                false_block,
                                false_block_args,
                            } => {
                                let condition = values[condition.0].unwrap();
                                let then_block = blocks[true_block.0].unwrap();
                                let else_block = blocks[false_block.0].unwrap();

                                let then_block_args = true_block_args
                                    .iter()
                                    .map(|v| BlockArg::Value(values[v.0].unwrap()))
                                    .collect::<Vec<_>>();
                                let else_block_args = false_block_args
                                    .iter()
                                    .map(|v| BlockArg::Value(values[v.0].unwrap()))
                                    .collect::<Vec<_>>();

                                builder.ins().brif(
                                    condition,
                                    then_block,
                                    &then_block_args,
                                    else_block,
                                    &else_block_args,
                                );
                                added_inst_count += 1;
                            }
                            Instruction::StackLoad {
                                slot,
                                dst,
                                ty,
                                offset,
                            } => {
                                let slot = stack_slots[slot.0].unwrap();
                                let ty = map_type(*ty);

                                // TODO: We can do manual address calcuation when offset is too big.
                                let offset = i32::try_from(*offset).unwrap_or_else(|_| todo!());

                                let res = builder.ins().stack_load(ty, slot, offset);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::StackStore {
                                slot,
                                value,
                                offset,
                            } => {
                                let value = values[value.0].unwrap();
                                let slot = stack_slots[slot.0].unwrap();

                                // TODO: We can do manual address calcuation when offset is too big.
                                let offset = i32::try_from(*offset).unwrap_or_else(|_| todo!());

                                builder.ins().stack_store(value, slot, offset);
                                added_inst_count += 1;
                            }
                            Instruction::StackAddr { slot, dst, offset } => {
                                let slot = stack_slots[slot.0].unwrap();
                                let ty = map_type(Type::Pointer);

                                // TODO: We can do manual address calcuation when offset is too big.
                                let offset = i32::try_from(*offset).unwrap_or_else(|_| todo!());

                                let res = builder.ins().stack_addr(ty, slot, offset);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::ICmp { lhs, rhs, dst, op } => {
                                let lhs = values[lhs.0].unwrap();
                                let rhs = values[rhs.0].unwrap();

                                let cond = match op {
                                    IntCompareOp::Equal => IntCC::Equal,
                                    IntCompareOp::NotEqual => IntCC::NotEqual,
                                    IntCompareOp::SignedGreaterThan => IntCC::SignedGreaterThan,
                                    IntCompareOp::SignedGreaterThanOrEqual => {
                                        IntCC::SignedGreaterThanOrEqual
                                    }
                                    IntCompareOp::SignedLessThan => IntCC::SignedLessThan,
                                    IntCompareOp::SignedLessThanOrEqual => {
                                        IntCC::SignedLessThanOrEqual
                                    }
                                    IntCompareOp::UnsignedGreaterThan => IntCC::UnsignedGreaterThan,
                                    IntCompareOp::UnsignedGreaterThanOrEqual => {
                                        IntCC::UnsignedGreaterThanOrEqual
                                    }
                                    IntCompareOp::UnsignedLessThan => IntCC::UnsignedLessThan,
                                    IntCompareOp::UnsignedLessThanOrEqual => {
                                        IntCC::UnsignedLessThanOrEqual
                                    }
                                };

                                let res = builder.ins().icmp(cond, lhs, rhs);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FCmp { lhs, rhs, dst, op } => {
                                let lhs = values[lhs.0].unwrap();
                                let rhs = values[rhs.0].unwrap();

                                let cond = match op {
                                    FloatCompareOp::Equal => FloatCC::Equal,
                                    FloatCompareOp::NotEqual => FloatCC::NotEqual,
                                    FloatCompareOp::LessThan => FloatCC::LessThan,
                                    FloatCompareOp::LessThanOrEqual => FloatCC::LessThanOrEqual,
                                    FloatCompareOp::GreaterThan => FloatCC::GreaterThan,
                                    FloatCompareOp::GreaterThanOrEqual => {
                                        FloatCC::GreaterThanOrEqual
                                    }
                                };

                                let res = builder.ins().fcmp(cond, lhs, rhs);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::Load {
                                ty,
                                addr,
                                dst,
                                offset,
                            } => {
                                let ty = map_type(*ty);
                                let addr = values[addr.0].unwrap();

                                // TODO: We can do manual address calcuation when offset is too big.
                                let offset = i32::try_from(*offset).unwrap_or_else(|_| todo!());

                                let res = builder.ins().load(ty, MemFlags::trusted(), addr, offset);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::Store {
                                value,
                                addr,
                                offset,
                            } => {
                                let value = values[value.0].unwrap();
                                let addr = values[addr.0].unwrap();

                                // TODO: We can do manual address calcuation when offset is too big.
                                let offset = i32::try_from(*offset).unwrap_or_else(|_| todo!());

                                builder
                                    .ins()
                                    .store(MemFlags::trusted(), value, addr, offset);
                                added_inst_count += 1;
                            }
                            Instruction::FToU { value, ty, dst } => {
                                let value = values[value.0].unwrap();
                                let ty = map_type(*ty);

                                let res = builder.ins().fcvt_to_uint_sat(ty, value);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::FToS { value, ty, dst } => {
                                let value = values[value.0].unwrap();
                                let ty = map_type(*ty);

                                let res = builder.ins().fcvt_to_sint_sat(ty, value);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::UToF { value, ty, dst } => {
                                let value = values[value.0].unwrap();
                                let ty = map_type(*ty);

                                let res = builder.ins().fcvt_from_uint(ty, value);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SToF { value, ty, dst } => {
                                let value = values[value.0].unwrap();
                                let ty = map_type(*ty);

                                let res = builder.ins().fcvt_from_sint(ty, value);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::BAnd(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().band(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::BOr(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().bor(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::BXor(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();

                                let res = builder.ins().bxor(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::BNot(op) => {
                                let src = values[op.src.0].unwrap();
                                let res = builder.ins().bnot(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::UExtend { value, dst, ty } => {
                                let src = values[value.0].unwrap();
                                let res = builder.ins().uextend(map_type(*ty), src);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SExtend { value, dst, ty } => {
                                let src = values[value.0].unwrap();
                                let res = builder.ins().sextend(map_type(*ty), src);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::IReduce { value, dst, ty } => {
                                let src = values[value.0].unwrap();
                                let res = builder.ins().ireduce(map_type(*ty), src);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::Bitcast { value, dst, ty } => {
                                let src = values[value.0].unwrap();
                                let ty = map_type(*ty);
                                let res = builder.ins().bitcast(ty, MemFlags::new(), src);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::Popcnt(op) => {
                                let src = values[op.src.0].unwrap();
                                let res = builder.ins().popcnt(src);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::DefVar { ty, var, value } => {
                                let res = builder.declare_var(map_type(*ty));
                                let value = values[value.0].unwrap();
                                builder.def_var(res, value);
                                variables[var.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::GetVar { var, dst } => {
                                let var = variables[var.0].unwrap();
                                let res = builder.use_var(var);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SetVar { var, value } => {
                                let var = variables[var.0].unwrap();
                                let value = values[value.0].unwrap();
                                builder.def_var(var, value);
                                added_inst_count += 1;
                            }
                            Instruction::Memcpy { src, dst, size } => {
                                let src = values[src.0].unwrap();
                                let dst = values[dst.0].unwrap();
                                let size = values[size.0].unwrap();
                                builder.call_memcpy(self.isa.frontend_config(), dst, src, size);
                                added_inst_count += 1;
                            }
                            Instruction::Memcmp {
                                lhs,
                                rhs,
                                dst,
                                size,
                            } => {
                                let lhs = values[lhs.0].unwrap();
                                let rhs = values[rhs.0].unwrap();
                                let size = values[size.0].unwrap();
                                let res =
                                    builder.call_memcmp(self.isa.frontend_config(), lhs, rhs, size);
                                values[dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SMin(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();
                                let res = builder.ins().smin(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::SMax(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();
                                let res = builder.ins().smax(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::UMin(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();
                                let res = builder.ins().umin(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                            Instruction::UMax(op) => {
                                let lhs = values[op.lhs.0].unwrap();
                                let rhs = values[op.rhs.0].unwrap();
                                let res = builder.ins().umax(lhs, rhs);
                                values[op.dst.0] = Some(res);
                                added_inst_count += 1;
                            }
                        }

                        if self.debug
                            && let Some(block) = current_block_on_entry
                            && added_inst_count != 0
                            && let Some(srcloc) = func_def.srcloc.get(index)
                        {
                            let insts: Vec<_> = builder
                                .func
                                .layout
                                .block_insts(block)
                                .rev()
                                .take(added_inst_count)
                                .collect();

                            for inst in insts {
                                builder.func.set_srcloc(inst, ir::SourceLoc::new(srcloc.0));
                            }
                        }
                    }
                }
            }

            builder.seal_all_blocks();

            let clif = self
                .debug
                .then(|| builder.func.display().to_string())
                .unwrap_or_default();

            builder.finalize();

            self.ctx.clear();
            self.ctx.func = func;

            self.ctx
                .compile(&*self.isa, &mut ControlPlane::default())
                .unwrap();
            let code = self.ctx.take_compiled_code().unwrap();

            let mut text = None;
            if self.debug {
                text = Some(debug::format_clif(&clif));
            }

            let mut final_srclocs = SourceLocMap::default();
            if self.debug {
                for loc in code.buffer.get_srclocs_sorted() {
                    if !loc.loc.is_default() {
                        let bits = loc.loc.bits();
                        final_srclocs.insert(loc.start as usize, loc.end as usize, SourceLoc(bits));
                    }
                }
            }

            let layout =
                Layout::from_size_align(code.buffer.data().len(), code.buffer.alignment as usize)
                    .unwrap();

            let allocation = self.allocator.alloc(layout);

            let func = &mut self.functions[id.0];
            func.compiled = Some(CompilationResult {
                code,
                allocation: Arc::new(allocation),
                callees: refs_to_funcs,
                clif: text,
                srclocs: final_srclocs,
            });

            compiled_funcs.insert(id);
        }

        for id in compiled_funcs {
            perform_relocations(id, &mut self.functions);
        }

        wasmtime_internal_jit_icache_coherence::pipeline_flush_mt().unwrap();

        let func = &self.functions[id.0];
        let compiled = func.compiled.as_ref().unwrap();
        let trampoline = self.functions[func.trampoline.unwrap().0]
            .compiled
            .as_ref()
            .unwrap();

        CompiledFunction {
            _allocation: compiled.allocation.clone(),
            fn_ptr: compiled.allocation.as_exec_ptr(),
            clif: compiled.clif.clone(),
            srclocs: compiled.srclocs.clone(),
            len: compiled.code.code_buffer().len(),
            _trampoline_allocation: trampoline.allocation.clone(),
            trampoline_fn_ptr: trampoline.allocation.as_exec_ptr(),
        }
    }
}

struct CompilationResult {
    code: CompiledCode,
    allocation: Arc<Allocation>,
    callees: HashMap<UserExternalNameRef, Callee>,
    clif: Option<TextWithSrcLocs<String>>,
    srclocs: SourceLocMap,
}

#[derive(Copy, Clone, Debug)]
enum Callee {
    Id(FuncId),
    HostCallTrampoline,
}

struct FuncState {
    signature: Signature,
    definition: Option<FunctionDefinition>,
    compiled: Option<CompilationResult>,
    trampoline: Option<FuncId>,
}

#[derive(Clone, Debug)]
enum FunctionDefinition {
    Host(HostFunctionDefinition),
    Guest(GuestFunctionDefinition),
    GuestTrampoline(FuncId),
}

#[derive(Clone, Debug)]
struct HostFunctionDefinition {
    /// The `data` value to pass to the host shim.
    data: *const HostFunc,
}

unsafe impl Send for HostFunctionDefinition {}
unsafe impl Sync for HostFunctionDefinition {}

#[derive(Clone, Debug)]
struct GuestFunctionDefinition {
    instructions: Vec<Instruction>,
    num_values: usize,
    num_variables: usize,
    stack_slots: Slab<StackSlotDesc>,
    srcloc: SourceLocMap,
    blocks: Vec<BlockProps>,
}

#[derive(Clone)]
pub struct CompiledFunction {
    /// The allocation must not be droped until all function calls terminate.
    _allocation: Arc<Allocation>,
    _trampoline_allocation: Arc<Allocation>,
    fn_ptr: *const u8,
    trampoline_fn_ptr: *const u8,
    clif: Option<TextWithSrcLocs<String>>,
    srclocs: SourceLocMap,
    len: usize,
}

impl CompiledFunction {
    pub unsafe fn call(&self, params: *const u8, returns: *mut u8) {
        unsafe {
            let fn_ptr = transmute::<*const u8, unsafe extern "C" fn(*const u8, *mut u8)>(
                self.trampoline_fn_ptr,
            );
            fn_ptr(params, returns);
        }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.fn_ptr
    }

    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let len = self.len;
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }

    pub fn clif(&self) -> Option<&TextWithSrcLocs<String>> {
        self.clif.as_ref()
    }

    pub fn srclocs(&self) -> &SourceLocMap {
        &self.srclocs
    }
}

// SAFETY: We carry a function pointer which is always read-only
// and can be called from any thread.
unsafe impl Send for CompiledFunction {}
unsafe impl Sync for CompiledFunction {}

fn map_type(ptr_type: ir::Type, ty: Type) -> ir::Type {
    match ty {
        Type::I8 => ir::types::I8,
        Type::I16 => ir::types::I16,
        Type::I32 => ir::types::I32,
        Type::I64 => ir::types::I64,
        Type::F32 => ir::types::F32,
        Type::F64 => ir::types::F64,
        Type::Pointer => ptr_type,
    }
}

#[derive(Clone, Debug, Default)]
struct Translator {
    signatures: HashMap<ir::Signature, ir::SigRef>,
    host_call_trampoline: Option<ir::FuncRef>,
}
