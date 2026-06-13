use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{Compiler, FuncId, Signature, Type as JitType};
use openfo4_pex::{Instruction, Value};

use crate::compiler::NativeFunction;
use crate::compiler::codegen::hir::{
    self, AddrCompareOp, FunctionBuilder, Literal, Null, ScalarCompareOp, StringCompareOp, Variable,
};
use crate::compiler::codegen::pex::cfg::ControlFlowGraph;
use crate::compiler::codegen::pex::registry::{ClassId, FunctionBody, FunctionId, StateId};
use crate::compiler::codegen::trace::Span;
use crate::compiler::codegen::trampoline::generate_guest_to_host_trampoline;
use crate::compiler::codegen::{
    ClassDef, CodegenData, Error, FunctionDef, FunctionStateImpl, PscSignature, Type,
    get_parent_class_with_property,
};
use crate::compiler::debug::DebugInfo;
use crate::compiler::resources::TypeId;
use crate::runtime::object::ObjectTag;

#[derive(Debug)]
pub struct FunctionResult {
    // Keep this as Box instead of inlining directly.
    // In most cases this will be None because debug info
    // collection is disabled.
    pub debug: Option<Box<DebugInfo>>,
}

pub fn declare_function(
    data: &CodegenData,
    class: &ClassDef,
    func_id: FunctionId,
    func: &FunctionDef,
    compiler: &mut Compiler,
    use_native_stubs: bool,
) -> Result<FuncId, Error> {
    let signature = translate_function_signature(&func);

    if func.is_native {
        tracing::debug!(
            "compiling {}.{} (native)",
            class.debug_name,
            func.debug_name
        );

        let native = match data.native_funcs.get(&func_id) {
            Some(native) => {
                if signature != native.signature {
                    return Err(Error::NativeFuncSignatureMissmatch {
                        name: format!("{}.{}", class.debug_name, func.debug_name),
                        script: signature,
                        host: native.signature.clone(),
                    });
                }

                native.clone()
            }
            None if use_native_stubs => NativeFunction {
                signature: signature.clone(),
                index: 0,
                ptr: crate::runtime::host_functions::stub as *const u8,
            },
            None => {
                return Err(Error::UndefNativeFunc(format!(
                    "{}.{}",
                    class.debug_name, func.debug_name
                )));
            }
        };

        let id = generate_guest_to_host_trampoline(
            compiler,
            data,
            &signature,
            &native,
            &class.debug_name,
            &func.debug_name,
        );
        Ok(id)
    } else {
        let id = compiler.declare_function(signature);
        Ok(id)
    }
}

pub fn translate_function_psc_signature(data: &CodegenData, func: &FunctionDef) -> PscSignature {
    let mut sig = PscSignature::default();

    if !func.is_static {
        let ty = type_to_id(data, &Type::Class(func.class));
        sig.params.push(ty);
    }

    for ty in &func.params {
        let ty = type_to_id(data, ty);
        sig.params.push(ty);
    }

    if func.returns != Type::None {
        let ty = type_to_id(data, &func.returns);
        sig.returns.push(ty);
    }

    sig
}

pub fn translate_function(
    data: &CodegenData,
    class: &ClassDef,
    func: &FunctionDef,
    compiler: &mut Compiler,
    enable_debug: bool,
) -> FunctionResult {
    let func_id = data.functions[&func.id];

    let mut debug = None;

    if !func.is_native {
        tracing::debug!("compiling {}.{}", class.debug_name, func.debug_name);

        let mut builder = FunctionBuilder::new();

        translate_function_body(data, class.id, func, &mut builder);

        let span = Span::new(data, &class.debug_name, &func.debug_name);

        let f = builder.clone().finalize();

        let builder = compiler.define_function(func_id);
        f.lower(data, builder, span);

        if enable_debug {
            debug = Some(Box::new(DebugInfo {
                name: format!("{}.{}", class.debug_name, func.debug_name),
                pex: "TODO".into(),
                hir: f.debug_text(data),
            }));
        }
    }

    FunctionResult { debug }
}

pub fn translate_function_signature(func: &FunctionDef) -> Signature {
    let mut params = Vec::new();
    let mut returns = Vec::new();

    // All functions must carry the global context pointer.
    params.push(JitType::Pointer);

    // For non-static function (methods) the first argument is the object pointer.
    if !func.is_static {
        params.push(JitType::Pointer);
    }

    for ty in &func.params {
        params.push(ty.to_jit());
    }

    if func.returns != Type::None {
        returns.push(func.returns.to_jit());
    }

    // VM error value
    returns.push(JitType::I32);

    Signature { params, returns }
}

fn translate_function_body(
    data: &CodegenData,
    class: ClassId,
    func: &FunctionDef,
    builder: &mut FunctionBuilder,
) {
    let self_ty = *data.class_types.get(&class).unwrap();
    let self_ptr = (!func.is_static).then(|| builder.add_param(self_ty));

    let mut params = Vec::new();
    for ty in &func.params {
        let var = builder.add_param(type_to_id(data, ty));
        params.push((ty, var));
    }

    let return_ty = (func.returns != Type::None).then(|| type_to_id(data, &func.returns));
    if let Some(ty) = return_ty {
        builder.set_returns(ty);
    }

    let mut translator = FunctionTranslator {
        data,
        builder,
        locals: HashMap::new(),
        self_ptr,
        self_ty: class,
    };

    // No need to load state value if only default state exists.
    if func.states.len() > 1 {
        let state_value = translator.load(&Value::Ident("::state".to_owned()));
        let state_matches = translator
            .builder
            .declare_variable(translator.data.resources.primitives.bool);

        let ret_dst = hir::Value::Variable(
            translator
                .builder
                .declare_variable(type_to_id(translator.data, &func.returns)),
        );

        // States only work on non-static functions, i.e. the object
        // pointer is always available.
        let self_ptr = translator.self_ptr.unwrap();

        let mut args: Vec<_> = params
            .iter()
            .map(|(_, var)| hir::Value::Variable(*var))
            .collect();
        args.insert(0, hir::Value::Variable(self_ptr));

        let mut next_block = translator.builder.create_block();

        for (id, state) in &func.states {
            let StateId::Named(name) = id else {
                continue;
            };

            translator.builder.compare_string(
                state_value.clone(),
                hir::Value::Const(name.clone()),
                hir::Value::Variable(state_matches),
                StringCompareOp::Equal,
            );

            let block = translator.builder.create_block();
            translator
                .builder
                .brif_true(hir::Value::Variable(state_matches), block, next_block);

            translator.builder.switch_to_block(block);

            match state {
                FunctionStateImpl::Body(body) => {
                    translator.locals.clear();
                    for ((ty, var), name) in params.iter().zip(&body.params) {
                        translator.locals.insert(
                            name.clone(),
                            Local {
                                ty: (*ty).clone(),
                                var: *var,
                            },
                        );
                    }

                    translate_state_body(&mut translator, func, body);
                }
                FunctionStateImpl::Call(id) => {
                    translator.builder.call(
                        *id,
                        Some(hir::Value::Variable(self_ptr)),
                        &args,
                        ret_dst.clone(),
                    );
                    translator.builder.return_(ret_dst.clone());
                }
            }

            translator.builder.switch_to_block(next_block);
            next_block = translator.builder.create_block();
        }

        translator.builder.jump(next_block);
        translator.builder.switch_to_block(next_block);
    }

    let state = func.states.get(&StateId::Default).unwrap();
    match state {
        FunctionStateImpl::Body(body) => {
            translator.locals.clear();
            for ((ty, var), name) in params.iter().zip(&body.params) {
                translator.locals.insert(
                    name.clone(),
                    Local {
                        ty: (*ty).clone(),
                        var: *var,
                    },
                );
            }

            translate_state_body(&mut translator, func, body);
        }
        FunctionStateImpl::Call(id) => {
            let ret_dst = hir::Value::Variable(
                translator
                    .builder
                    .declare_variable(type_to_id(translator.data, &func.returns)),
            );

            // States only work on non-static functions, i.e. the object
            // pointer is always available.
            let self_ptr = translator.self_ptr.unwrap();

            let mut args: Vec<_> = params
                .iter()
                .map(|(_, var)| hir::Value::Variable(*var))
                .collect();
            args.insert(0, hir::Value::Variable(self_ptr));

            translator.builder.call(
                *id,
                Some(hir::Value::Variable(self_ptr)),
                &args,
                ret_dst.clone(),
            );
            translator.builder.return_(ret_dst.clone());
        }
    }
}

fn translate_state_body(
    mut translator: &mut FunctionTranslator<'_>,
    func: &FunctionDef,
    body: &FunctionBody,
) {
    let cfg = ControlFlowGraph::new(&body.instructions);

    for (name, ty) in &body.locals {
        let id = type_to_id(translator.data, ty);
        let var = translator.builder.declare_variable(id);
        translator.locals.insert(
            name.clone(),
            Local {
                ty: ty.clone(),
                var,
            },
        );
    }

    let mut blocks = HashMap::new();
    for (block_id, _) in cfg.blocks() {
        let block = translator.builder.create_block();
        blocks.insert(block_id, block);
    }

    let (entry_block_id, _) = cfg.blocks().next().unwrap();
    let entry_block = blocks[&entry_block_id];
    translator.builder.jump(entry_block);
    translator.builder.switch_to_block(entry_block);

    for (block_index, (block_id, instructions)) in cfg.blocks().enumerate() {
        let mut current_block_sealed = false;

        // The entry block is enabled by default.
        if block_index != 0 {
            let block = blocks[&block_id];
            translator.builder.switch_to_block(block);
        }

        for instruction in instructions {
            debug_assert!(!current_block_sealed);

            match instruction {
                Instruction::Nop => {}
                Instruction::IAdd(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.iadd(lhs, rhs, dst);
                }
                Instruction::ISub(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.isub(lhs, rhs, dst);
                }
                Instruction::IMul(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.imul(lhs, rhs, dst);
                }
                Instruction::IDiv(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.idiv(lhs, rhs, dst);
                }
                Instruction::IMod(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.imod(lhs, rhs, dst);
                }
                Instruction::INeg(op) => {
                    let src = translator.load(&op.src);
                    let dst = translator.load(&op.dst);
                    translator.builder.ineg(src, dst);
                }
                Instruction::FAdd(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.fadd(lhs, rhs, dst);
                }
                Instruction::FSub(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.fsub(lhs, rhs, dst);
                }
                Instruction::FMul(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.fmul(lhs, rhs, dst);
                }
                Instruction::FDiv(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.fdiv(lhs, rhs, dst);
                }
                Instruction::FNeg(op) => {
                    let src = translator.load(&op.src);
                    let dst = translator.load(&op.dst);
                    translator.builder.fneg(src, dst);
                }
                Instruction::Not(op) => match translator.type_of_value(&op.src) {
                    Type::Bool => {
                        let src = translator.load(&op.src);
                        let dst = translator.load(&op.dst);

                        translator.builder.bnot(src, dst);
                    }
                    Type::Class(_) | Type::Struct(_) | Type::Array(_) => {
                        let src = translator.load(&op.src);
                        let dst = translator.load(&op.dst);

                        translator.builder.compare_addr(
                            src,
                            hir::Value::Const(Null),
                            dst,
                            AddrCompareOp::NotEqual,
                        );
                    }
                    _ => unreachable!(),
                },
                Instruction::Store(op) => {
                    let src = translator.load(&op.src);
                    let dst = translator.load(&op.dst);
                    translator.builder.store(src, dst);
                }
                Instruction::CmpEq(op)
                | Instruction::CmpLt(op)
                | Instruction::CmpLe(op)
                | Instruction::CmpGt(op)
                | Instruction::CmpGe(op) => match translator.type_of_value(&op.lhs) {
                    Type::None => {
                        todo!()
                    }
                    Type::Bool => {
                        let lhs = translator.load(&op.lhs);
                        let rhs = translator.load(&op.rhs);
                        let dst = translator.load(&op.dst);

                        let op = match instruction {
                            Instruction::CmpEq(_) => ScalarCompareOp::Equal,
                            Instruction::CmpLt(_) => ScalarCompareOp::LessThan,
                            Instruction::CmpLe(_) => ScalarCompareOp::LessThanOrEqual,
                            Instruction::CmpGt(_) => ScalarCompareOp::GreaterThan,
                            Instruction::CmpGe(_) => ScalarCompareOp::GreaterThanOrEqual,
                            _ => unreachable!(),
                        };

                        translator.builder.compare_bool(lhs, rhs, dst, op);
                    }
                    Type::Int => {
                        let lhs = translator.load(&op.lhs);
                        let rhs = translator.load(&op.rhs);
                        let dst = translator.load(&op.dst);

                        let op = match instruction {
                            Instruction::CmpEq(_) => ScalarCompareOp::Equal,
                            Instruction::CmpLt(_) => ScalarCompareOp::LessThan,
                            Instruction::CmpLe(_) => ScalarCompareOp::LessThanOrEqual,
                            Instruction::CmpGt(_) => ScalarCompareOp::GreaterThan,
                            Instruction::CmpGe(_) => ScalarCompareOp::GreaterThanOrEqual,
                            _ => unreachable!(),
                        };

                        translator.builder.compare_int(lhs, rhs, dst, op);
                    }
                    Type::Float => {
                        let lhs = translator.load(&op.lhs);
                        let rhs = translator.load(&op.rhs);
                        let dst = translator.load(&op.dst);

                        let op = match instruction {
                            Instruction::CmpEq(_) => ScalarCompareOp::Equal,
                            Instruction::CmpLt(_) => ScalarCompareOp::LessThan,
                            Instruction::CmpLe(_) => ScalarCompareOp::LessThanOrEqual,
                            Instruction::CmpGt(_) => ScalarCompareOp::GreaterThan,
                            Instruction::CmpGe(_) => ScalarCompareOp::GreaterThanOrEqual,
                            _ => unreachable!(),
                        };

                        translator.builder.compare_float(lhs, rhs, dst, op);
                    }
                    Type::String => {
                        let lhs = translator.load(&op.lhs);
                        let rhs = translator.load(&op.rhs);
                        let dst = translator.load(&op.dst);

                        let op = match instruction {
                            Instruction::CmpEq(_) => StringCompareOp::Equal,
                            _ => unreachable!(),
                        };

                        translator.builder.compare_string(lhs, rhs, dst, op);
                    }
                    Type::Class(_) | Type::Struct(_) | Type::Array(_) | Type::Var => {
                        let lhs = translator.load(&op.lhs);
                        let rhs = translator.load(&op.rhs);
                        let dst = translator.load(&op.dst);

                        let op = match instruction {
                            Instruction::CmpEq(_) => AddrCompareOp::Equal,
                            _ => unreachable!(),
                        };

                        translator.builder.compare_addr(lhs, rhs, dst, op);
                    }
                },
                Instruction::Branch(_) => {
                    let succ = cfg.successors(block_id);
                    let target = *blocks.get(&succ[0]).unwrap();
                    translator.builder.jump(target);
                    current_block_sealed = true;
                }
                Instruction::BranchIfTrue(op) | Instruction::BranchIfFalse(op) => {
                    let succ = cfg.successors(block_id);
                    let then_target = *blocks.get(&succ[0]).unwrap();
                    let else_target = *blocks.get(&succ[1]).unwrap();

                    let condition = translator.load(&op.condition);

                    if matches!(instruction, Instruction::BranchIfTrue(_)) {
                        translator
                            .builder
                            .brif_true(condition, then_target, else_target);
                    } else {
                        translator
                            .builder
                            .brif_false(condition, then_target, else_target);
                    }

                    current_block_sealed = true;
                }
                Instruction::CallMethod(op) => {
                    let Type::Class(class_id) = translator.type_of_value(&op.object) else {
                        unreachable!();
                    };
                    let function_name = Lowercase::new(op.function_name.as_str());

                    let class = translator.data.registry.class(class_id).unwrap();
                    let callee = class.functions.get_method(&function_name).unwrap();

                    let object = translator.load(&op.object);
                    let dst = translator.load(&op.return_assignment);

                    let mut args = vec![object.clone().map(|_| Literal::Null)];
                    for arg in &op.args {
                        let arg = translator.load(arg);
                        args.push(arg);
                    }

                    translator
                        .builder
                        .call_indirect(callee, Some(object), &args, dst);
                }
                Instruction::CallParent(op) => {
                    let func_name = Lowercase::new(op.function.as_str());

                    let class = translator.data.registry.class(translator.self_ty).unwrap();
                    let callee = class.functions.get_parent(&func_name).unwrap();

                    let object = translator.self_ptr.unwrap();
                    let dst = translator.load(&op.return_assignment);

                    let mut args = vec![hir::Value::Variable(object)];
                    for arg in &op.args {
                        let arg = translator.load(arg);
                        args.push(arg);
                    }

                    translator
                        .builder
                        .call(callee, Some(hir::Value::Variable(object)), &args, dst);
                }
                Instruction::CallStatic(op) => {
                    let class_name = Lowercase::new(op.class.as_str());
                    let func_name = Lowercase::new(op.function.as_str());

                    let args = op
                        .args
                        .iter()
                        .map(|v| translator.load(v))
                        .collect::<Vec<_>>();
                    let dst = translator.load(&op.return_assignment);

                    let class = translator.data.registry.class_by_name(&class_name).unwrap();
                    let callee = class.functions.get_static(&func_name).unwrap();

                    translator.builder.call(callee, None, &args, dst);
                }
                Instruction::PropertyGet(op) => {
                    let Type::Class(class_id) = translator.type_of_value(&op.value) else {
                        unreachable!();
                    };
                    let prop_name = Lowercase::new(op.property.as_str());

                    let class_id =
                        get_parent_class_with_property(translator.data, class_id, &prop_name)
                            .unwrap();

                    let class = translator.data.registry.class(class_id).unwrap();
                    let prop = class.properties.get(&prop_name).unwrap();

                    // If a getter exists we must call it, otherwise we can load directly
                    // from the variable.
                    if let Some(id) = prop.getter {
                        let object = translator.load(&op.value);
                        let dst = translator.load(&op.return_assignment);

                        translator.builder.call(
                            id,
                            Some(object.clone()),
                            &[object.map(|Null| Literal::Null)],
                            dst,
                        );
                    } else if let Some(field) = &prop.field {
                        let object = translator.load(&op.value);
                        let dst = translator.load(&op.return_assignment);

                        translator.builder.class_load(object, dst, class.id, &field);
                    } else {
                        unreachable!();
                    }
                }
                Instruction::PropertySet(op) => {
                    let Type::Class(class_id) = translator.type_of_value(&op.object) else {
                        unreachable!();
                    };
                    let prop_name = Lowercase::new(op.property.as_str());

                    let class_id =
                        get_parent_class_with_property(translator.data, class_id, &prop_name)
                            .unwrap();

                    let class = translator.data.registry.class(class_id).unwrap();
                    let prop = class.properties.get(&prop_name).unwrap();

                    // If a setter exists we must call it, otherwise we can store directly
                    // to the variable.
                    if let Some(id) = prop.setter {
                        let object = translator.load(&op.object);
                        let value = translator.load(&op.value);

                        translator.builder.call(
                            id,
                            Some(object.clone()),
                            &[object.map(|Null| Literal::Null), value],
                            hir::Value::Const(Literal::Null),
                        );
                    } else if let Some(field) = &prop.field {
                        let object = translator.load(&op.object);
                        let value = translator.load(&op.value);

                        translator
                            .builder
                            .class_store(object, value, class.id, &field);
                    }
                }
                Instruction::Return(value) => {
                    let value = translator.load(value);
                    translator.builder.return_(value);
                    current_block_sealed = true;
                }
                Instruction::Cast(op) => {
                    perform_cast(&mut translator, &op.src, &op.dst);
                }
                Instruction::Is(op) => {
                    let Value::Ident(class) = &op.class else {
                        unreachable!();
                    };
                    let class = Lowercase::new(class.as_str());
                    let class = translator.data.registry.class_by_name(&class).unwrap();

                    let src = translator.load(&op.object);
                    let dst = translator.load(&op.return_assignment);
                    translator.builder.class_is_instance_of(src, dst, class.id);
                }
                Instruction::StructCreate(op) => {
                    let dst = translator.load(&op.return_assignment);

                    let ty = translator.ty_of_val(&op.return_assignment);

                    translator.builder.struct_create(dst, ty);
                }
                Instruction::StructGet(op) => {
                    let Value::Ident(member) = &op.member_name else {
                        unreachable!()
                    };
                    let Type::Struct(id) = translator.type_of_value(&op.struct_value) else {
                        unreachable!();
                    };

                    let field_name = Lowercase::new(&**member);

                    let object = translator.load(&op.struct_value);
                    let dst = translator.load(&op.return_assignment);

                    translator.builder.struct_load(object, dst, id, field_name);
                }
                Instruction::StructSet(op) => {
                    let Value::Ident(member) = &op.member_name else {
                        unreachable!()
                    };
                    let Type::Struct(id) = translator.type_of_value(&op.struct_value) else {
                        unreachable!();
                    };

                    let field_name = Lowercase::new(&**member);

                    let object = translator.load(&op.struct_value);
                    let value = translator.load(&op.value);

                    translator
                        .builder
                        .struct_store(object, value, id, field_name);
                }
                Instruction::ArrayLength(op) => {
                    let array = translator.load(&op.src);
                    let dst = translator.load(&op.dst);
                    translator.builder.array_length(array, dst);
                }
                Instruction::ArrayGet(op) => {
                    let array = translator.load(&op.array);
                    let index = translator.load(&op.index);
                    let dst = translator.load(&op.return_assignment);
                    translator.builder.array_get(array, index, dst);
                }
                Instruction::ArraySet(op) => {
                    let index = translator.load(&op.index);
                    let value = translator.load(&op.value);
                    let array = translator.load(&op.array);
                    translator.builder.array_set(array, index, value);
                }
                Instruction::ArrayCreate(op) => {
                    let Type::Array(elem_ty) = translator.type_of_value(&op.dst) else {
                        unreachable!();
                    };
                    let elem_ty = type_to_id(translator.data, &elem_ty);

                    let len = translator.load(&op.src);
                    let dst = translator.load(&op.dst);
                    translator.builder.array_create(len, dst, elem_ty);
                }
                Instruction::ArrayAdd(op) => {
                    let Type::Array(elem_ty) = translator.type_of_value(&op.array) else {
                        unreachable!();
                    };
                    let elem_ty = type_to_id(translator.data, &elem_ty);

                    let array = translator.load(&op.array);
                    let value = translator.load(&op.value);
                    let count = translator.load(&op.count);
                    translator.builder.array_add(array, value, count, elem_ty);
                }
                Instruction::ArrayInsert(op) => {
                    let Type::Array(elem_ty) = translator.type_of_value(&op.array) else {
                        unreachable!();
                    };
                    let elem_ty = type_to_id(translator.data, &elem_ty);

                    let array = translator.load(&op.array);
                    let value = translator.load(&op.value);
                    let index = translator.load(&op.index);
                    translator
                        .builder
                        .array_insert(array, value, index, elem_ty);
                }
                Instruction::ArrayRemove(op) => {
                    let Type::Array(elem_ty) = translator.type_of_value(&op.array_value) else {
                        unreachable!();
                    };
                    let elem_ty = type_to_id(translator.data, &elem_ty);

                    let array = translator.load(&op.array_value);
                    let index = translator.load(&op.index);
                    let count = translator.load(&op.count);
                    translator
                        .builder
                        .array_remove(array, index, count, elem_ty);
                }
                Instruction::ArrayClear(op) => {
                    let array = translator.load(&op.array);
                    translator.builder.array_clear(array);
                }
                Instruction::ArrayPop(op) => {
                    todo!();
                }
                Instruction::ArrayFind(op) | Instruction::ArrayRFind(op) => {
                    let Type::Array(elem_ty) = translator.type_of_value(&op.array_value) else {
                        unreachable!();
                    };
                    let elem_ty = type_to_id(translator.data, &elem_ty);

                    let scan_backwards = matches!(instruction, Instruction::ArrayRFind(_));

                    let array = translator.load(&op.array_value);
                    let start_index = translator.load(&op.start_index);
                    let searched_value = translator.load(&op.value);
                    let write_index = translator.load(&op.return_index);

                    translator.builder.array_find(
                        array,
                        start_index,
                        searched_value,
                        scan_backwards,
                        write_index,
                        elem_ty,
                    );
                }
                Instruction::ArrayFindStruct(op) | Instruction::ArrayRFindStruct(op) => {
                    let scan_backwards = matches!(instruction, Instruction::ArrayRFind(_));

                    let array_ty = translator.type_of_value(&op.array_value);
                    let Type::Array(struct_name) = array_ty else {
                        unreachable!();
                    };
                    let Type::Struct(struct_name) = *struct_name else {
                        unreachable!();
                    };

                    let Value::String(member_name) = &op.member_name else {
                        unreachable!();
                    };
                    let member_name = Lowercase::new(&**member_name);

                    let array = translator.load(&op.array_value);
                    let start_index = translator.load(&op.start_index);
                    let searched_value = translator.load(&op.value);
                    let write_index = translator.load(&op.return_index);

                    translator.builder.array_find_struct(
                        array,
                        start_index,
                        searched_value,
                        write_index,
                        scan_backwards,
                        struct_name,
                        member_name,
                    );
                }
                Instruction::StringConcat(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let dst = translator.load(&op.dst);
                    translator.builder.string_concat(lhs, rhs, dst);
                }
            }
        }

        // Sometimes the final return instruction is missing and we must insert it.
        // We must return the default value in this case.
        if !matches!(instructions.last(), Some(Instruction::Return(_))) && !current_block_sealed {
            let value = match func.returns {
                Type::None => Literal::Null,
                Type::Bool => Literal::Bool(false),
                Type::Int => Literal::Int(0),
                Type::Float => Literal::Float(0.0),
                Type::String => Literal::String("".to_owned()),
                Type::Class(_) | Type::Struct(_) | Type::Array(_) | Type::Var => Literal::Null,
            };

            translator.builder.return_(hir::Value::Const(value));
        }
    }
}

trait ToLiteral {
    fn to_literal(value: &Value) -> Self;
}

impl ToLiteral for Null {
    fn to_literal(value: &Value) -> Self {
        match value {
            Value::Null => Self,
            _ => unreachable!(),
        }
    }
}

impl ToLiteral for bool {
    fn to_literal(value: &Value) -> Self {
        match value {
            Value::Bool(v) => *v,
            _ => unreachable!(),
        }
    }
}

impl ToLiteral for i32 {
    fn to_literal(value: &Value) -> Self {
        match value {
            Value::Int(v) => *v,
            _ => unreachable!(),
        }
    }
}

impl ToLiteral for f32 {
    fn to_literal(value: &Value) -> Self {
        match value {
            Value::Float(v) => *v,
            _ => unreachable!(),
        }
    }
}

impl ToLiteral for String {
    fn to_literal(value: &Value) -> Self {
        match value {
            Value::String(v) => v.clone(),
            _ => unreachable!(),
        }
    }
}

impl ToLiteral for Literal {
    fn to_literal(value: &Value) -> Self {
        match value {
            Value::Null => Literal::Null,
            Value::Bool(v) => Literal::Bool(*v),
            Value::Int(v) => Literal::Int(*v),
            Value::Float(v) => Literal::Float(*v),
            Value::String(v) => Literal::String(v.clone()),
            _ => unreachable!(),
        }
    }
}

pub struct FunctionTranslator<'a> {
    data: &'a CodegenData,
    pub builder: &'a mut FunctionBuilder,
    locals: HashMap<Lowercase<String>, Local>,
    self_ptr: Option<Variable>,
    self_ty: ClassId,
}

impl<'a> FunctionTranslator<'a> {
    fn load<T>(&mut self, value: &Value) -> hir::Value<T>
    where
        T: ToLiteral,
    {
        if let Value::Ident(name) = value {
            let name = Lowercase::new(&**name);
            return self.load_var::<T>(&name);
        }

        hir::Value::Const(T::to_literal(value))
    }

    fn load_var<T>(&mut self, name: &Lowercase<str>) -> hir::Value<T>
    where
        T: ToLiteral,
    {
        match name.as_inner() {
            "self" => return hir::Value::Variable(self.self_ptr.unwrap()),
            _ => {}
        }

        if let Some(local) = self.locals.get(name) {
            return hir::Value::Variable(local.var);
        }

        let mut class_id = self.self_ty;
        loop {
            let class = self.data.registry.class(class_id).unwrap();

            if class.fields.contains_key(name) {
                return hir::Value::ClassVar {
                    object: self.self_ptr.unwrap(),
                    class: class_id,
                    field: name.to_owned(),
                };
            }

            if let Some(parent) = class.parent {
                class_id = parent;
            } else {
                break;
            }
        }

        panic!("undefined variable {}", name);
    }

    fn type_of_value(&self, value: &Value) -> Type {
        match value {
            Value::Null => Type::None,
            Value::Bool(_) => Type::Bool,
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::String(_) => Type::String,
            Value::Ident(name) => {
                let name = Lowercase::new(&**name);

                if name.as_inner() == "self" {
                    Type::Class(self.self_ty)
                } else if let Some(local) = &self.locals.get(&name) {
                    local.ty.clone()
                } else {
                    let mut class_id = self.self_ty;
                    loop {
                        let class = self.data.registry.class(class_id).unwrap();

                        if let Some(field) = class.fields.get(&name) {
                            return field.ty.clone();
                        }

                        if let Some(parent) = class.parent {
                            class_id = parent;
                        } else {
                            break;
                        }
                    }

                    panic!("undefined variable {}", name);
                }
            }
        }
    }

    fn ty(&self, ty: &Type) -> TypeId {
        type_to_id(self.data, ty)
    }

    fn ty_of_val(&self, value: &Value) -> TypeId {
        self.ty(&self.type_of_value(value))
    }
}

#[derive(Clone, Debug)]
struct Local {
    ty: Type,
    var: Variable,
}

/// Performs a runtime cast of `src` to `dst`.
fn perform_cast(translator: &mut FunctionTranslator<'_>, src_val: &Value, dst_val: &Value) {
    let src_ty = translator.type_of_value(&src_val);
    let dst_ty = translator.type_of_value(&dst_val);

    let src = translator.load(src_val);
    let dst = translator.load(dst_val);

    // Identity casts are allowed.
    if src_ty == dst_ty {
        let src = translator.load(src_val);
        let dst = translator.load(dst_val);
        translator.builder.store(src, dst);
        return;
    }

    match (src_ty, dst_ty) {
        // Cast each element individually.
        (Type::Array(src_elem_ty), Type::Array(dst_elem_ty)) => {
            perform_cast_inner(translator, &src_elem_ty, &dst_elem_ty, src, dst, true);
        }
        (src_ty, dst_ty) => perform_cast_inner(translator, &src_ty, &dst_ty, src, dst, false),
    }
}

fn perform_cast_inner(
    translator: &mut FunctionTranslator<'_>,
    src_ty: &Type,
    dst_ty: &Type,
    src_val: hir::Value<Literal>,
    dst_val: hir::Value<Literal>,
    conv_array: bool,
) {
    match (src_ty, dst_ty) {
        // Integer <-> float conversions
        (Type::Int, Type::Float) => {
            translator
                .builder
                .convert_int_to_float(src_val, dst_val, conv_array);
        }

        (Type::Float, Type::Int) => {
            translator
                .builder
                .convert_float_to_int(src_val, dst_val, conv_array);
        }
        // Bool conversions
        (Type::Int, Type::Bool) => {
            translator
                .builder
                .convert_int_to_bool(src_val, dst_val, conv_array);
        }
        (Type::Float, Type::Bool) => {
            translator
                .builder
                .convert_float_to_bool(src_val, dst_val, conv_array);
        }
        (Type::String | Type::Class(_) | Type::Struct(_) | Type::Array(_), Type::Bool) => {
            // Bool value is true if addr is non-null.
            let src = src_val.map(|v| v.to_null().unwrap());
            let dst = dst_val.map(|v| v.to_bool().unwrap());
            translator.builder.compare_addr(
                src,
                hir::Value::Const(Null),
                dst,
                AddrCompareOp::NotEqual,
            );
        }
        (Type::Bool, Type::Int) => {
            translator
                .builder
                .convert_bool_to_int(src_val, dst_val, conv_array);
        }
        (Type::Bool, Type::Float) => {
            translator
                .builder
                .convert_bool_to_float(src_val, dst_val, conv_array);
        }
        (Type::Class(src_class), Type::Class(dst_class)) => {
            let src = src_val.map(|v| v.to_null().unwrap());
            let dst = dst_val.map(|v| v.to_null().unwrap());

            // Try upcasting.
            let mut class_id = *src_class;
            loop {
                if class_id == *dst_class {
                    translator
                        .builder
                        .class_upcast(src, dst, *src_class, *dst_class);

                    return;
                }

                let class = translator.data.registry.class(class_id).unwrap();
                if let Some(parent) = &class.parent {
                    class_id = *parent;
                } else {
                    break;
                }
            }

            // Try downcasting.
            // Casting unrelated class will never be successful.
            // We validate that `src_class` is a subtype of `dst_class` before checking.
            let mut class_id = *dst_class;
            loop {
                if class_id == *src_class {
                    translator.builder.class_downcast(
                        src,
                        dst,
                        src_class.to_owned(),
                        dst_class.to_owned(),
                    );
                    return;
                }

                let class = translator.data.registry.class(class_id).unwrap();
                if let Some(parent) = &class.parent {
                    class_id = *parent;
                } else {
                    break;
                }
            }

            unreachable!("cannot cast unrelated classes")
        }
        (Type::Var, dst_ty) => {
            let src = src_val.map(|v| v.to_null().unwrap());
            let dst = dst_val;

            match dst_ty {
                Type::None => {
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::TypeId(translator.data.resources.primitives.none),
                        None,
                        conv_array,
                    );
                }
                Type::Bool => {
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::TypeId(translator.data.resources.primitives.bool),
                        None,
                        conv_array,
                    );
                }
                Type::Int => {
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::TypeId(translator.data.resources.primitives.int),
                        None,
                        conv_array,
                    );
                }
                Type::Float => {
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::TypeId(translator.data.resources.primitives.float),
                        None,
                        conv_array,
                    );
                }
                Type::String => {
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::TypeId(translator.data.resources.primitives.string),
                        None,
                        conv_array,
                    );
                }
                Type::Class(class) => {
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::Class,
                        Some(class.clone()),
                        conv_array,
                    );
                }
                Type::Struct(name) => {
                    let id = *translator.data.struct_types.get(name).unwrap();
                    translator.builder.extract_var(
                        src,
                        dst,
                        ObjectTag::TypeId(id),
                        None,
                        conv_array,
                    );
                }
                Type::Array(_) => {
                    todo!();
                }
                Type::Var => {}
            }
        }
        (src_ty, Type::Var) => {
            let src = src_val;
            let dst = dst_val.map(|v| v.to_null().unwrap());

            let tag = match src_ty {
                Type::None => ObjectTag::TypeId(translator.data.resources.primitives.none),
                Type::Int => ObjectTag::TypeId(translator.data.resources.primitives.int),
                Type::Bool => ObjectTag::TypeId(translator.data.resources.primitives.bool),
                Type::Float => ObjectTag::TypeId(translator.data.resources.primitives.float),
                Type::String => ObjectTag::TypeId(translator.data.resources.primitives.string),
                Type::Class(_) => ObjectTag::Class,
                Type::Struct(name) => {
                    let id = *translator.data.struct_types.get(name).unwrap();
                    ObjectTag::TypeId(id)
                }
                Type::Var => unreachable!(),
                x => todo!("{:?}", x),
            };

            translator.builder.create_var(src, dst, tag, conv_array);
        }
        // String formatting conversions
        (Type::Bool, Type::String) => {
            let src = src_val.map(|v| v.to_bool().unwrap());
            let dst = dst_val.map(|v| v.to_string().unwrap());
            translator.builder.bool_to_string(src, dst);
        }
        (Type::Int, Type::String) => {
            let src = src_val.map(|v| v.to_int().unwrap());
            let dst = dst_val.map(|v| v.to_string().unwrap());
            translator.builder.int_to_string(src, dst);
        }
        (Type::Float, Type::String) => {
            let src = src_val.map(|v| v.to_float().unwrap());
            let dst = dst_val.map(|v| v.to_string().unwrap());
            translator.builder.float_to_string(src, dst);
        }
        (Type::Class(_), Type::String) => {
            translator
                .builder
                .object_to_string(src_val, dst_val, conv_array);
        }
        (Type::Struct(_) | Type::Array(_), Type::String) => {
            // FIXME: Implement
            assert!(!conv_array);
            translator
                .builder
                .store(hir::Value::Const(Literal::String("".to_owned())), dst_val);
        }
        // A null pointer.
        (Type::None, Type::Class(_)) => {
            translator
                .builder
                .store(hir::Value::Const(Literal::Null), dst_val);
        }
        (src_ty, dst_ty) => todo!("{:?} {:?}", src_ty, dst_ty),
    }
}

pub fn type_to_id(data: &CodegenData, ty: &Type) -> TypeId {
    match ty {
        Type::None => data.resources.primitives.none,
        Type::Bool => data.resources.primitives.bool,
        Type::Int => data.resources.primitives.int,
        Type::Float => data.resources.primitives.float,
        Type::String => data.resources.primitives.string,
        Type::Var => data.resources.primitives.var,
        Type::Class(id) => data.class_types[id],
        Type::Struct(id) => data.struct_types[id],
        Type::Array(_) => data.resources.primitives.array,
    }
}
