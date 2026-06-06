use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{Compiler, FuncId, Signature, Type as JitType};
use openfo4_pex::{Function, Instruction, Value};

use crate::compiler::NativeFunction;
use crate::compiler::codegen::hir::{
    self, AddrCompareOp, FunctionBuilder, Literal, Null, ScalarCompareOp, StringCompareOp, Variable,
};
use crate::compiler::codegen::pex::cfg::ControlFlowGraph;
use crate::compiler::codegen::trace::Span;
use crate::compiler::codegen::trampoline::generate_guest_to_host_trampoline;
use crate::compiler::codegen::{
    ClassDef, CodegenData, Error, FunctionDef, PscSignature, Type, get_parent_class_with_property,
};
use crate::compiler::resources::TypeId;
use crate::runtime::object::ObjectTag;

#[derive(Debug)]
pub struct FunctionResult {
    pub id: FuncId,
    pub signature: Signature,
    pub psc_signature: PscSignature,
}

pub fn declare_function(
    data: &CodegenData,
    class: &ClassDef,
    function: &FunctionDef,
    compiler: &mut Compiler,
    use_native_stubs: bool,
) -> Result<FuncId, Error> {
    let signature = translate_function_signature(data, &function.function);

    if function.function.is_native {
        tracing::debug!(
            "compiling {}.{} (native)",
            function.class,
            function.function.name
        );

        let native = match function.native.as_ref() {
            Some(native) => {
                if signature != native.signature {
                    return Err(Error::NativeFuncSignatureMissmatch {
                        name: format!("{}.{}", function.class, function.function.name),
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
                    function.class, function.function.name
                )));
            }
        };

        let id = generate_guest_to_host_trampoline(
            compiler,
            data,
            &signature,
            &native,
            &function.class_name_debug,
            &function.function.name,
        );
        Ok(id)
    } else {
        let id = compiler.declare_function(signature);
        Ok(id)
    }
}

pub fn translate_function_psc_signature(data: &CodegenData, func: &FunctionDef) -> PscSignature {
    let mut sig = PscSignature::default();
    for param in func.function.params.iter() {
        let ty_name = Lowercase::new(&*param.ty);

        let ty = if ty_name.as_inner().ends_with("[]") {
            data.resources.primitives.array
        } else {
            *data.resources.string_to_type.lock().get(&ty_name).unwrap()
        };
        sig.params.push(ty);
    }

    if !func.function.return_ty.eq_ignore_ascii_case("none") {
        let ty_name = Lowercase::new(&*func.function.return_ty);

        let ty = if ty_name.as_inner().ends_with("[]") {
            data.resources.primitives.array
        } else {
            *data.resources.string_to_type.lock().get(&ty_name).unwrap()
        };
        sig.returns.push(ty);
    }

    sig
}

pub fn translate_function(
    data: &CodegenData,
    class: &ClassDef,
    function: &FunctionDef,
    compiler: &mut Compiler,
) -> FunctionResult {
    let signature = translate_function_signature(data, &function.function);

    let func_id = function.func_id.unwrap();

    let sig = translate_function_psc_signature(data, function);

    let func_id = if function.function.is_native {
        func_id
    } else {
        tracing::debug!("compiling {}.{}", function.class, function.function.name);

        let mut builder = FunctionBuilder::default();

        translate_function_body(
            data,
            class,
            &function.class_name_debug,
            &function.function,
            &mut builder,
        );

        let func = builder.clone().finalize();

        let span = Span::new(
            data,
            &class.class.as_ref().unwrap().name,
            &function.function.name,
        );

        let builder = compiler.define_function(func_id);
        func.lower(data, builder, span);

        func_id
    };

    FunctionResult {
        id: func_id,
        signature,
        psc_signature: sig,
    }
}

pub fn translate_function_signature(data: &CodegenData, function: &Function) -> Signature {
    let mut params = Vec::new();
    let mut returns = Vec::new();

    // All functions must carry the global context pointer.
    params.push(JitType::Pointer);

    // For non-static function (methods) the first argument is the object pointer.
    if !function.is_static {
        params.push(JitType::Pointer);
    }

    for param in &function.params {
        let ty = data.resolve_ty_jit(Lowercase::new(&*param.ty).as_str());
        params.push(ty);
    }

    if !function.return_ty.is_empty() && function.return_ty.to_lowercase() != "none" {
        let ty = data.resolve_ty_jit(Lowercase::new(&*function.return_ty).as_str());
        returns.push(ty);
    }

    // VM error value
    returns.push(JitType::I32);

    Signature { params, returns }
}

fn translate_function_body(
    data: &CodegenData,
    class: &ClassDef,
    class_name: &str,
    function: &Function,
    builder: &mut FunctionBuilder,
) {
    let self_ty = *data
        .resources
        .string_to_type
        .lock()
        .get(&Lowercase::new(&*class_name))
        .unwrap();
    let self_ptr = (!function.is_static).then(|| builder.add_param(self_ty));

    let mut locals = HashMap::with_capacity(function.locals.len());

    // Move function parameters into stack slots.
    for param in function.params.iter() {
        let ty_name = Lowercase::new(&*param.ty);

        let ty = if ty_name.as_inner().ends_with("[]") {
            data.resources.primitives.array
        } else {
            *data.resources.string_to_type.lock().get(&ty_name).unwrap()
        };
        let variable = builder.add_param(ty);

        locals.insert(Lowercase::new(&*param.name), Local { ty_name, variable });
    }

    // Create stack slots for local variables.
    for local in &function.locals {
        let ty_name = Lowercase::new(&*local.ty);

        let ty = if ty_name.as_inner().ends_with("[]") {
            data.resources.primitives.array
        } else {
            *data.resources.string_to_type.lock().get(&ty_name).unwrap()
        };
        let variable = builder.declare_variable(ty);

        let src = match data.resolve_ty(&ty_name) {
            Type::None => hir::Value::Const(Literal::Null),
            Type::Bool => hir::Value::Const(Literal::Bool(false)),
            Type::Int => hir::Value::Const(Literal::Int(0)),
            Type::Float => hir::Value::Const(Literal::Float(0.0)),
            Type::String => hir::Value::Const(Literal::String("".to_owned())),
            Type::Class(_) | Type::Struct(_) | Type::Array(_) | Type::Var => {
                hir::Value::Const(Literal::Null)
            }
        };
        let dst = hir::Value::Variable(variable);
        builder.store(src, dst);

        locals.insert(Lowercase::new(&*local.name), Local { ty_name, variable });
    }

    let mut fields = HashMap::new();
    for (name, _) in &class.fields {
        fields.insert(name.as_str(), Field {});
    }

    if !function.return_ty.eq_ignore_ascii_case("none") {
        let ty_name = Lowercase::new(&*function.return_ty);

        let ty = if ty_name.as_inner().ends_with("[]") {
            data.resources.primitives.array
        } else {
            *data.resources.string_to_type.lock().get(&ty_name).unwrap()
        };

        builder.set_returns(ty);
    }

    let mut translator = FunctionTranslator {
        data,
        builder,
        locals,
        self_ptr,
        self_ty: &class.name,
    };

    let cfg = ControlFlowGraph::new(function);

    let mut labels = HashMap::new();
    for (block_id, _) in cfg.blocks() {
        let label = translator.builder.create_label();
        labels.insert(block_id, label);
    }

    for (block_id, instructions) in cfg.blocks() {
        let mut current_block_sealed = false;

        let label = *labels.get(&block_id).unwrap();
        translator.builder.set_label(label);

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
                Instruction::Not(op) => match translator.get_type(&op.src) {
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
                | Instruction::CmpGe(op) => match translator.get_type(&op.lhs) {
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
                    let target = *labels.get(&succ[0]).unwrap();
                    translator.builder.jump(target);
                    current_block_sealed = true;
                }
                Instruction::BranchIfTrue(op) | Instruction::BranchIfFalse(op) => {
                    let succ = cfg.successors(block_id);
                    let then_target = *labels.get(&succ[0]).unwrap();
                    let else_target = *labels.get(&succ[1]).unwrap();

                    let condition = translator.load(&op.condition);

                    if matches!(instruction, Instruction::BranchIfTrue(_)) {
                        translator.builder.brif_true(condition, then_target);
                    } else {
                        translator.builder.brif_false(condition, then_target);
                    }

                    translator.builder.jump(else_target);
                    current_block_sealed = true;
                }
                Instruction::CallMethod(op) => {
                    let class_name = translator.get_ty_name(&op.object);
                    let function_name = Lowercase::new(op.function_name.as_str());

                    let callee = data
                        .get_function_id(class_name.as_str(), function_name.as_str())
                        .unwrap();

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
                    let class_name = translator.get_ty_name(&Value::Ident("self".to_owned()));
                    let class = translator.data.classes.get(class_name.as_str()).unwrap();
                    let parent = class.parent.as_ref().unwrap();
                    let function_name = Lowercase::new(op.function.as_str());

                    let callee = data
                        .get_function_id(parent.as_str(), function_name.as_str())
                        .unwrap();

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
                    let function_name = Lowercase::new(op.function.as_str());

                    let args = op
                        .args
                        .iter()
                        .map(|v| translator.load(v))
                        .collect::<Vec<_>>();
                    let dst = translator.load(&op.return_assignment);

                    let callee = data.get_function_id(&class_name, &function_name).unwrap();

                    translator.builder.call(callee, None, &args, dst);
                }
                Instruction::PropertyGet(op) => {
                    let class_name = translator.get_ty_name(&op.value);
                    let prop_name = Lowercase::new(op.property.as_str());

                    // If a getter exists we must call it, otherwise we can load directly
                    // from the variable.
                    if let Some(id) = translator
                        .data
                        .get_property_getter_id(&class_name, &prop_name)
                    {
                        let object = translator.load(&op.value);
                        let dst = translator.load(&op.return_assignment);

                        translator.builder.call(
                            id,
                            Some(object.clone()),
                            &[object.map(|Null| Literal::Null)],
                            dst,
                        );
                    } else {
                        let class_name =
                            get_parent_class_with_property(&data.classes, &class_name, &prop_name)
                                .unwrap();

                        let class = data.classes.get(class_name).unwrap();
                        let prop = class.properties.get(&prop_name).unwrap();
                        let var_name = prop.variable.as_ref().unwrap();

                        let object = translator.load(&op.value);
                        let dst = translator.load(&op.return_assignment);

                        translator
                            .builder
                            .class_load(object, dst, class_name, &var_name);
                    }
                }
                Instruction::PropertySet(op) => {
                    let class_name = translator.get_ty_name(&op.object);
                    let prop_name = Lowercase::new(op.property.as_str());

                    // If a setter exists we must call it, otherwise we can store directly
                    // to the variable.
                    if let Some(id) = translator
                        .data
                        .get_property_setter_id(&class_name, &prop_name)
                    {
                        let object = translator.load(&op.object);
                        let value = translator.load(&op.value);

                        translator.builder.call(
                            id,
                            Some(object.clone()),
                            &[object.map(|Null| Literal::Null), value],
                            hir::Value::Const(Literal::Null),
                        );
                    } else {
                        let class_name =
                            get_parent_class_with_property(&data.classes, &class_name, &prop_name)
                                .unwrap();

                        let class = data.classes.get(class_name).unwrap();
                        let prop = class.properties.get(&prop_name).unwrap();
                        let var_name = prop.variable.as_ref().unwrap();

                        let object = translator.load(&op.object);
                        let value = translator.load(&op.value);

                        translator
                            .builder
                            .class_store(object, value, class_name, &var_name);
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

                    let src = translator.load(&op.object);
                    let dst = translator.load(&op.return_assignment);
                    translator.builder.class_is_instance_of(src, dst, class);
                }
                Instruction::StructCreate(op) => {
                    let dst = translator.load(&op.return_assignment);

                    let ty = translator.get_type(&op.return_assignment);
                    let ty = translator.get_type_id(&ty);

                    translator.builder.struct_create(dst, ty);
                }
                Instruction::StructGet(op) => {
                    let Value::Ident(member) = &op.member_name else {
                        unreachable!()
                    };

                    let struct_name = translator.get_ty_name(&op.struct_value);
                    let field_name = Lowercase::new(&**member);

                    let object = translator.load(&op.struct_value);
                    let dst = translator.load(&op.return_assignment);

                    translator
                        .builder
                        .struct_load(object, dst, struct_name, field_name);
                }
                Instruction::StructSet(op) => {
                    let Value::Ident(member) = &op.member_name else {
                        unreachable!()
                    };

                    let struct_name = translator.get_ty_name(&op.struct_value);
                    let field_name = Lowercase::new(&**member);

                    let object = translator.load(&op.struct_value);
                    let value = translator.load(&op.value);

                    translator
                        .builder
                        .struct_store(object, value, struct_name, field_name);
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
                    let Type::Array(elem_ty) = translator.get_type(&op.dst) else {
                        unreachable!();
                    };

                    let elem_ty = translator.get_type_id(&elem_ty);

                    let len = translator.load(&op.src);
                    let dst = translator.load(&op.dst);
                    translator.builder.array_create(len, dst, elem_ty);
                }
                Instruction::ArrayAdd(op) => {
                    let Type::Array(elem_ty) = translator.get_type(&op.array) else {
                        unreachable!();
                    };
                    let elem_ty = translator.get_type_id(&elem_ty);

                    let array = translator.load(&op.array);
                    let value = translator.load(&op.value);
                    let count = translator.load(&op.count);
                    translator.builder.array_add(array, value, count, elem_ty);
                }
                Instruction::ArrayInsert(op) => {
                    let Type::Array(elem_ty) = translator.get_type(&op.array) else {
                        unreachable!();
                    };
                    let elem_ty = translator.get_type_id(&elem_ty);

                    let array = translator.load(&op.array);
                    let value = translator.load(&op.value);
                    let index = translator.load(&op.index);
                    translator
                        .builder
                        .array_insert(array, value, index, elem_ty);
                }
                Instruction::ArrayRemove(op) => {
                    let Type::Array(elem_ty) = translator.get_type(&op.array_value) else {
                        unreachable!();
                    };
                    let elem_ty = translator.get_type_id(&elem_ty);

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
                    let Type::Array(elem_ty) = translator.get_type(&op.array_value) else {
                        unreachable!();
                    };

                    let elem_ty = translator.get_type_id(&elem_ty);

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

                    let array_ty = translator.get_type(&op.array_value);
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
            let return_ty = Lowercase::new(&*function.return_ty);
            let value = match translator.data.resolve_ty(&return_ty) {
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
    self_ty: &'a Lowercase<str>,
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
            return hir::Value::Variable(local.variable);
        }

        let mut class = self.data.get_class(&self.self_ty).unwrap();
        loop {
            if let Some(_) = class.fields.get(name) {
                let self_ptr = self.self_ptr.unwrap();

                return hir::Value::ClassVar {
                    object: self_ptr,
                    class: class.name.clone(),
                    field: name.to_owned(),
                };
            }

            if let Some(parent) = &class.parent {
                class = self.data.get_class(&parent).unwrap();
            } else {
                break;
            }
        }

        panic!("undefined variable {}", name);
    }

    pub fn get_type(&self, value: &Value) -> Type {
        self.data.resolve_ty(&self.get_ty_name(value))
    }

    pub fn get_type_id(&self, ty: &Type) -> TypeId {
        match ty {
            Type::None => self.data.resources.primitives.none,
            Type::Bool => self.data.resources.primitives.bool,
            Type::Int => self.data.resources.primitives.int,
            Type::Float => self.data.resources.primitives.float,
            Type::String => self.data.resources.primitives.string,
            Type::Struct(name) => {
                let s = self.data.structs.get(name).unwrap();
                s.ty
            }
            Type::Class(name) => {
                let c = self.data.classes.get(name).unwrap();
                c.type_id
            }
            Type::Array(_) => self.data.resources.primitives.array,
            Type::Var => self.data.resources.primitives.var,
        }
    }

    pub fn get_ty_name(&self, value: &Value) -> Lowercase<String> {
        match value {
            Value::Null => Lowercase::new("none"),
            Value::Bool(_) => Lowercase::new("bool"),
            Value::Int(_) => Lowercase::new("int"),
            Value::Float(_) => Lowercase::new("float"),
            Value::String(_) => Lowercase::new("string"),
            Value::Ident(name) => {
                let name = Lowercase::new(name.as_str());

                if name.as_inner() == "self" {
                    return self.self_ty.to_owned();
                };

                if let Some(local) = self.locals.get(&name) {
                    return local.ty_name.clone();
                }

                let mut class = self.data.get_class(&self.self_ty).unwrap();
                loop {
                    if let Some(ty) = class.fields.get(&name) {
                        return ty.clone();
                    }

                    if let Some(parent) = &class.parent {
                        class = self.data.get_class(&parent).unwrap();
                    } else {
                        break;
                    }
                }

                panic!("undefiend variable {}", name);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Field {}

#[derive(Clone, Debug)]
struct Local {
    ty_name: Lowercase<String>,
    variable: Variable,
}

/// Performs a runtime cast of `src` to `dst`.
fn perform_cast(translator: &mut FunctionTranslator<'_>, src_val: &Value, dst_val: &Value) {
    let src_ty = translator.get_type(&src_val);
    let dst_ty = translator.get_type(&dst_val);

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
            let mut class_name = src_class;
            loop {
                if class_name == dst_class {
                    translator.builder.class_upcast(
                        src,
                        dst,
                        src_class.to_owned(),
                        dst_class.to_owned(),
                    );

                    return;
                }

                let class = translator.data.classes.get(class_name).unwrap();
                if let Some(parent) = &class.parent {
                    class_name = parent;
                } else {
                    break;
                }
            }

            // Try downcasting.
            // Casting unrelated class will never be successful.
            // We validate that `src_class` is a subtype of `dst_class` before checking.
            let mut class_name = dst_class;
            loop {
                if class_name == src_class {
                    translator.builder.class_downcast(
                        src,
                        dst,
                        src_class.to_owned(),
                        dst_class.to_owned(),
                    );
                    return;
                }

                let class = translator.data.classes.get(class_name).unwrap();
                if let Some(parent) = &class.parent {
                    class_name = parent;
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
                    let id = translator.data.structs.get(name).unwrap().ty;
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
                    let id = translator.data.structs.get(name).unwrap().ty;
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
            // translator.builder.iconst(JitType::Pointer, 0);
            todo!();
        }
        (src_ty, dst_ty) => todo!("{:?} {:?}", src_ty, dst_ty),
    }
}
