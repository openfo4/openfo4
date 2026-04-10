mod stack;
mod string;
mod vtable;

use std::mem::offset_of;

use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{
    Block, Compiler, FloatCompareOp, FuncId, FunctionBuilder, IntCompareOp, Signature,
    Type as JitType, Value as JitValue, Variable as JitVariable,
};
use openfo4_pex::{Function, Instruction, Value};

use crate::compiler::codegen::builtin::call_builtin;
use crate::compiler::codegen::function::stack::{
    pop_stack_frame, prepare_stack_for_call, push_stack_frame,
};
use crate::compiler::codegen::function_props::{
    FunctionProps, StackFrame, Stage, StageVariable, function_properties,
};
use crate::compiler::codegen::trace::Span;
use crate::compiler::codegen::trampoline::generate_guest_to_host_trampoline;
use crate::compiler::codegen::{
    ClassDef, CodegenData, Error, FunctionDef, Type, get_parent_class_with_property,
};
use crate::compiler::resources::TypeId;
use crate::compiler::{NativeFunction, resources};
use crate::runtime::object::{ObjectTag, VarObject};
use crate::runtime::{ArrayObject, RESULT_ERROR, RESULT_OK, RawStack};

#[derive(Debug)]
pub struct FunctionResult {
    pub id: FuncId,
    pub signature: Signature,
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

pub fn translate_function(
    data: &CodegenData,
    class: &ClassDef,
    function: &FunctionDef,
    compiler: &mut Compiler,
) -> FunctionResult {
    let signature = translate_function_signature(data, &function.function);
    let props = function_properties(data, function);

    let func_id = function.func_id.unwrap();

    let func_id = if function.function.is_native {
        func_id
    } else {
        tracing::debug!("compiling {}.{}", function.class, function.function.name);

        let builder = compiler.define_function(func_id);
        translate_function_body(
            data,
            class,
            &function.class_name_debug,
            &function.function,
            &props,
            builder,
        );

        func_id
    };

    FunctionResult {
        id: func_id,
        signature,
    }
}

fn translate_function_signature(data: &CodegenData, function: &Function) -> Signature {
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
    props: &FunctionProps,
    mut builder: FunctionBuilder<'_>,
) {
    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx_ptr = builder.block_params(entry_block)[0];

    let mut span = Span::new(data, class_name, &function.name);
    span.enter(data, &mut builder, ctx_ptr);

    let self_ptr = builder.block_params(entry_block).get(1).copied();
    let params_offset = if !function.is_static {
        debug_assert!(self_ptr.is_some());
        2
    } else {
        1
    };

    let mut locals = HashMap::with_capacity(function.locals.len());

    // Move function parameters into stack slots.
    for (index, param) in function.params.iter().enumerate() {
        let ty_name = Lowercase::new(&*param.ty);

        let ty = data.resolve_ty_jit(ty_name.as_str());

        let value = builder.block_params(entry_block)[params_offset + index];
        let variable = builder.def_var(ty, value);

        locals.insert(
            Lowercase::new(&*param.name),
            Local {
                ty,
                ty_name,
                variable,
            },
        );
    }

    // Create stack slots for local variables.
    for local in &function.locals {
        let ty_name = Lowercase::new(&*local.ty);

        let ty = data.resolve_ty_jit(ty_name.as_str());

        let value = match data.resolve_ty(ty_name.as_str()) {
            Type::None | Type::Bool | Type::Int => builder.iconst(JitType::I32, 0),
            Type::Float => builder.fconst(JitType::F32, 0.0),
            Type::String | Type::Class(_) | Type::Array(_) | Type::Struct(_) | Type::Var => {
                builder.iconst(JitType::Pointer, 0)
            }
        };

        let variable = builder.def_var(ty, value);

        locals.insert(
            Lowercase::new(&*local.name),
            Local {
                ty,
                ty_name,
                variable,
            },
        );
    }

    let mut fields = HashMap::new();
    for (name, ty) in &class.fields {
        fields.insert(
            name.as_str(),
            Field {
                ty: data.resolve_ty_jit(&ty),
                ty_name: ty.clone(),
            },
        );
    }

    let mut translator = FunctionTranslator {
        function,
        data,
        builder,
        locals,
        fields,
        self_ptr,
        self_ty: &class.name,
        span: &mut span,
        ctx_ptr,
        stack_frame: &props.stack_frame,
    };

    let start_block = translator.builder.create_block(&[]);

    // Setup blocks
    let mut blocks = HashMap::new();
    blocks.insert(0, start_block);

    let mut next_stage = 0;
    let mut resume_points = HashMap::new();

    let result_ok = translator.builder.iconst(JitType::I32, RESULT_OK);

    for (index, instruction) in function.instructions.iter().enumerate() {
        match instruction {
            Instruction::Branch(offset) => {
                let offset = match offset {
                    Value::Int(offset) => *offset,
                    _ => unreachable!(),
                };

                let addr = usize::min(
                    (index as i32 + offset) as usize,
                    function.instructions.len(),
                );

                if !blocks.contains_key(&addr) {
                    let block = translator.builder.create_block(&[]);
                    blocks.insert(addr, block);
                }
            }
            Instruction::BranchIfTrue(op) | Instruction::BranchIfFalse(op) => {
                let offset = match &op.offset {
                    Value::Int(offset) => *offset,
                    _ => unreachable!(),
                };

                let addr = usize::min(
                    (index as i32 + offset) as usize,
                    function.instructions.len(),
                );

                if !blocks.contains_key(&addr) {
                    let then_block = translator.builder.create_block(&[]);
                    blocks.insert(addr, then_block);
                }

                let next_addr = index + 1;
                if !blocks.contains_key(&next_addr) {
                    let else_block = translator.builder.create_block(&[]);
                    blocks.insert(next_addr, else_block);
                }
            }
            Instruction::CallMethod(_)
            | Instruction::CallParent(_)
            | Instruction::CallStatic(_) => {
                let stage = &props.stages[next_stage];
                next_stage += 1;

                let block = blocks
                    .entry(index)
                    .or_insert_with(|| translator.builder.create_block(&[]));

                resume_points.insert(stage.index, *block);
            }
            Instruction::PropertyGet(op) => {
                let Value::Ident(prop_name) = &op.property else {
                    unreachable!();
                };

                let class_name = translator.get_ty_name(&op.value);
                let prop_name = Lowercase::new(prop_name.as_str());

                if translator
                    .data
                    .get_property_getter_id(&class_name, &prop_name)
                    .is_some()
                {
                    let stage = &props.stages[next_stage];
                    next_stage += 1;

                    let block = blocks
                        .entry(index)
                        .or_insert_with(|| translator.builder.create_block(&[]));

                    resume_points.insert(stage.index, *block);
                }
            }
            Instruction::PropertySet(op) => {
                let Value::Ident(prop_name) = &op.property else {
                    unreachable!();
                };

                let class_name = translator.get_ty_name(&op.object);
                let prop_name = Lowercase::new(prop_name.as_str());

                if translator
                    .data
                    .get_property_setter_id(&class_name, &prop_name)
                    .is_some()
                {
                    let stage = &props.stages[next_stage];
                    next_stage += 1;

                    let block = blocks
                        .entry(index)
                        .or_insert_with(|| translator.builder.create_block(&[]));

                    resume_points.insert(stage.index, *block);
                }
            }
            _ => {}
        }
    }

    resumse_stack(&mut translator, &props.stages, &resume_points, start_block);
    translator.builder.switch_to_block(start_block);

    push_stack_frame(&mut translator);

    let mut next_stage = 0;

    let mut current_block = start_block;
    for (index, instruction) in function.instructions.iter().enumerate() {
        if let Some(block) = blocks.get(&index)
            && *block != current_block
        {
            if !translator.builder.is_current_block_full() {
                translator.builder.jump(*block, &[]);
            }

            translator.builder.switch_to_block(*block);
            current_block = *block;
        }

        // Sometimes there are dead instructions present after returns.
        if translator.builder.is_current_block_full() {
            continue;
        }

        match instruction {
            Instruction::Nop => {}
            Instruction::IAdd(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.iadd(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::ISub(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.isub(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::IMul(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.imul(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::IDiv(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.sdiv(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::IMod(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.srem(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::INeg(op) => {
                let src = translator.load(&op.src);
                let res = translator.builder.ineg(src);
                translator.store(&op.dst, res);
            }
            Instruction::FAdd(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.fadd(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::FSub(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.fsub(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::FMul(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.fmul(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::FDiv(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.fdiv(lhs, rhs);
                translator.store(&op.dst, res);
            }
            Instruction::FNeg(op) => {
                let src = translator.load(&op.src);
                let res = translator.builder.fneg(src);
                translator.store(&op.dst, res);
            }
            Instruction::Not(op) => {
                // This instruction can operate on any bool-castable types,
                // in which case it will be casted to a bool before the not
                // operation is executed.
                // The resulting type is always bool.
                let src = if translator.get_type(&op.src) == Type::Bool {
                    translator.load(&op.src)
                } else {
                    perform_cast(
                        &mut translator,
                        &op.src,
                        // Use `dst` as a "temporary" boolean variable.
                        // (Needed to provide a type for `perform_cast`)
                        &op.dst,
                        &mut current_block,
                    )
                };

                let res = translator.builder.bnot(src);
                translator.store(&op.dst, res);
            }
            Instruction::Store(op) => {
                let value = translator.load(&op.src);
                translator.store(&op.dst, value);
            }
            Instruction::CmpEq(op)
            | Instruction::CmpLt(op)
            | Instruction::CmpLe(op)
            | Instruction::CmpGt(op)
            | Instruction::CmpGe(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);

                let res = match translator.get_type(&op.lhs) {
                    Type::Float => {
                        let cmp_op = match instruction {
                            Instruction::CmpEq(_) => FloatCompareOp::Equal,
                            Instruction::CmpLt(_) => FloatCompareOp::LessThan,
                            Instruction::CmpLe(_) => FloatCompareOp::LessThanOrEqual,
                            Instruction::CmpGt(_) => FloatCompareOp::GreaterThan,
                            Instruction::CmpGe(_) => FloatCompareOp::GreaterThanOrEqual,
                            _ => unreachable!(),
                        };

                        translator.builder.fcmp(lhs, rhs, cmp_op)
                    }
                    Type::String => string::string_cmpeq(&mut translator, lhs, rhs),
                    _ => {
                        let cmp_op = match instruction {
                            Instruction::CmpEq(_) => IntCompareOp::Equal,
                            Instruction::CmpLt(_) => IntCompareOp::SignedLessThan,
                            Instruction::CmpLe(_) => IntCompareOp::SignedLessThanOrEqual,
                            Instruction::CmpGt(_) => IntCompareOp::SignedGreaterThan,
                            Instruction::CmpGe(_) => IntCompareOp::SignedGreaterThanOrEqual,
                            _ => unreachable!(),
                        };

                        translator.builder.icmp(lhs, rhs, cmp_op)
                    }
                };

                let res = translator.builder.uextend(res, JitType::I32);
                translator.store(&op.dst, res);
            }
            Instruction::Branch(offset) => {
                let offset = match offset {
                    Value::Int(offset) => *offset,
                    _ => unreachable!(),
                };

                let addr = usize::min(
                    (index as i32 + offset) as usize,
                    function.instructions.len(),
                );

                let block = *blocks.get(&addr).unwrap();
                translator.builder.jump(block, &[]);
            }
            Instruction::BranchIfTrue(op) => {
                let offset = match &op.offset {
                    Value::Int(offset) => *offset,
                    _ => unreachable!(),
                };

                let addr = usize::min(
                    (index as i32 + offset) as usize,
                    function.instructions.len(),
                );
                let then_block = *blocks.get(&addr).unwrap();
                let else_block = *blocks.get(&(index + 1)).unwrap();

                let condition = translator.load(&op.condition);

                translator
                    .builder
                    .brif(condition, then_block, &[], else_block, &[]);
            }
            Instruction::BranchIfFalse(op) => {
                let offset = match &op.offset {
                    Value::Int(offset) => *offset,
                    _ => unreachable!(),
                };

                let addr = usize::min(
                    (index as i32 + offset) as usize,
                    function.instructions.len(),
                );
                let else_block = *blocks.get(&addr).unwrap();
                let then_block = *blocks.get(&(index + 1)).unwrap();

                let condition = translator.load(&op.condition);

                translator
                    .builder
                    .brif(condition, then_block, &[], else_block, &[]);
            }
            Instruction::CallMethod(op) => {
                let function_name = match &op.function_name {
                    Value::Ident(v) => v,
                    _ => unreachable!(),
                };

                let class_name = translator.get_ty_name(&op.object);
                let function_name = Lowercase::new(function_name.as_str());

                let func = data
                    .get_function(class_name.as_str(), function_name.as_str())
                    .unwrap();

                let mut args = op
                    .args
                    .iter()
                    .map(|v| translator.load(v))
                    .collect::<Vec<_>>();

                // In our ABI the first argument is the object pointer.
                let object = translator.load(&op.object);

                // Abort if object == null
                let abort_block = translator.builder.create_block(&[]);
                let continue_block = translator.builder.create_block(&[]);
                translator
                    .builder
                    .brif(object, continue_block, &[], abort_block, &[]);

                translator.builder.switch_to_block(abort_block);
                abort_function(&mut translator);

                translator.builder.switch_to_block(continue_block);
                current_block = continue_block;

                args.insert(0, object);

                args.insert(0, ctx_ptr);

                let stage = &props.stages[next_stage];
                next_stage += 1;

                if let Some(res) = perform_checked_call(
                    &mut translator,
                    function,
                    func,
                    &args,
                    &mut current_block,
                    stage,
                    true,
                ) {
                    translator.store(&op.return_assignment, res);
                }
            }
            Instruction::CallParent(op) => {
                let Value::Ident(function_name) = &op.function else {
                    unreachable!();
                };

                let class_name = translator.get_ty_name(&Value::Ident("self".to_owned()));
                let class = translator.data.classes.get(class_name.as_str()).unwrap();
                let parent = class.parent.as_ref().unwrap();
                let function_name = Lowercase::new(function_name.as_str());

                let func = data
                    .get_function(parent.as_str(), function_name.as_str())
                    .unwrap();

                let mut args = op
                    .args
                    .iter()
                    .map(|v| translator.load(v))
                    .collect::<Vec<_>>();

                // In our ABI the first argument is the object pointer.
                args.insert(0, translator.self_ptr.unwrap());

                args.insert(0, ctx_ptr);

                let stage = &props.stages[next_stage];
                next_stage += 1;

                if let Some(res) = perform_checked_call(
                    &mut translator,
                    function,
                    func,
                    &args,
                    &mut current_block,
                    stage,
                    false,
                ) {
                    translator.store(&op.return_assignment, res);
                }
            }
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(class), Value::Ident(func)) => (class, func),
                    _ => unreachable!(),
                };

                let class_name = Lowercase::new(class_name.as_str());
                let function_name = Lowercase::new(function_name.as_str());

                let mut args = op
                    .args
                    .iter()
                    .map(|v| translator.load(v))
                    .collect::<Vec<_>>();

                // For some functions we can provide better directly translated
                // implementations. This avoid the need to go through guest->host->guest
                // shim and provides the optimizer more freedom.
                let returns =
                    match call_builtin(&mut translator, &class_name, &function_name, &args) {
                        Some(res) => res,
                        None => {
                            let func = data.get_function(&class_name, &function_name).unwrap();

                            args.insert(0, ctx_ptr);

                            let stage = &props.stages[next_stage];
                            next_stage += 1;

                            let res = if let Some(res) = perform_checked_call(
                                &mut translator,
                                function,
                                func,
                                &args,
                                &mut current_block,
                                stage,
                                false,
                            ) {
                                translator.store(&op.return_assignment, res);
                                vec![res]
                            } else {
                                vec![]
                            };

                            res
                        }
                    };

                if let Some(res) = returns.get(0).copied() {
                    translator.store(&op.return_assignment, res);
                }
            }
            Instruction::PropertyGet(op) => {
                let Value::Ident(prop_name) = &op.property else {
                    unreachable!();
                };

                let class_name = translator.get_ty_name(&op.value);
                let prop_name = Lowercase::new(prop_name.as_str());

                let src = translator.load(&op.value);

                // If a getter exists we must call it, otherwise we can load directly
                // from the variable.
                let res = if let Some(id) = translator
                    .data
                    .get_property_getter_id(&class_name, &prop_name)
                {
                    let func = &data.functions[id.0];

                    let stage = &props.stages[next_stage];
                    next_stage += 1;

                    perform_checked_call(
                        &mut translator,
                        function,
                        func,
                        &[ctx_ptr, src],
                        &mut current_block,
                        stage,
                        false,
                    )
                    .unwrap()
                } else {
                    let class_name =
                        get_parent_class_with_property(&data.classes, &class_name, &prop_name)
                            .unwrap();

                    let class = data.get_class(&class_name).unwrap();

                    let prop = class.properties.get(&prop_name).unwrap();
                    let offset = *class
                        .field_offsets
                        .get(prop.variable.as_ref().unwrap())
                        .unwrap();

                    let ty = translator
                        .data
                        .resolve_ty_jit(&translator.get_ty_name(&op.return_assignment));

                    translator.builder.load(ty, src, offset as u32)
                };

                translator.store(&op.return_assignment, res);
            }
            Instruction::PropertySet(op) => {
                let Value::Ident(prop_name) = &op.property else {
                    unreachable!();
                };

                let class_name = translator.get_ty_name(&op.object);
                let prop_name = Lowercase::new(prop_name.as_str());

                let object = translator.load(&op.object);
                let value = translator.load(&op.value);

                // If a setter exists we must call it, otherwise we can store directly
                // to the variable.
                if let Some(id) = translator
                    .data
                    .get_property_setter_id(&class_name, &prop_name)
                {
                    let func = &data.functions[id.0];

                    let stage = &props.stages[next_stage];
                    next_stage += 1;

                    perform_checked_call(
                        &mut translator,
                        function,
                        func,
                        &[ctx_ptr, object, value],
                        &mut current_block,
                        stage,
                        false,
                    );
                } else {
                    let class_name =
                        get_parent_class_with_property(&data.classes, &class_name, &prop_name)
                            .unwrap();

                    let class = data.get_class(&class_name).unwrap();

                    let prop = class.properties.get(&prop_name).unwrap();
                    let offset = *class
                        .field_offsets
                        .get(prop.variable.as_ref().unwrap())
                        .unwrap();

                    translator.builder.store(value, object, offset as u32);
                }
            }
            Instruction::Return(value) => {
                translator.span.exit(data, &mut translator.builder, ctx_ptr);
                pop_stack_frame(&mut translator);

                if !function.return_ty.is_empty()
                    && !function.return_ty.eq_ignore_ascii_case("none")
                {
                    let rval = translator.load(value);
                    translator.builder.return_(&[rval, result_ok]);
                } else {
                    translator.builder.return_(&[result_ok]);
                }
            }
            Instruction::Cast(op) => {
                let res = perform_cast(&mut translator, &op.src, &op.dst, &mut current_block);
                translator.store(&op.dst, res);
            }
            Instruction::Is(op) => {
                let Value::Ident(class) = &op.class else {
                    unreachable!();
                };
                let class = Lowercase::new(class.as_str());

                let res =
                    perform_typecheck(&mut translator, &op.object, &class, &mut current_block);
                let res = translator.builder.uextend(res, JitType::I32);
                translator.store(&op.return_assignment, res);
            }
            Instruction::StructCreate(op) => {
                let ty = translator.get_type(&op.return_assignment);
                let Type::Struct(name) = &ty else {
                    unreachable!();
                };
                let st = translator.data.structs.get(name).unwrap();
                let type_id = st.ty;

                let one = translator.builder.iconst(JitType::Pointer, 1);
                let ptr = call_malloc(&mut translator, type_id, one);

                translator.store(&op.return_assignment, ptr);
            }
            Instruction::StructGet(op) => {
                let offset =
                    struct_member_offset(&mut translator, &op.struct_value, &op.member_name);

                let src = translator.load(&op.struct_value);
                let ty = translator.get_ty_name(&op.return_assignment);
                let ty = translator.data.resolve_ty_jit(&ty);

                translator.builder.load(ty, src, offset);
            }
            Instruction::StructSet(op) => {
                let offset =
                    struct_member_offset(&mut translator, &op.struct_value, &op.member_name);

                let st = translator.load(&op.struct_value);
                let value = translator.load(&op.value);

                translator.builder.store(value, st, offset);
            }
            Instruction::ArrayLength(op) => {
                let src = translator.load(&op.src);

                // If array == null the length is considered 0.
                let end_block = translator.builder.create_block(&[JitType::Pointer]);
                let fetch_len_block = translator.builder.create_block(&[]);

                let zero = translator.builder.iconst(JitType::Pointer, 0);
                translator
                    .builder
                    .brif(src, fetch_len_block, &[], end_block, &[zero]);

                {
                    translator.builder.switch_to_block(fetch_len_block);
                    let array_len = translator.builder.load(
                        JitType::Pointer,
                        src,
                        offset_of!(ArrayObject, len) as u32,
                    );
                    translator.builder.jump(end_block, &[array_len]);
                }

                translator.builder.switch_to_block(end_block);
                let array_len = translator.builder.block_params(end_block)[0];
                current_block = end_block;

                // The array length is usize, but integers in payrus only i32.
                let res = translator.builder.ireduce(array_len, JitType::I32);
                translator.store(&op.dst, res);
            }
            Instruction::ArrayGet(op) => {
                let array = translator.load(&op.array);
                let index = translator.load(&op.index);

                let count = translator.builder.iconst(JitType::I32, 1);
                emit_array_bounds_check(&mut translator, &mut current_block, array, index, count);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );

                let elem_ty = translator
                    .data
                    .resolve_ty_jit(&translator.get_ty_name(&op.return_assignment));
                let stride = elem_ty.size();

                let index_wide = translator.builder.uextend(index, JitType::Pointer);
                let offset = translator.builder.imul_imm(index_wide, stride as i64);
                let addr = translator.builder.iadd(array_ptr, offset);

                let value = translator.builder.load(elem_ty, addr, 0);
                translator.store(&op.return_assignment, value);
            }
            Instruction::ArraySet(op) => {
                let index = translator.load(&op.index);
                let value = translator.load(&op.value);
                let array = translator.load(&op.array);

                let count = translator.builder.iconst(JitType::I32, 1);
                emit_array_bounds_check(&mut translator, &mut current_block, array, index, count);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );

                let elem_ty = translator
                    .data
                    .resolve_ty_jit(&translator.get_ty_name(&op.value));
                let stride = elem_ty.size();

                let index_wide = translator.builder.uextend(index, JitType::Pointer);
                let offset = translator.builder.imul_imm(index_wide, stride as i64);
                let addr = translator.builder.iadd(array_ptr, offset);

                translator.builder.store(value, addr, 0);
            }
            Instruction::ArrayCreate(op) => {
                let Type::Array(elem_ty) = translator.get_type(&op.dst) else {
                    unreachable!();
                };

                let type_id = translator.get_type_id(&elem_ty);

                let array_len = translator.load(&op.src);
                let array_len = translator.builder.uextend(array_len, JitType::Pointer);
                let array_ptr = call_malloc(&mut translator, type_id, array_len);

                let one_usize = translator.builder.iconst(JitType::Pointer, 1);
                let array_object_ptr =
                    call_malloc(&mut translator, data.resources.primitives.array, one_usize);

                translator.builder.store(
                    array_ptr,
                    array_object_ptr,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                translator.builder.store(
                    array_len,
                    array_object_ptr,
                    offset_of!(ArrayObject, len) as u32,
                );

                translator.store(&op.dst, array_object_ptr);
            }
            Instruction::ArrayAdd(op) => {
                let Type::Array(elem_ty) = translator.get_type(&op.array) else {
                    unreachable!();
                };

                let type_id = translator.get_type_id(&elem_ty);
                let ty = data.resources.types.get(type_id).unwrap();

                let array = translator.load(&op.array);
                let value = translator.load(&op.value);
                let count = translator.load(&op.count);

                let continue_block = translator.builder.create_block(&[]);
                let abort_block = translator.builder.create_block(&[]);

                translator
                    .builder
                    .brif(array, continue_block, &[], abort_block, &[]);

                translator.builder.switch_to_block(abort_block);
                abort_function(&mut translator);

                translator.builder.switch_to_block(continue_block);

                // Clamp count at 0, we cannot add a negative amount of elems.
                let zero_i32 = translator.builder.iconst(JitType::I32, 0);
                let count_greater_zero = translator.builder.icmp(
                    count,
                    zero_i32,
                    IntCompareOp::SignedGreaterThanOrEqual,
                );
                let count_greater_zero =
                    translator.builder.uextend(count_greater_zero, JitType::I32);
                //  count = count * (count >= 0)
                let count = translator.builder.imul(count, count_greater_zero);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                let array_len = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, len) as u32,
                );

                let count_wide = translator.builder.uextend(count, JitType::Pointer);
                let new_len = translator.builder.iadd(array_len, count_wide);

                let stride = ty.size();
                let new_array_ptr = call_malloc(&mut translator, type_id, new_len);

                // Update array ptr and len.
                translator
                    .builder
                    .store(new_array_ptr, array, offset_of!(ArrayObject, ptr) as u32);
                translator
                    .builder
                    .store(new_len, array, offset_of!(ArrayObject, len) as u32);

                // Copy existing region
                let copy_leading_bytes = translator.builder.imul_imm(array_len, stride as i64);
                translator
                    .builder
                    .memcpy(array_ptr, new_array_ptr, copy_leading_bytes);

                // Append value count times at the end.

                let loop_block = translator.builder.create_block(&[JitType::Pointer]);
                let loop_body_block = translator.builder.create_block(&[JitType::Pointer]);
                let final_block = translator.builder.create_block(&[]);

                let start_index = array_len;
                let end_index = translator.builder.iadd(array_len, count_wide);

                translator.builder.jump(loop_block, &[start_index]);

                // Loop block
                {
                    translator.builder.switch_to_block(loop_block);

                    // Continue if count > 0
                    let index = translator.builder.block_params(loop_block)[0];
                    let should_continue =
                        translator
                            .builder
                            .icmp(index, end_index, IntCompareOp::UnsignedLessThan);

                    translator.builder.brif(
                        should_continue,
                        loop_body_block,
                        &[index],
                        final_block,
                        &[],
                    );
                }

                // Loop body block
                {
                    translator.builder.switch_to_block(loop_body_block);

                    let index = translator.builder.block_params(loop_body_block)[0];
                    let offset = translator.builder.imul_imm(index, stride as i64);
                    let addr = translator.builder.iadd(new_array_ptr, offset);

                    translator.builder.store(value, addr, 0);

                    let next_index = translator.builder.iadd_imm(index, 1);
                    translator.builder.jump(loop_block, &[next_index]);
                }

                translator.builder.switch_to_block(final_block);
                current_block = final_block;
            }
            Instruction::ArrayInsert(op) => {
                let Type::Array(elem_ty) = translator.get_type(&op.array) else {
                    unreachable!();
                };

                let type_id = translator.get_type_id(&elem_ty);
                let ty = data.resources.types.get(type_id).unwrap();

                let array = translator.load(&op.array);
                let value = translator.load(&op.value);
                let index = translator.load(&op.index);

                // We allow inserting at the last position of the array, e.g.
                // we can insert into an empty array at index 0.
                let zero = translator.builder.iconst(JitType::I32, 0);
                emit_array_bounds_check(&mut translator, &mut current_block, array, index, zero);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                let array_len = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, len) as u32,
                );

                let new_len = translator.builder.iadd_imm(array_len, 1);
                let index_wide = translator.builder.uextend(index, JitType::Pointer);

                // Allocate new array.
                let stride = ty.size();
                let new_array_ptr = call_malloc(&mut translator, type_id, new_len);

                // Update array ptr and len.
                translator
                    .builder
                    .store(new_array_ptr, array, offset_of!(ArrayObject, ptr) as u32);
                translator
                    .builder
                    .store(new_len, array, offset_of!(ArrayObject, len) as u32);

                // Copy left region (all elements 0..index).
                let copy_leading_elems = index_wide;
                let copy_leading_bytes = translator
                    .builder
                    .imul_imm(copy_leading_elems, stride as i64);

                translator
                    .builder
                    .memcpy(array_ptr, new_array_ptr, copy_leading_bytes);

                // Copy right region (all elements index..end to index+1..end).
                let copy_trailing_elems = translator.builder.isub(array_len, index_wide);
                let copy_trailing_bytes = translator
                    .builder
                    .imul_imm(copy_trailing_elems, stride as i64);
                let trailing_src_offset = translator.builder.imul_imm(index_wide, stride as i64);
                let trailing_dst_offset = translator
                    .builder
                    .iadd_imm(trailing_src_offset, stride as i64);
                let trailing_src_ptr = translator.builder.iadd(array_ptr, trailing_src_offset);
                let trailing_dst_ptr = translator.builder.iadd(new_array_ptr, trailing_dst_offset);

                translator
                    .builder
                    .memcpy(trailing_src_ptr, trailing_dst_ptr, copy_trailing_bytes);

                // Insert new element at index.
                let offset = translator.builder.imul_imm(index_wide, stride as i64);
                let addr = translator.builder.iadd(new_array_ptr, offset);
                translator.builder.store(value, addr, 0);
            }
            Instruction::ArrayRemove(op) => {
                let Type::Array(elem_ty) = translator.get_type(&op.array_value) else {
                    unreachable!();
                };

                let type_id = translator.get_type_id(&elem_ty);
                let ty = data.resources.types.get(type_id).unwrap();

                let array = translator.load(&op.array_value);
                let index = translator.load(&op.index);
                let count = translator.load(&op.count);
                emit_array_bounds_check(&mut translator, &mut current_block, array, index, count);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                let array_len = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, len) as u32,
                );
                let index_wide = translator.builder.uextend(index, JitType::Pointer);
                let count_wide = translator.builder.uextend(count, JitType::Pointer);

                // This cannot overflow after our bounds check.
                let new_len = translator.builder.isub(array_len, count_wide);

                // Allocate new array.
                let stride = ty.size();
                let new_array_ptr = call_malloc(&mut translator, type_id, new_len);

                // Copy regions from old array.
                // Copy 0..index.
                let copy_leading_bytes = translator.builder.imul_imm(index_wide, stride as i64);
                translator
                    .builder
                    .memcpy(array_ptr, new_array_ptr, copy_leading_bytes);
                // Copy index+count..len.
                let index_plus_count = translator.builder.iadd(index_wide, count_wide);
                let copy_trailing_elems = translator.builder.isub(array_len, index_plus_count);
                let copy_trailing_bytes = translator
                    .builder
                    .imul_imm(copy_trailing_elems, stride as i64);
                let array_ptr_plus_index_count =
                    translator.builder.iadd(array_ptr, index_plus_count);
                let new_array_ptr_plus_index = translator.builder.iadd(array_ptr, index_wide);
                translator.builder.memcpy(
                    array_ptr_plus_index_count,
                    new_array_ptr_plus_index,
                    copy_trailing_bytes,
                );

                translator
                    .builder
                    .store(new_array_ptr, array, offset_of!(ArrayObject, ptr) as u32);
            }
            Instruction::ArrayClear(op) => {
                let array = translator.load(&op.array);

                let continue_block = translator.builder.create_block(&[]);
                let abort_block = translator.builder.create_block(&[]);

                translator
                    .builder
                    .brif(array, continue_block, &[], abort_block, &[]);

                translator.builder.switch_to_block(abort_block);
                abort_function(&mut translator);

                translator.builder.switch_to_block(continue_block);
                current_block = continue_block;

                let zero = translator.builder.iconst(JitType::Pointer, 0);
                translator
                    .builder
                    .store(zero, array, offset_of!(ArrayObject, ptr) as u32);
                translator
                    .builder
                    .store(zero, array, offset_of!(ArrayObject, len) as u32);
            }
            Instruction::ArrayFind(op) | Instruction::ArrayRFind(op) => {
                let scan_backwards = matches!(instruction, Instruction::ArrayRFind(_));

                let Type::Array(elem_ty) = translator.get_type(&op.array_value) else {
                    unreachable!();
                };

                let array = translator.load(&op.array_value);
                let start_index = translator.load(&op.start_index);
                let searched_value = translator.load(&op.value);

                let continue_block = translator.builder.create_block(&[]);
                let abort_block = translator.builder.create_block(&[]);

                translator
                    .builder
                    .brif(array, continue_block, &[], abort_block, &[]);

                translator.builder.switch_to_block(abort_block);
                abort_function(&mut translator);

                translator.builder.switch_to_block(continue_block);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                let array_len = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, len) as u32,
                );

                let loop_block = translator.builder.create_block(&[JitType::Pointer]);
                let mut loop_body_block = translator.builder.create_block(&[JitType::Pointer]);
                let final_block = translator.builder.create_block(&[JitType::Pointer]);

                let mut start_index_wide =
                    translator.builder.uextend(start_index, JitType::Pointer);

                // If using RFind a start index of -1 indicates the end of the array.
                if scan_backwards {
                    // start_index = max(start_index, (start_index == -1) * (array_len - 1))
                    let negative_one = translator.builder.iconst(JitType::I32, -1);
                    let start_at_end =
                        translator
                            .builder
                            .icmp(start_index, negative_one, IntCompareOp::Equal);

                    let start_at_end_wide =
                        translator.builder.uextend(start_at_end, JitType::Pointer);
                    let array_len_minus_one = translator.builder.isub_imm(array_len, 1);
                    let array_len_mul_start_at_end = translator
                        .builder
                        .imul(start_at_end_wide, array_len_minus_one);
                    // If the array is empty, array_len - 1 will become usize::MAX and
                    // the index check will fail correctly.
                    start_index_wide = translator
                        .builder
                        .umax(start_at_end_wide, array_len_mul_start_at_end);
                }

                translator.builder.jump(loop_block, &[start_index_wide]);

                // Loop block
                {
                    translator.builder.switch_to_block(loop_block);
                    let index = translator.builder.block_params(loop_block)[0];

                    let negative_one = translator.builder.iconst(JitType::Pointer, -1);

                    let zero_isize = translator.builder.iconst(JitType::Pointer, 0);
                    // index < 0
                    let lower_oob =
                        translator
                            .builder
                            .icmp(index, zero_isize, IntCompareOp::SignedLessThan);
                    // index >= array_len
                    let upper_oob = translator.builder.icmp(
                        index,
                        array_len,
                        IntCompareOp::UnsignedGreaterThanOrEqual,
                    );
                    let terminate_loop = translator.builder.bor(lower_oob, upper_oob);

                    translator.builder.brif(
                        terminate_loop,
                        final_block,
                        &[negative_one],
                        loop_body_block,
                        &[index],
                    );
                }

                // Loop body block
                {
                    translator.builder.switch_to_block(loop_body_block);
                    let index = translator.builder.block_params(loop_body_block)[0];

                    let ty = match *elem_ty {
                        Type::None | Type::Bool | Type::Int => JitType::I32,
                        Type::Float => JitType::F32,
                        Type::Class(_)
                        | Type::Array(_)
                        | Type::Struct(_)
                        | Type::String
                        | Type::Var => JitType::Pointer,
                    };

                    let offset = translator.builder.imul_imm(index, ty.size() as i64);
                    let addr = translator.builder.iadd(array_ptr, offset);
                    let value = translator.builder.load(ty, addr, 0);

                    let is_eq = check_equality(
                        &mut translator,
                        &elem_ty,
                        value,
                        searched_value,
                        &mut loop_body_block,
                    );

                    let next_index = if scan_backwards {
                        translator.builder.isub_imm(index, 1)
                    } else {
                        translator.builder.iadd_imm(index, 1)
                    };

                    translator.builder.brif(
                        is_eq,
                        final_block,
                        &[index],
                        loop_block,
                        &[next_index],
                    );
                }

                // Final block
                {
                    translator.builder.switch_to_block(final_block);
                    let index = translator.builder.block_params(final_block)[0];
                    let truncated_index = translator.builder.ireduce(index, JitType::I32);

                    translator.store(&op.return_index, truncated_index);
                    current_block = final_block;
                }
            }
            Instruction::ArrayFindStruct(op) => {
                let Type::Array(elem_ty) = translator.get_type(&op.array_value) else {
                    unreachable!();
                };
                let Value::String(member_name) = &op.member_name else {
                    unreachable!();
                };

                let array = translator.load(&op.array_value);
                let start_index = translator.load(&op.start_index);
                let seached_value = translator.load(&op.value);

                let continue_block = translator.builder.create_block(&[]);
                let abort_block = translator.builder.create_block(&[]);

                translator
                    .builder
                    .brif(array, continue_block, &[], abort_block, &[]);

                translator.builder.switch_to_block(abort_block);
                abort_function(&mut translator);

                translator.builder.switch_to_block(continue_block);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                let array_len = translator.builder.load(
                    JitType::Pointer,
                    array,
                    offset_of!(ArrayObject, len) as u32,
                );

                let loop_block = translator.builder.create_block(&[JitType::Pointer]);
                let mut loop_body_block = translator.builder.create_block(&[JitType::Pointer]);
                let final_block = translator.builder.create_block(&[JitType::Pointer]);

                let start_index_wide = translator.builder.uextend(start_index, JitType::Pointer);
                translator.builder.jump(loop_block, &[start_index_wide]);

                // Loop block
                {
                    translator.builder.switch_to_block(loop_block);
                    let index = translator.builder.block_params(loop_block)[0];

                    let negative_one = translator.builder.iconst(JitType::Pointer, -1);
                    let terminate_loop = translator.builder.icmp(
                        index,
                        array_len,
                        IntCompareOp::UnsignedGreaterThanOrEqual,
                    );

                    translator.builder.brif(
                        terminate_loop,
                        final_block,
                        &[negative_one],
                        loop_body_block,
                        &[index],
                    );
                }

                // Loop body block
                {
                    translator.builder.switch_to_block(loop_body_block);
                    let index = translator.builder.block_params(loop_body_block)[0];

                    let (field_offset, field_ty) = match &*elem_ty {
                        Type::Struct(name) => {
                            let struct_def = translator.data.structs.get(name).unwrap();

                            let member_name = Lowercase::new(&**member_name);
                            let offset =
                                *struct_def.field_offsets.get(&member_name).unwrap() as u32;

                            let member = struct_def
                                .s
                                .members
                                .iter()
                                .find(|v| Lowercase::new(&*v.name) == member_name)
                                .unwrap();

                            let ty = translator
                                .data
                                .resolve_ty(&Lowercase::new(member.ty.as_str()));
                            (offset, ty)
                        }
                        _ => unreachable!(),
                    };

                    let ty = match field_ty {
                        Type::None | Type::Bool | Type::Int => JitType::I32,
                        Type::Float => JitType::F32,
                        Type::Class(_)
                        | Type::Array(_)
                        | Type::Struct(_)
                        | Type::String
                        | Type::Var => JitType::Pointer,
                    };

                    let offset = translator
                        .builder
                        .imul_imm(index, JitType::Pointer.size() as i64);
                    let addr = translator.builder.iadd(array_ptr, offset);
                    let struct_ptr = translator.builder.load(JitType::Pointer, addr, 0);
                    let value = translator.builder.load(ty, struct_ptr, field_offset as u32);

                    let is_eq = check_equality(
                        &mut translator,
                        &field_ty,
                        value,
                        seached_value,
                        &mut loop_body_block,
                    );

                    let next_index = translator.builder.iadd_imm(index, 1);
                    translator.builder.brif(
                        is_eq,
                        final_block,
                        &[index],
                        loop_block,
                        &[next_index],
                    );
                }

                // Final block
                {
                    translator.builder.switch_to_block(final_block);
                    let index = translator.builder.block_params(final_block)[0];
                    let truncated_index = translator.builder.ireduce(index, JitType::I32);

                    translator.store(&op.return_index, truncated_index);
                    current_block = final_block;
                }
            }
            Instruction::StringConcat(op) => {
                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);

                let res = string::string_concat(&mut translator, lhs, rhs);
                translator.store(&op.dst, res);
            }
            op => todo!("{:?}", op),
        }
    }

    // Sometimes the final return instruction is missing and we must insert it.
    // This is only possible for functions that return "none" and the fact we
    // return "none" in this case is checked by the validation pass.
    // Other functions that don't return also can also sometimes refer to this
    // section due to weird pex codegen, however in that case this section
    // is statically unreachable.
    let return_ty =
        if function.return_ty.is_empty() || function.return_ty.eq_ignore_ascii_case("none") {
            None
        } else {
            let ty = Lowercase::new(&*function.return_ty);
            Some(data.resolve_ty_jit(&ty))
        };

    if !translator.builder.is_current_block_full() {
        translator.span.exit(data, &mut translator.builder, ctx_ptr);
        pop_stack_frame(&mut translator);

        if let Some(return_ty) = return_ty {
            let rval = translator.builder.uninit(return_ty);
            translator.builder.return_(&[rval, result_ok]);
        } else {
            translator.builder.return_(&[result_ok]);
        }
    }

    if let Some(block) = blocks.get(&function.instructions.len()) {
        if !translator.builder.is_block_full(*block) {
            translator.builder.switch_to_block(*block);

            translator.span.exit(data, &mut translator.builder, ctx_ptr);
            pop_stack_frame(&mut translator);

            if let Some(return_ty) = return_ty {
                let rval = translator.builder.uninit(return_ty);
                translator.builder.return_(&[rval, result_ok]);
            } else {
                translator.builder.return_(&[result_ok]);
            }
        }
    }

    translator.builder.finalize();
}

pub struct FunctionTranslator<'a> {
    function: &'a Function,
    data: &'a CodegenData,
    pub builder: FunctionBuilder<'a>,
    locals: HashMap<Lowercase<String>, Local>,
    fields: HashMap<&'a Lowercase<str>, Field>,
    self_ptr: Option<JitValue>,
    self_ty: &'a Lowercase<str>,
    span: &'a mut Span,
    ctx_ptr: JitValue,
    stack_frame: &'a StackFrame,
}

impl<'a> FunctionTranslator<'a> {
    pub fn load(&mut self, value: &Value) -> JitValue {
        match value {
            Value::Null => self.builder.iconst(JitType::I64, 0),
            Value::Bool(value) => self.builder.iconst(JitType::I32, u8::from(*value)),
            Value::Int(value) => self.builder.iconst(JitType::I32, *value),
            Value::Float(value) => self.builder.fconst(JitType::F32, *value),
            Value::String(value) => string::string_create(self, &value),
            Value::Ident(name) => self.load_var(&Lowercase::new(name.as_str())),
        }
    }

    pub fn load_var(&mut self, name: &Lowercase<str>) -> JitValue {
        match name.as_inner() {
            "self" => return self.self_ptr.unwrap(),
            _ => {}
        }

        if let Some(local) = self.locals.get(name) {
            return self.builder.get_var(local.variable);
        }

        let mut class = self.data.get_class(&self.self_ty).unwrap();
        loop {
            if let Some(field_ty) = class.fields.get(name) {
                let ty = self.data.resolve_ty_jit(&field_ty);

                let offset = *class.field_offsets.get(name).unwrap();
                return self.builder.load(ty, self.self_ptr.unwrap(), offset as u32);
            }

            if let Some(parent) = &class.parent {
                class = self.data.get_class(&parent).unwrap();
            } else {
                break;
            }
        }

        panic!("undefined variable {}", name);
    }

    pub fn store(&mut self, slot: &Value, value: JitValue) {
        let Value::Ident(name) = slot else {
            return;
        };

        self.store_var(&Lowercase::new(name.as_str()), value);
    }

    pub fn store_var(&mut self, name: &Lowercase<str>, value: JitValue) {
        match name.as_inner() {
            // ::nonevar is a magic variable that always holds none.
            // Writing to it discards the value.
            "::nonevar" => return,
            _ => {}
        }

        if let Some(local) = self.locals.get(name) {
            self.builder.set_var(local.variable, value);
            return;
        }

        let mut class = self.data.get_class(&self.self_ty).unwrap();
        loop {
            if let Some(_) = class.fields.get(name) {
                let offset = *class.field_offsets.get(name).unwrap();
                self.builder
                    .store(value, self.self_ptr.unwrap(), offset as u32);
                return;
            }

            if let Some(parent) = &class.parent {
                class = self.data.get_class(&parent).unwrap();
            } else {
                break;
            }
        }

        panic!("undefiend variable {}", name);
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
struct Field {
    ty: JitType,
    ty_name: Lowercase<String>,
}

#[derive(Clone, Debug)]
struct Local {
    ty: JitType,
    ty_name: Lowercase<String>,
    variable: JitVariable,
}

/// Performs a runtime cast of `src` to `dst`.
fn perform_cast(
    translator: &mut FunctionTranslator<'_>,
    src_val: &Value,
    dst_val: &Value,
    block: &mut Block,
) -> JitValue {
    let src_ty = translator.get_type(&src_val);
    let dst_ty = translator.get_type(&dst_val);
    let src = translator.load(&src_val);

    // Identity casts are allowed.
    if src_ty == dst_ty {
        return src;
    }

    match (src_ty, dst_ty) {
        // Cast each element individually.
        (Type::Array(src_elem_ty), Type::Array(dst_elem_ty)) => {
            let src_array_ptr =
                translator
                    .builder
                    .load(JitType::Pointer, src, offset_of!(ArrayObject, ptr) as u32);
            let src_array_len =
                translator
                    .builder
                    .load(JitType::Pointer, src, offset_of!(ArrayObject, len) as u32);

            let type_id = translator.get_type_id(&dst_elem_ty);
            let dst_array_ptr = call_malloc(translator, type_id, src_array_len);

            let one = translator.builder.iconst(JitType::Pointer, 1);
            let new_array_object_ptr =
                call_malloc(translator, translator.data.resources.primitives.array, one);

            translator.builder.store(
                dst_array_ptr,
                new_array_object_ptr,
                offset_of!(ArrayObject, ptr) as u32,
            );
            translator.builder.store(
                src_array_len,
                new_array_object_ptr,
                offset_of!(ArrayObject, len) as u32,
            );

            let src_ty = src_elem_ty.to_jit_type();
            let src_stride = src_ty.size();

            let dst_ty = dst_elem_ty.to_jit_type();
            let dst_stride = dst_ty.size();

            let loop_block = translator.builder.create_block(&[]);
            let mut loop_body_block = translator.builder.create_block(&[]);
            let final_block = translator.builder.create_block(&[]);

            let start_index = translator.builder.iconst(JitType::Pointer, 0);
            let index_var = translator.builder.def_var(JitType::Pointer, start_index);

            translator.builder.jump(loop_block, &[]);

            // Loop block
            {
                translator.builder.switch_to_block(loop_block);

                // Continue if index < src_array_len
                let index = translator.builder.get_var(index_var);
                let should_continue =
                    translator
                        .builder
                        .icmp(index, src_array_len, IntCompareOp::UnsignedLessThan);

                translator
                    .builder
                    .brif(should_continue, loop_body_block, &[], final_block, &[]);
            }

            // Loop body block
            {
                translator.builder.switch_to_block(loop_body_block);

                let index = translator.builder.get_var(index_var);
                let src_offset = translator.builder.imul_imm(index, src_stride as i64);
                let src_addr = translator.builder.iadd(src_array_ptr, src_offset);

                let elem = translator.builder.load(src_ty, src_addr, 0);
                let dst = perform_cast_inner(
                    translator,
                    &src_elem_ty,
                    &dst_elem_ty,
                    elem,
                    &mut loop_body_block,
                );

                let dst_offset = translator.builder.imul_imm(index, dst_stride as i64);
                let dst_addr = translator.builder.iadd(dst_array_ptr, dst_offset);

                translator.builder.store(dst, dst_addr, 0);

                let next_index = translator.builder.iadd_imm(index, 1);
                translator.builder.set_var(index_var, next_index);

                translator.builder.jump(loop_block, &[]);
            }

            translator.builder.switch_to_block(final_block);
            *block = final_block;

            new_array_object_ptr
        }
        (src_ty, dst_ty) => perform_cast_inner(translator, &src_ty, &dst_ty, src, block),
    }
}

fn perform_cast_inner(
    translator: &mut FunctionTranslator<'_>,
    src_ty: &Type,
    dst_ty: &Type,
    src: JitValue,
    block: &mut Block,
) -> JitValue {
    match (src_ty, dst_ty) {
        // Integer <-> float conversions
        (Type::Int, Type::Float) => translator.builder.convert_sint_to_float(src, JitType::F32),
        (Type::Float, Type::Int) => translator.builder.convert_float_to_sint(src, JitType::I32),
        // Bool conversions
        (Type::Int, Type::Bool) => src,
        // Under IEEE 754 floats with value 0.0 have the exact bit pattern of 0.
        (Type::Float, Type::Bool) => translator.builder.bitcast(src, JitType::I32),
        (Type::String | Type::Class(_) | Type::Struct(_) | Type::Array(_), Type::Bool) => {
            // We need to reduce from isize to i32.
            // We do a popcnt before ireduce so that we get a non-zero value, even
            // if all LSB are zero.
            let popcnt = translator.builder.popcnt(src);
            translator.builder.ireduce(popcnt, JitType::I32)
        }
        // Bools and ints 0..1 have the same integer representation.
        (Type::Bool, Type::Int) => src,
        (Type::Bool, Type::Float) => translator.builder.convert_uint_to_float(src, JitType::F32),
        (Type::Class(src_class), Type::Class(dst_class)) => {
            // Try upcasting.
            let mut class_name = src_class;
            loop {
                if class_name == dst_class {
                    return src;
                }

                let class = translator.data.classes.get(class_name).unwrap();
                if let Some(parent) = &class.parent {
                    class_name = parent;
                } else {
                    break;
                }
            }

            // Try downcasting.
            let mut class_name = dst_class;
            loop {
                if class_name == src_class {
                    let is_ok = perform_class_typecheck(translator, src, &class_name, block);

                    let continue_block = translator.builder.create_block(&[]);
                    let abort_block = translator.builder.create_block(&[]);

                    translator
                        .builder
                        .brif(is_ok, continue_block, &[], abort_block, &[]);

                    translator.builder.switch_to_block(abort_block);
                    abort_function(translator);

                    translator.builder.switch_to_block(continue_block);
                    return src;
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
        (Type::Var, Type::Bool) => perform_var_downcast(
            translator,
            src,
            ObjectTag::TypeId(translator.data.resources.primitives.bool),
            block,
        ),
        (Type::Var, Type::Int) => perform_var_downcast(
            translator,
            src,
            ObjectTag::TypeId(translator.data.resources.primitives.int),
            block,
        ),
        (Type::Var, Type::Float) => perform_var_downcast(
            translator,
            src,
            ObjectTag::TypeId(translator.data.resources.primitives.float),
            block,
        ),
        (Type::Var, Type::String) => perform_var_downcast(
            translator,
            src,
            ObjectTag::TypeId(translator.data.resources.primitives.string),
            block,
        ),
        (Type::Var, Type::Class(class_name)) => {
            // Perform var to class cast, then attempt to downcast the class value.
            let res = perform_var_downcast(translator, src, ObjectTag::Class, block);
            let is_ok = perform_class_typecheck(translator, res, &class_name, block);

            let continue_block = translator.builder.create_block(&[]);
            let abort_block = translator.builder.create_block(&[]);

            translator
                .builder
                .brif(is_ok, continue_block, &[], abort_block, &[]);

            translator.builder.switch_to_block(abort_block);
            abort_function(translator);

            translator.builder.switch_to_block(continue_block);
            *block = continue_block;

            res
        }
        (Type::Var, Type::Struct(name)) => {
            let id = translator.data.structs.get(name).unwrap().ty;
            perform_var_downcast(translator, src, ObjectTag::TypeId(id), block)
        }
        (src_ty, Type::Var) => {
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

            create_var(translator, src, tag)
        }
        // String formatting conversions
        (Type::Bool, Type::String) => {
            let ptr = translator.builder.call(
                translator.data.runtime_funcs.bool_to_string,
                &[translator.ctx_ptr, src],
            )[0];
            ptr
        }
        (Type::Int, Type::String) => {
            let ptr = translator.builder.call(
                translator.data.runtime_funcs.i32_to_string,
                &[translator.ctx_ptr, src],
            )[0];
            ptr
        }
        (Type::Float, Type::String) => {
            let ptr = translator.builder.call(
                translator.data.runtime_funcs.f32_to_string,
                &[translator.ctx_ptr, src],
            )[0];
            ptr
        }
        (Type::Class(_), Type::String) => {
            let ptr = translator.builder.call(
                translator.data.runtime_funcs.object_to_string,
                &[translator.ctx_ptr, src],
            )[0];
            ptr
        }
        (Type::Struct(_) | Type::Array(_), Type::String) => {
            // FIXME: Implement
            string::string_create(translator, "")
        }
        // A null pointer.
        (Type::None, Type::Class(_)) => translator.builder.iconst(JitType::Pointer, 0),
        (src_ty, dst_ty) => todo!("{:?} {:?}", src_ty, dst_ty),
    }
}

/// Performs a runtime cast from `var` to another type.
fn perform_var_downcast(
    translator: &mut FunctionTranslator<'_>,
    src: JitValue,
    dst: ObjectTag,
    block: &mut Block,
) -> JitValue {
    let tag = translator.builder.load(JitType::Pointer, src, 0);

    let continue_block = translator.builder.create_block(&[]);
    let abort_block = translator.builder.create_block(&[]);

    let wanted_tag = translator
        .builder
        .iconst(JitType::Pointer, dst.to_bits() as i64);
    let is_ok = translator
        .builder
        .icmp(tag, wanted_tag, IntCompareOp::Equal);
    translator
        .builder
        .brif(is_ok, continue_block, &[], abort_block, &[]);

    translator.builder.switch_to_block(abort_block);
    abort_function(translator);

    translator.builder.switch_to_block(continue_block);
    *block = continue_block;

    let ty = match dst {
        ObjectTag::Class => JitType::Pointer,
        ObjectTag::TypeId(id) => match translator.data.resources.types.get(id).unwrap() {
            resources::Type::Unit => JitType::I32,
            resources::Type::Int { bits } => {
                // FIXME: Relax
                assert_eq!(bits, 32);
                JitType::I32
            }
            resources::Type::Float { bits } => {
                // FIXME: Relax
                assert_eq!(bits, 32);
                JitType::F32
            }
            resources::Type::Composite(_) => {
                // FIXME: Currently all composite types are implemented as pointers
                // to the type, but the `TypeId` still refers to the actual composite
                // type. In the future we should add an extra `Pointer(TypeId)` type
                // that indicates a pointer type specifically, then we can remove this hack.
                JitType::Pointer
            }
        },
    };

    translator.builder.load(ty, src, size_of::<usize>() as u32)
}

/// Creates a new `Var`.
fn create_var(
    translator: &mut FunctionTranslator<'_>,
    value: JitValue,
    tag: ObjectTag,
) -> JitValue {
    // Allocate var object
    let one = translator.builder.iconst(JitType::Pointer, 1);
    let ptr = call_malloc(translator, translator.data.resources.primitives.var, one);

    // Store tag
    let tag = translator
        .builder
        .iconst(JitType::Pointer, tag.to_bits() as i64);
    translator
        .builder
        .store(tag, ptr, offset_of!(VarObject, tag) as u32);

    // Store data
    translator
        .builder
        .store(value, ptr, offset_of!(VarObject, data) as u32);

    ptr
}

/// Performs a runtime typecheck whether `src` is `dst_class` or a subtype of `dst_class`.
fn perform_typecheck(
    translator: &mut FunctionTranslator<'_>,
    src: &Value,
    dst_class: &Lowercase<str>,
    block: &mut Block,
) -> JitValue {
    let src_ty = translator.get_type(src);

    let Type::Class(_) = src_ty else {
        return translator.builder.iconst(JitType::I32, 0);
    };

    let src = translator.load(src);
    perform_class_typecheck(translator, src, dst_class, block)
}

fn perform_class_typecheck(
    translator: &mut FunctionTranslator<'_>,
    src: JitValue,
    dst_class: &Lowercase<str>,
    block: &mut Block,
) -> JitValue {
    let dst_id = translator.data.classes.get(dst_class).unwrap().type_id;
    let dst_id = translator
        .builder
        .iconst(JitType::I64, dst_id.as_bits() as i64);

    let type_info_ptr = translator.builder.load(JitType::Pointer, src, 0);

    // The first word of an object contains a constant pointer to the type
    // info. The type info starts with a single word containing the number
    // of type ids + vtable pairs that follow.
    // If any type id matches we have a valid object of that class.

    let count = translator.builder.load(JitType::Pointer, type_info_ptr, 0);

    let next_block = translator.builder.create_block(&[]);

    // Note that every object has at least one typeid. This means count is
    // always >= 1.
    let iter_block = translator.builder.create_block(&[JitType::I64]);

    let zero = translator.builder.iconst(JitType::Pointer, 0);
    translator.builder.jump(iter_block, &[zero]);

    translator.builder.switch_to_block(iter_block);
    let index = translator.builder.block_params(iter_block)[0];

    // Load and check current offset.
    let stride = size_of::<usize>() * 2;
    let offset = translator.builder.imul_imm(index, stride as i64);
    let offset = translator
        .builder
        .iadd_imm(offset, size_of::<usize>() as i64);

    let addr = translator.builder.iadd(type_info_ptr, offset);
    let type_id = translator.builder.load(JitType::Pointer, addr, 0);

    let is_eq = translator
        .builder
        .icmp(type_id, dst_id, IntCompareOp::Equal);

    // Increment index.
    let next_index = translator.builder.iadd_imm(offset, 1);
    let next_index_valid = translator
        .builder
        .icmp(next_index, count, IntCompareOp::NotEqual);

    // Equivalent to `next_offset_valid && !is_eq`.
    let is_neq = translator.builder.bnot(is_eq);
    let should_continue = translator.builder.band(next_index_valid, is_neq);

    translator
        .builder
        .brif(should_continue, iter_block, &[next_index], next_block, &[]);

    translator.builder.switch_to_block(next_block);
    *block = next_block;

    is_eq
}

fn perform_checked_call(
    translator: &mut FunctionTranslator<'_>,
    caller: &Function,
    callee: &FunctionDef,
    args: &[JitValue],
    block: &mut Block,
    stage: &Stage,
    indirect: bool,
) -> Option<JitValue> {
    // We need to be able to jump straight to the function call
    // to resume a suspended call.
    let call_block = translator.builder.create_block(&[]);
    translator.builder.jump(call_block, &[]);
    translator.builder.switch_to_block(call_block);

    let res = if indirect {
        let signature = translate_function_signature(translator.data, &callee.function);

        let fn_ptr = vtable::load_function_addr(translator, callee, args[1]);
        translator.builder.call_indirect(signature, fn_ptr, args)
    } else {
        let func_id = callee.func_id.unwrap();

        translator.builder.call(func_id, args)
    };

    // We need to remember the offset relative to the stack base for the function call.
    // The loaded stack len already includes our stack frame, but we want the offset
    // where we can start storing our stackframe.
    let stack_len_before_call = translator.builder.load(
        JitType::Pointer,
        translator.ctx_ptr,
        offset_of!(RawStack, len) as u32,
    );
    let local_stack_offset = translator
        .builder
        .isub_imm(stack_len_before_call, translator.stack_frame.size as i64);

    // Allocate the minimal stack size required by the callee.
    prepare_stack_for_call(translator);

    if let Some(result) = res.get(1) {
        let result_ok = translator.builder.iconst(JitType::I32, RESULT_OK);
        let is_ok = translator
            .builder
            .icmp(*result, result_ok, IntCompareOp::Equal);

        let continue_block = translator.builder.create_block(&[]);
        let suspend_block = translator.builder.create_block(&[]);

        translator
            .builder
            .brif(is_ok, continue_block, &[], suspend_block, &[]);

        translator.builder.switch_to_block(suspend_block);

        let mut rvals = Vec::new();

        if !caller.return_ty.is_empty() && !caller.return_ty.eq_ignore_ascii_case("none") {
            let ty = translator
                .data
                .resolve_ty_jit(&Lowercase::new(caller.return_ty.as_str()));
            rvals.push(translator.builder.uninit(ty));
        }

        rvals.push(*result);

        // The callee might move our current stack pointer, so we need
        // to reload it.
        let stack_base = translator
            .builder
            .load(JitType::Pointer, translator.ctx_ptr, 0);
        let stack_ptr = translator.builder.iadd(stack_base, local_stack_offset);

        translator.builder.switch_to_block(suspend_block);
        suspend_stack(translator, stack_ptr, stage);

        translator
            .span
            .exit(translator.data, &mut translator.builder, translator.ctx_ptr);

        translator.builder.return_(&rvals);

        translator.builder.switch_to_block(continue_block);

        *block = continue_block;
    }

    res.get(0).copied()
}

fn suspend_stack(translator: &mut FunctionTranslator<'_>, stack_base: JitValue, stage: &Stage) {
    let mut offset = 0;

    let index = translator.builder.iconst(JitType::I64, stage.index as i64);
    translator.builder.store(index, stack_base, 0);
    offset += JitType::I64.size();

    for var in &stage.variables {
        if offset % var.ty_jit.align() != 0 {
            offset += var.ty_jit.align() - (offset % var.ty_jit.align());
        }

        let value = translator.load_var(&var.name);
        translator.builder.store(value, stack_base, offset as u32);
        translator.store_var(&var.name, value);

        offset += var.ty_jit.size();
    }
}

#[derive(Clone, Debug)]
struct ResumeStage<'a> {
    variables: &'a [StageVariable],
    stages: Vec<usize>,
    /// Max stage index (inclusive).
    max_stage_index: usize,
}

fn resumse_stack(
    translator: &mut FunctionTranslator<'_>,
    stages: &[Stage],
    resume_points: &HashMap<usize, Block>,
    start_block: Block,
) {
    let stack_base = translator
        .builder
        .load(JitType::Pointer, translator.ctx_ptr, 0);

    if resume_points.is_empty() {
        translator.builder.jump(start_block, &[]);
        return;
    }

    let index = translator.builder.load(JitType::I64, stack_base, 0);
    let offset = JitType::I64.size();

    if stages.len() == 0 {
        translator.builder.jump(start_block, &[]);
        translator.builder.switch_to_block(start_block);
        return;
    }

    let match_index_block = translator.builder.create_block(&[]);

    // Don't resume if index == 0.
    translator
        .builder
        .brif(index, match_index_block, &[], start_block, &[]);

    // There commonly are suspension points with the same active variables.
    // Variable resumption can generate a lot of code, so we group the loading
    // code for stages with the same variables together.
    let mut resume_stages = Vec::<ResumeStage<'_>>::new();
    for stage in stages {
        if let Some(last) = resume_stages.last_mut()
            && last.variables == stage.variables
        {
            debug_assert!(last.max_stage_index < stage.index);
            last.stages.push(stage.index);
            last.max_stage_index = stage.index;
        } else {
            resume_stages.push(ResumeStage {
                variables: &stage.variables,
                stages: vec![stage.index],
                max_stage_index: stage.index,
            });
        }
    }

    let mut loading_blocks = Vec::new();
    for stage in resume_stages.iter() {
        let load_block = translator.builder.create_block(&[]);
        translator.builder.switch_to_block(load_block);
        loading_blocks.push(load_block);

        let mut offset = offset;
        for var in stage.variables {
            if offset % var.ty_jit.align() != 0 {
                offset += var.ty_jit.align() - (offset % var.ty_jit.align());
            }

            let value = translator
                .builder
                .load(var.ty_jit, stack_base, offset as u32);
            translator.store_var(&var.name, value);

            offset += var.ty_jit.size();
        }

        debug_assert!(!stage.stages.is_empty());

        if stage.stages.len() == 1 {
            translator
                .builder
                .jump(resume_points[&stage.stages[0]], &[]);
            continue;
        }

        for (i, stage_index) in stage.stages.iter().enumerate() {
            // No next stage. `index` is guaranteed to be a valid stage index, so it must
            // be this stage.
            if stage.stages.get(i + 1).is_none() {
                translator.builder.jump(resume_points[stage_index], &[]);
                continue;
            }

            let stage_index_value = translator.builder.iconst(JitType::I64, *stage_index as i64);
            let is_eq = translator
                .builder
                .icmp(index, stage_index_value, IntCompareOp::Equal);

            let next_block = translator.builder.create_block(&[]);

            translator
                .builder
                .brif(is_eq, resume_points[stage_index], &[], next_block, &[]);

            translator.builder.switch_to_block(next_block);
        }
    }

    translator.builder.switch_to_block(match_index_block);

    for (i, (stage, block)) in resume_stages.iter().zip(loading_blocks).enumerate() {
        if resume_stages.get(i + 1).is_none() {
            translator.builder.jump(block, &[]);
            continue;
        }

        let max_stage_index = translator
            .builder
            .iconst(JitType::I64, stage.max_stage_index as i64);
        let is_eq = translator.builder.icmp(
            index,
            max_stage_index,
            IntCompareOp::UnsignedLessThanOrEqual,
        );

        let next_block = translator.builder.create_block(&[]);

        translator.builder.brif(is_eq, block, &[], next_block, &[]);
        translator.builder.switch_to_block(next_block);
    }
}

/// Generate function abort.
fn abort_function(translator: &mut FunctionTranslator<'_>) {
    let mut rvals = vec![];

    if !translator.function.return_ty.is_empty()
        && !translator.function.return_ty.eq_ignore_ascii_case("none")
    {
        let ty = translator
            .data
            .resolve_ty_jit(&Lowercase::new(translator.function.return_ty.as_str()));
        rvals.push(translator.builder.uninit(ty));
    }

    let result = translator.builder.iconst(JitType::I32, RESULT_ERROR);
    rvals.push(result);

    translator
        .span
        .exit(translator.data, &mut translator.builder, translator.ctx_ptr);
    pop_stack_frame(translator);

    translator.builder.return_(&rvals);
}

fn struct_member_offset(
    translator: &mut FunctionTranslator<'_>,
    struct_value: &Value,
    member_name: &Value,
) -> u32 {
    let Value::Ident(member_name) = &member_name else {
        unreachable!();
    };

    let struct_name = translator.get_ty_name(&struct_value);
    let st = translator.data.structs.get(&struct_name).unwrap();

    let offset = st
        .field_offsets
        .get(&Lowercase::new(&**member_name))
        .unwrap();
    *offset as u32
}

fn call_malloc(
    translator: &mut FunctionTranslator<'_>,
    type_id: TypeId,
    array_len: JitValue,
) -> JitValue {
    let type_id = translator
        .builder
        .iconst(JitType::Pointer, type_id.as_bits() as i64);

    let res = translator.builder.call(
        translator.data.runtime_funcs.alloc_zeroed,
        &[translator.ctx_ptr, type_id, array_len],
    );
    res[0]
}

fn emit_array_bounds_check(
    translator: &mut FunctionTranslator<'_>,
    block: &mut Block,
    array: JitValue,
    index: JitValue,
    count: JitValue,
) {
    let abort_block = translator.builder.create_block(&[]);
    let is_non_null_block = translator.builder.create_block(&[]);

    translator
        .builder
        .brif(array, is_non_null_block, &[], abort_block, &[]);

    translator.builder.switch_to_block(is_non_null_block);

    // Bounds check
    let array_len =
        translator
            .builder
            .load(JitType::Pointer, array, offset_of!(ArrayObject, len) as u32);

    // Papyrus integers are i32, but the array len is usize.
    let index_wide = translator.builder.uextend(index, JitType::Pointer);
    let count_wide = translator.builder.uextend(count, JitType::Pointer);
    let index_plus_count = translator.builder.iadd(index_wide, count_wide);

    // Negative indices are always out of bounds.
    let zero = translator.builder.iconst(JitType::Pointer, 0);
    let is_not_negative = translator.builder.icmp(
        index_plus_count,
        zero,
        IntCompareOp::SignedGreaterThanOrEqual,
    );

    // index + count <= len
    let is_within_len = translator.builder.icmp(
        index_plus_count,
        array_len,
        IntCompareOp::UnsignedLessThanOrEqual,
    );

    let is_ok = translator.builder.band(is_not_negative, is_within_len);

    let continue_block = translator.builder.create_block(&[]);
    translator
        .builder
        .brif(is_ok, continue_block, &[], abort_block, &[]);

    translator.builder.switch_to_block(abort_block);
    abort_function(translator);

    translator.builder.switch_to_block(continue_block);
    *block = continue_block;
}

/// Returns a truthy value if lhs == rhs.
fn check_equality(
    translator: &mut FunctionTranslator<'_>,
    ty: &Type,
    lhs: JitValue,
    rhs: JitValue,
    block: &mut Block,
) -> JitValue {
    match ty {
        // None types are trivially equal to itself.
        Type::None => translator.builder.iconst(JitType::I8, 1),
        Type::Bool => translator.builder.icmp(lhs, rhs, IntCompareOp::Equal),
        Type::Int => translator.builder.icmp(lhs, rhs, IntCompareOp::Equal),
        Type::Float => translator.builder.fcmp(lhs, rhs, FloatCompareOp::Equal),
        Type::String => {
            // if lhs.len != rhs.len the strings are not equal.
            // if lhs.ptr == rhs.ptr && lhs.len == rhs.len both strings are equal.
            // Otherwise we need to memcmp the strings.

            let false_u8 = translator.builder.iconst(JitType::I8, 0);
            let true_u8 = translator.builder.iconst(JitType::I8, 1);
            let zero_i32 = translator.builder.iconst(JitType::I32, 0);

            let lhs_ptr =
                translator
                    .builder
                    .load(JitType::Pointer, lhs, offset_of!(ArrayObject, ptr) as u32);
            let lhs_len =
                translator
                    .builder
                    .load(JitType::Pointer, lhs, offset_of!(ArrayObject, len) as u32);
            let rhs_ptr =
                translator
                    .builder
                    .load(JitType::Pointer, rhs, offset_of!(ArrayObject, ptr) as u32);
            let rhs_len =
                translator
                    .builder
                    .load(JitType::Pointer, rhs, offset_of!(ArrayObject, len) as u32);

            let final_block = translator.builder.create_block(&[JitType::I8]);
            let cmp_ptr_block = translator.builder.create_block(&[]);
            let memcmp_block = translator.builder.create_block(&[]);

            let len_eq = translator
                .builder
                .icmp(lhs_len, rhs_len, IntCompareOp::Equal);
            translator
                .builder
                .brif(len_eq, cmp_ptr_block, &[], final_block, &[false_u8]);

            translator.builder.switch_to_block(cmp_ptr_block);
            let ptr_eq = translator
                .builder
                .icmp(lhs_ptr, rhs_len, IntCompareOp::Equal);
            translator
                .builder
                .brif(ptr_eq, final_block, &[true_u8], memcmp_block, &[]);

            translator.builder.switch_to_block(memcmp_block);
            // memcmp returns 0 if both sides are equal.
            let is_not_eq = translator.builder.memcmp(lhs_ptr, rhs_ptr, lhs_len);
            let is_eq = translator
                .builder
                .icmp(is_not_eq, zero_i32, IntCompareOp::Equal);
            translator.builder.jump(final_block, &[is_eq]);

            translator.builder.switch_to_block(final_block);
            *block = final_block;
            translator.builder.block_params(final_block)[0]
        }
        Type::Class(_) => translator.builder.icmp(lhs, rhs, IntCompareOp::Equal),
        Type::Struct(_) => translator.builder.icmp(lhs, rhs, IntCompareOp::Equal),
        ty => todo!("{:?}", ty),
    }
}

fn create_default_value(translator: &mut FunctionTranslator<'_>, ty: &Type) -> JitValue {
    match ty {
        Type::None | Type::Int | Type::Bool => translator.builder.iconst(JitType::I32, 0),
        Type::Float => translator.builder.fconst(JitType::F32, 0.0),
        Type::String | Type::Struct(_) | Type::Class(_) | Type::Array(_) | Type::Var => {
            translator.builder.iconst(JitType::Pointer, 0)
        }
    }
}
