mod debug;
mod libcall;
mod memory;
mod relocs;

use std::alloc::Layout;
use std::sync::Arc;

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    self, ArgumentExtension, ArgumentPurpose, BlockArg, ExtFuncData, ExternalName, Function,
    InstBuilder, InstBuilderBase, MemFlags, StackSlotData, StackSlotKind, UserExternalName,
    UserExternalNameRef, UserFuncName,
};
use cranelift_codegen::isa::OwnedTargetIsa;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{CompiledCode, Context, settings};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use slab::Slab;

use crate::backend::cranelift::memory::{Allocation, MemoryAllocator};
use crate::backend::cranelift::relocs::perform_relocations;
use crate::backend::{FunctionDefinitionDesc, SourceLocMap, StackSlotDesc, TextWithSrcLocs};
use crate::{FloatCompareOp, FuncId, Instruction, IntCompareOp, Signature, SourceLoc, Type, Value};

#[derive(Clone, Debug)]
pub enum Error {}

pub struct CraneliftModule {
    functions: Slab<FuncState>,
    compiled_functions: HashMap<usize, CompiledFunction>,
    allocator: MemoryAllocator,
    isa: OwnedTargetIsa,
    ctx: Context,
    func_ctx: FunctionBuilderContext,
    debug: bool,
}

impl CraneliftModule {
    pub fn new(validation: bool, debug: bool) -> Self {
        let mut flags = settings::builder();
        flags.set("use_colocated_libcalls", "false").unwrap();
        flags.set("is_pic", "false").unwrap();
        flags.set("opt_level", "speed_and_size").unwrap();
        flags.set("regalloc_algorithm", "backtracking").unwrap();

        if validation {
            flags.set("enable_verifier", "true").unwrap();
        } else {
            flags.set("enable_verifier", "false").unwrap();
        }

        let builder = cranelift_native::builder().unwrap();
        let isa = builder.finish(settings::Flags::new(flags)).unwrap();

        Self {
            functions: Slab::new(),
            compiled_functions: HashMap::new(),
            allocator: MemoryAllocator::new(),
            ctx: Context::new(),
            isa,
            func_ctx: FunctionBuilderContext::new(),
            debug,
        }
    }

    pub fn declare_function(&mut self, signature: &Signature) -> Result<FuncId, Error> {
        let id = self.functions.insert(FuncState {
            signature: signature.clone(),
            definition: None,
            compiled: None,
        });

        Ok(FuncId(id))
    }

    pub fn define_native_function(&mut self, id: FuncId, ptr: *const u8) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(state.definition.is_none(), "cannot redefine function");

        state.definition = Some(FunctionDefinition::Host(HostFunctionDefinition { ptr }));
    }

    pub fn define_function(&mut self, id: FuncId, definition: FunctionDefinitionDesc) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(state.definition.is_none(), "cannot redefine function");

        state.definition = Some(FunctionDefinition::Guest(GuestFunctionDefinition {
            instructions: definition.instructions,
            num_values: definition.num_values,
            num_blocks: definition.num_blocks,
            block_params: definition.block_params,
            stack_slots: definition.stack_slots,
            num_variables: definition.num_variables,
            srcloc: definition.srcloc,
        }));
    }

    pub fn compile_function(&mut self, id: FuncId) -> CompiledFunction {
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

            let call_conv = self.isa.default_call_conv();

            ir::Signature {
                params,
                returns,
                call_conv,
            }
        };

        let mut funcs_to_compile = vec![id];

        let mut compiled_funcs = HashSet::new();

        while let Some(id) = funcs_to_compile.pop() {
            let func = &self.functions[id.0];
            if func.compiled.is_some() {
                continue;
            }

            // Note that FuncRefs are only valid for a single function,
            // so we must recreate them for each function.
            let mut callees = HashMap::new();
            let mut refs_to_funcs = HashMap::new();

            let func_def = match func.definition.as_ref().unwrap() {
                FunctionDefinition::Host(_) => continue,
                FunctionDefinition::Guest(def) => def,
            };

            self.ctx.clear();

            let signature = convert_signature(&func.signature);

            let mut func = Function::with_name_signature(
                UserFuncName::User(UserExternalName::new(0, id.0.try_into().unwrap())),
                signature,
            );

            let mut builder = FunctionBuilder::new(&mut func, &mut self.func_ctx);

            let mut values = vec![const { None }; func_def.num_values];
            let mut blocks = vec![const { None }; func_def.num_blocks];
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

            let mut current_block = None;

            for (index, instruction) in func_def.instructions.iter().enumerate() {
                let current_block_on_entry = current_block;
                let mut added_inst_count = 0;

                match instruction {
                    Instruction::CreateBlock {
                        id,
                        params,
                        values: dst,
                        is_entry,
                    } => {
                        let block = builder.create_block();

                        if *is_entry {
                            builder.append_block_params_for_function_params(block);
                            builder.switch_to_block(block);
                            builder.seal_block(block);
                        } else {
                            for param in params {
                                builder.append_block_param(block, map_type(*param));
                            }
                        }

                        for (block_param, param) in builder.block_params(block).iter().zip(dst) {
                            values[param.0] = Some(*block_param);
                        }

                        blocks[id.0] = Some(block);
                    }
                    Instruction::SwitchBlock(block) => {
                        let block = blocks[block.0].unwrap();
                        builder.switch_to_block(block);
                        current_block = Some(block);
                    }
                    Instruction::IConst(ty, value, dst) => {
                        let res = builder.ins().iconst(map_type(*ty), *value);
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
                    Instruction::ISubImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let res = builder.ins().iadd_imm(lhs, -op.rhs);
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
                        let sig_ref = builder.import_signature(signature);

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

                        let compile_func = match func.definition {
                            Some(FunctionDefinition::Host(_)) => false,
                            Some(FunctionDefinition::Guest(_)) => func.compiled.is_none(),
                            None => unreachable!(),
                        };

                        if compile_func {
                            funcs_to_compile.push(*func_id);
                        }

                        let func_ref =
                            *callees.entry(*func_id).or_insert_with(|| {
                                let name = builder.func.declare_imported_user_function(
                                    UserExternalName::new(0, func_id.0.try_into().unwrap()),
                                );

                                refs_to_funcs.insert(name, *func_id);

                                let signature = convert_signature(&func.signature);
                                let sig_ref = builder.import_signature(signature);

                                builder.import_function(ExtFuncData {
                                    name: ExternalName::User(name),
                                    signature: sig_ref,
                                    colocated: false,
                                    patchable: false,
                                })
                            });

                        let inst = builder.ins().call(func_ref, &args);
                        added_inst_count += 1;

                        let return_values = builder.inst_results(inst);
                        debug_assert_eq!(returns.len(), return_values.len());

                        for (lhs, rhs) in returns.iter().zip(return_values) {
                            values[lhs.0] = Some(*rhs);
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

                        let res = builder.ins().stack_load(ty, slot, *offset as i32);
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

                        builder.ins().stack_store(value, slot, *offset as i32);
                        added_inst_count += 1;
                    }
                    Instruction::StackAddr {
                        slot,
                        ty,
                        dst,
                        offset,
                    } => {
                        let slot = stack_slots[slot.0].unwrap();
                        let ty = map_type(*ty);

                        let res = builder.ins().stack_addr(ty, slot, *offset as i32);
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
                            IntCompareOp::SignedLessThanOrEqual => IntCC::SignedLessThanOrEqual,
                            IntCompareOp::UnsignedGreaterThan => IntCC::UnsignedGreaterThan,
                            IntCompareOp::UnsignedGreaterThanOrEqual => {
                                IntCC::UnsignedGreaterThanOrEqual
                            }
                            IntCompareOp::UnsignedLessThan => IntCC::UnsignedLessThan,
                            IntCompareOp::UnsignedLessThanOrEqual => IntCC::UnsignedLessThanOrEqual,
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
                            FloatCompareOp::GreaterThanOrEqual => FloatCC::GreaterThanOrEqual,
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

                        let res = builder
                            .ins()
                            .load(ty, MemFlags::trusted(), addr, *offset as i32);
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

                        builder
                            .ins()
                            .store(MemFlags::trusted(), value, addr, *offset as i32);
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
                        let res = builder.call_memcmp(self.isa.frontend_config(), lhs, rhs, size);
                        values[dst.0] = Some(res);
                        added_inst_count += 1;
                    }
                    Instruction::IAddImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let res = builder.ins().iadd_imm(lhs, op.rhs);
                        values[op.dst.0] = Some(res);
                        added_inst_count += 1;
                    }
                    Instruction::IMulImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let res = builder.ins().imul_imm(lhs, op.rhs);
                        values[op.dst.0] = Some(res);
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
                    // Cranelift has no immediate min/max.
                    Instruction::SMinImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let ty = builder.ins().data_flow_graph().value_type(lhs);
                        let rhs = builder.ins().iconst(ty, op.rhs);
                        let res = builder.ins().smin(lhs, rhs);
                        values[op.dst.0] = Some(res);
                        added_inst_count += 1;
                    }
                    Instruction::SMaxImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let ty = builder.ins().data_flow_graph().value_type(lhs);
                        let rhs = builder.ins().iconst(ty, op.rhs);
                        let res = builder.ins().smax(lhs, rhs);
                        values[op.dst.0] = Some(res);
                        added_inst_count += 1;
                    }
                    Instruction::UMinImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let ty = builder.ins().data_flow_graph().value_type(lhs);
                        let rhs = builder.ins().iconst(ty, op.rhs);
                        let res = builder.ins().umin(lhs, rhs);
                        values[op.dst.0] = Some(res);
                        added_inst_count += 1;
                    }
                    Instruction::UMaxImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let ty = builder.ins().data_flow_graph().value_type(lhs);
                        let rhs = builder.ins().iconst(ty, op.rhs);
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

        let compiled = self.functions[id.0].compiled.as_ref().unwrap();
        CompiledFunction {
            _allocation: compiled.allocation.clone(),
            fn_ptr: compiled.allocation.as_exec_ptr(),
            clif: compiled.clif.clone(),
            srclocs: compiled.srclocs.clone(),
            len: compiled.code.code_buffer().len(),
        }
    }
}

struct CompilationResult {
    code: CompiledCode,
    allocation: Arc<Allocation>,
    callees: HashMap<UserExternalNameRef, FuncId>,
    clif: Option<TextWithSrcLocs<String>>,
    srclocs: SourceLocMap,
}

struct FuncState {
    signature: Signature,
    definition: Option<FunctionDefinition>,
    compiled: Option<CompilationResult>,
}

#[derive(Clone, Debug)]
enum FunctionDefinition {
    Host(HostFunctionDefinition),
    Guest(GuestFunctionDefinition),
}

#[derive(Clone, Debug)]
struct HostFunctionDefinition {
    ptr: *const u8,
}

unsafe impl Send for HostFunctionDefinition {}
unsafe impl Sync for HostFunctionDefinition {}

#[derive(Clone, Debug)]
struct GuestFunctionDefinition {
    instructions: Vec<Instruction>,
    num_values: usize,
    num_blocks: usize,
    num_variables: usize,
    block_params: Vec<Vec<Value>>,
    stack_slots: Slab<StackSlotDesc>,
    srcloc: SourceLocMap,
}

#[derive(Clone)]
pub struct CompiledFunction {
    /// The allocation must not be droped until all function calls terminate.
    _allocation: Arc<Allocation>,
    fn_ptr: *const u8,
    clif: Option<TextWithSrcLocs<String>>,
    srclocs: SourceLocMap,
    len: usize,
}

impl CompiledFunction {
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
