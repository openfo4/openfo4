use std::collections::HashMap;
use std::hash::Hash;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    self, ArgumentExtension, ArgumentPurpose, BlockArg, InstBuilder, MemFlags, StackSlotData,
    StackSlotKind,
};
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{Context, settings};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use slab::Slab;

use crate::backend::{FunctionDefinitionDesc, StackSlotDesc};
use crate::{FloatCompareOp, Instruction, IntCompareOp, Signature, Type, Value};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(usize);

#[derive(Clone, Debug)]
pub enum Error {}

pub struct CraneliftModule {
    functions: Slab<FuncState>,
    compiled_functions: HashMap<usize, CompiledFunction>,
}

impl CraneliftModule {
    pub fn new() -> Self {
        Self {
            functions: Slab::new(),
            compiled_functions: HashMap::new(),
        }
    }

    pub fn declare_function(&mut self, signature: &Signature) -> Result<FunctionId, Error> {
        let id = self.functions.insert(FuncState {
            signature: signature.clone(),
            definition: None,
        });

        Ok(FunctionId(id))
    }

    pub fn define_native_function(&mut self, id: FunctionId, ptr: *const u8) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(state.definition.is_none(), "cannot redefine function");

        state.definition = Some(FunctionDefinition::Host(HostFunctionDefinition { ptr }));
    }

    pub fn define_function(&mut self, id: FunctionId, definition: FunctionDefinitionDesc) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(state.definition.is_none(), "cannot redefine function");

        state.definition = Some(FunctionDefinition::Guest(GuestFunctionDefinition {
            instructions: definition.instructions,
            num_values: definition.num_values,
            num_blocks: definition.num_blocks,
            block_params: definition.block_params,
            stack_slots: definition.stack_slots,
            num_variables: definition.num_variables,
        }));
    }

    pub fn compile_function(&mut self, id: FunctionId, debug: bool) -> CompiledFunction {
        if let Some(func) = self.compiled_functions.get(&id.0) {
            return func.clone();
        }

        let mut settings = settings::builder();
        settings.set("opt_level", "speed_and_size").unwrap();
        settings.set("regalloc_algorithm", "backtracking").unwrap();

        let mut builder = JITBuilder::with_flags(
            &[
                ("opt_level", "speed_and_size"),
                ("regalloc_algorithm", "backtracking"),
            ],
            cranelift_module::default_libcall_names(),
        )
        .unwrap();

        let native_functions = NativeFunctions::default();

        {
            let native_functions = native_functions.clone();
            builder.symbol_lookup_fn(Box::new(move |name| {
                dbg!(&name);
                dbg!(&native_functions.get(name));
                native_functions.get(name)
            }));
        }

        let mut module = JITModule::new(builder);
        let mut ctx = Context::new();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut function_ids = HashMap::new();
        let mut function_sigs = HashMap::new();
        let mut function_debug = HashMap::new();

        // Declare all functions.
        for (func_id, func) in &self.functions {
            let name = format!("anonymous_function_{}", func_id);

            let map_param = |ty: &Type| -> ir::AbiParam {
                let ty = match ty {
                    Type::I8 => ir::types::I8,
                    Type::I16 => ir::types::I16,
                    Type::I32 => ir::types::I32,
                    Type::I64 => ir::types::I64,
                    Type::F32 => ir::types::F32,
                    Type::F64 => ir::types::F64,
                    Type::Pointer => module.isa().pointer_type(),
                };

                ir::AbiParam {
                    value_type: ty,
                    purpose: ArgumentPurpose::Normal,
                    extension: ArgumentExtension::None,
                }
            };

            let params = func.signature.params.iter().map(map_param).collect();
            let returns = func.signature.returns.iter().map(map_param).collect();

            let call_conv = module.isa().default_call_conv();

            let sig = ir::Signature {
                params,
                returns,
                call_conv,
            };

            // FIXME: We currently use indirect calls for native functions. As such
            // they don't have to be registered here.
            // See comment on `Instruction::Call` for why we use indirect calls.
            if matches!(func.definition, Some(FunctionDefinition::Guest(_))) {
                let id = module
                    .declare_function(&name, Linkage::Export, &sig)
                    .unwrap();

                function_ids.insert(func_id, id);
            }

            function_sigs.insert(func_id, sig);
        }

        // Define functions
        for (func_id, func) in &self.functions {
            let name = format!("anonymous_function_{}", func_id);

            let func_def = match func.definition.as_ref().unwrap() {
                FunctionDefinition::Host(def) => {
                    native_functions.insert(name, def.ptr);
                    continue;
                }
                FunctionDefinition::Guest(def) => def,
            };

            module.clear_context(&mut ctx);

            let sig = function_sigs.get(&func_id).unwrap().clone();
            ctx.func.signature = sig;

            let mut builder =
                cranelift_frontend::FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

            let mut values = vec![const { None }; func_def.num_values];
            let mut blocks = vec![const { None }; func_def.num_blocks];
            let mut stack_slots = vec![const { None }; func_def.stack_slots.len()];
            let mut variables = vec![const { None }; func_def.num_variables];

            let ptr_type = module.isa().pointer_type();
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

            // Create stack slots
            for (id, slot) in &func_def.stack_slots {
                let res = builder.create_sized_stack_slot(StackSlotData {
                    kind: StackSlotKind::ExplicitSlot,
                    size: slot.size.try_into().unwrap(),
                    // `slot.align` already must be a power of two.
                    // FIXME: we can do better than ilog2.
                    align_shift: slot.align.ilog2() as u8,
                });

                stack_slots[id] = Some(res);
            }

            for instruction in &func_def.instructions {
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
                    }
                    Instruction::IConst(ty, value, dst) => {
                        let res = builder.ins().iconst(map_type(*ty), *value);
                        values[dst.0] = Some(res);
                    }
                    Instruction::FConst(ty, value, dst) => {
                        let res = match ty {
                            Type::F32 => builder.ins().f32const(*value as f32),
                            Type::F64 => builder.ins().f64const(*value),
                            _ => unreachable!(),
                        };

                        values[dst.0] = Some(res);
                    }
                    Instruction::IAdd(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().iadd(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::ISub(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().isub(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::IMul(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().imul(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::UDiv(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().udiv(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::SDiv(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().sdiv(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::URem(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().urem(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::SRem(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().srem(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FAdd(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().fadd(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FSub(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().fsub(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FMul(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().fmul(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FDiv(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().fdiv(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FMin(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().fmin(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FMax(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().fmax(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::INeg(op) => {
                        let src = values[op.src.0].unwrap();

                        let res = builder.ins().ineg(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FNeg(op) => {
                        let src = values[op.src.0].unwrap();

                        let res = builder.ins().fneg(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::FAbs(op) => {
                        let src = values[op.src.0].unwrap();

                        let res = builder.ins().fabs(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::Floor(op) => {
                        let src = values[op.src.0].unwrap();

                        let res = builder.ins().floor(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::Ceil(op) => {
                        let src = values[op.src.0].unwrap();

                        let res = builder.ins().ceil(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::Call(func, params, returns) => {
                        let args = params
                            .iter()
                            .map(|v| values[v.0].unwrap())
                            .collect::<Vec<_>>();

                        // FIXME: cranelift 0.125 emits 32 bit relocations, which
                        // causes some executable addresses to be too far away to call
                        // each other.
                        // https://github.com/bytecodealliance/wasmtime/issues/4000
                        // As a temporary workaround we use indirect calls to the address
                        // when calling host functions.
                        // This is not possible for guest functions because they may
                        // not be compiled and may not have an address yet.

                        let inst = match &self.functions.get(func.0).unwrap().definition {
                            Some(FunctionDefinition::Host(def)) => {
                                let sig = function_sigs.get(&func.0).unwrap();
                                let sig_ref = builder.import_signature(sig.clone());

                                let addr = u64::try_from(def.ptr.addr()).unwrap() as i64;
                                let callee = builder.ins().iconst(ptr_type, addr);

                                builder.ins().call_indirect(sig_ref, callee, &args)
                            }
                            Some(FunctionDefinition::Guest(_)) => {
                                let func = *function_ids.get(&func.0).unwrap();
                                let func_ref = module.declare_func_in_func(func, &mut builder.func);

                                builder.ins().call(func_ref, &args)
                            }
                            None => unreachable!(),
                        };

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
                    }
                    Instruction::Jump(block, block_args) => {
                        let block = blocks[block.0].unwrap();

                        let args = block_args
                            .iter()
                            .map(|v| BlockArg::Value(values[v.0].unwrap()))
                            .collect::<Vec<_>>();

                        builder.ins().jump(block, &args);
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
                    }
                    Instruction::StackStore {
                        slot,
                        value,
                        offset,
                    } => {
                        let value = values[value.0].unwrap();
                        let slot = stack_slots[slot.0].unwrap();

                        builder.ins().stack_store(value, slot, *offset as i32);
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
                    }
                    Instruction::FToU { value, ty, dst } => {
                        let value = values[value.0].unwrap();
                        let ty = map_type(*ty);

                        let res = builder.ins().fcvt_to_uint_sat(ty, value);
                        values[dst.0] = Some(res);
                    }
                    Instruction::FToS { value, ty, dst } => {
                        let value = values[value.0].unwrap();
                        let ty = map_type(*ty);

                        let res = builder.ins().fcvt_to_sint_sat(ty, value);
                        values[dst.0] = Some(res);
                    }
                    Instruction::UToF { value, ty, dst } => {
                        let value = values[value.0].unwrap();
                        let ty = map_type(*ty);

                        let res = builder.ins().fcvt_from_uint(ty, value);
                        values[dst.0] = Some(res);
                    }
                    Instruction::SToF { value, ty, dst } => {
                        let value = values[value.0].unwrap();
                        let ty = map_type(*ty);

                        let res = builder.ins().fcvt_from_sint(ty, value);
                        values[dst.0] = Some(res);
                    }
                    Instruction::BAnd(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().band(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::BOr(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().bor(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::BXor(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let rhs = values[op.rhs.0].unwrap();

                        let res = builder.ins().bxor(lhs, rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::BNot(op) => {
                        let src = values[op.src.0].unwrap();
                        let res = builder.ins().bnot(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::UExtend { value, dst, ty } => {
                        let src = values[value.0].unwrap();
                        let res = builder.ins().uextend(map_type(*ty), src);
                        values[dst.0] = Some(res);
                    }
                    Instruction::SExtend { value, dst, ty } => {
                        let src = values[value.0].unwrap();
                        let res = builder.ins().sextend(map_type(*ty), src);
                        values[dst.0] = Some(res);
                    }
                    Instruction::IReduce { value, dst, ty } => {
                        let src = values[value.0].unwrap();
                        let res = builder.ins().ireduce(map_type(*ty), src);
                        values[dst.0] = Some(res);
                    }
                    Instruction::Bitcast { value, dst, ty } => {
                        let src = values[value.0].unwrap();
                        let ty = map_type(*ty);
                        let res = builder.ins().bitcast(ty, MemFlags::new(), src);
                        values[dst.0] = Some(res);
                    }
                    Instruction::Popcnt(op) => {
                        let src = values[op.src.0].unwrap();
                        let res = builder.ins().popcnt(src);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::DefVar { ty, var, value } => {
                        let res = builder.declare_var(map_type(*ty));
                        let value = values[value.0].unwrap();
                        builder.def_var(res, value);
                        variables[var.0] = Some(res);
                    }
                    Instruction::GetVar { var, dst } => {
                        let var = variables[var.0].unwrap();
                        let res = builder.use_var(var);
                        values[dst.0] = Some(res);
                    }
                    Instruction::SetVar { var, value } => {
                        let var = variables[var.0].unwrap();
                        let value = values[value.0].unwrap();
                        builder.def_var(var, value);
                    }
                    Instruction::Memcpy { src, dst, size } => {
                        let src = values[src.0].unwrap();
                        let dst = values[dst.0].unwrap();
                        let size = values[size.0].unwrap();
                        builder.call_memcpy(module.isa().frontend_config(), dst, src, size);
                    }
                    Instruction::IAddImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let res = builder.ins().iadd_imm(lhs, op.rhs);
                        values[op.dst.0] = Some(res);
                    }
                    Instruction::IMulImm(op) => {
                        let lhs = values[op.lhs.0].unwrap();
                        let res = builder.ins().imul_imm(lhs, op.rhs);
                        values[op.dst.0] = Some(res);
                    }
                }
            }

            builder.seal_all_blocks();

            let clif = debug.then(|| builder.func.display().to_string());

            builder.finalize();

            let id = *function_ids.get(&func_id).unwrap();
            module
                .define_function(id, &mut ctx)
                .inspect_err(|err| {
                    println!("{}", clif.as_ref().unwrap());
                    println!("{}", err);
                })
                .unwrap();

            if debug {
                ctx.set_disasm(true);
                let compiled_code = ctx
                    .compile(module.isa(), &mut ControlPlane::default())
                    .unwrap();

                let disasm = compiled_code.vcode.as_ref().unwrap();

                let text = format!(
                    "CLIF:\n{}------------\nASM:\n{}------------",
                    clif.unwrap(),
                    disasm
                );
                function_debug.insert(func_id, text);
            }
        }

        module.finalize_definitions().unwrap();

        let module = Arc::new(SharedJITModule(ManuallyDrop::new(module)));

        for (func_id, func) in &self.functions {
            if !matches!(func.definition, Some(FunctionDefinition::Guest(_))) {
                continue;
            }

            let func = function_ids.get(&func_id).unwrap();

            let fn_ptr = module.0.get_finalized_function(*func);
            let text = function_debug.remove(&func_id);

            self.compiled_functions.insert(
                func_id,
                CompiledFunction {
                    _module: module.clone(),
                    fn_ptr,
                    text,
                },
            );
        }

        debug_assert!(self.compiled_functions.contains_key(&id.0));
        self.compiled_functions.get(&id.0).unwrap().clone()
    }
}

struct FuncState {
    signature: Signature,
    definition: Option<FunctionDefinition>,
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

#[derive(Clone, Debug)]
struct GuestFunctionDefinition {
    instructions: Vec<Instruction>,
    num_values: usize,
    num_blocks: usize,
    num_variables: usize,
    block_params: Vec<Vec<Value>>,
    stack_slots: Slab<StackSlotDesc>,
}

struct SharedJITModule(ManuallyDrop<JITModule>);

impl Drop for SharedJITModule {
    fn drop(&mut self) {
        unsafe {
            let module = ManuallyDrop::take(&mut self.0);
            module.free_memory();
        }
    }
}

#[derive(Clone)]
pub struct CompiledFunction {
    /// This module contains the executable machine code and must not be dropped until all
    /// function calls terminate.
    _module: Arc<SharedJITModule>,
    fn_ptr: *const u8,
    text: Option<String>,
}

impl CompiledFunction {
    pub fn as_ptr(&self) -> *const u8 {
        self.fn_ptr
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_ref().map(|v| v.as_str())
    }
}

impl Drop for CompiledFunction {
    fn drop(&mut self) {}
}

#[derive(Clone, Debug, Default)]
struct NativeFunctions(Arc<Mutex<HashMap<String, *const u8>>>);

impl NativeFunctions {
    fn insert(&self, name: String, ptr: *const u8) {
        self.0.lock().unwrap().insert(name, ptr);
    }

    fn get(&self, name: &str) -> Option<*const u8> {
        self.0.lock().unwrap().get(name).copied()
    }
}

// SAFETY: We store function pointers which have no additional safety
// requirements unless we call them.
unsafe impl Send for NativeFunctions {}
unsafe impl Sync for NativeFunctions {}
