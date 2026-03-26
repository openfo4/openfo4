use std::mem::offset_of;

use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{
    Block, Compiler, FloatCompareOp, FuncId, FunctionBuilder, IntCompareOp, Signature,
    Type as JitType, Value as JitValue, Variable as JitVariable,
};
use openfo4_pex::{Function, Instruction, Value};

use crate::compiler::codegen::builtin::call_builtin;
use crate::compiler::codegen::constant::{ConstantData, create_string_constant};
use crate::compiler::codegen::function_props::{
    FunctionProps, StackFrame, Stage, StageVariable, function_properties,
};
use crate::compiler::codegen::layout::{compute_struct_type_layout, generate_type_layout};
use crate::compiler::codegen::trampoline::generate_guest_to_host_trampoline;
use crate::compiler::codegen::{
    ClassDef, CodegenData, Error, FunctionDef, Type, get_parent_class_with_property,
};
use crate::compiler::{NativeFunction, TypeLayout};
use crate::runtime::object::{ARRAY_LAYOUT, BYTE_LAYOUT, VAR_LAYOUT, VarObject};
use crate::runtime::{ArrayObject, ObjectTag, RESULT_ERROR, RESULT_OK};

#[derive(Debug)]
pub struct FunctionResult {
    pub id: FuncId,
    pub signature: Signature,
    pub constants: Vec<ConstantData>,
    pub layouts: Vec<Box<TypeLayout>>,
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
                ptr: core::ptr::null(),
            },
            None => {
                return Err(Error::UndefNativeFunc(format!(
                    "{}.{}",
                    function.class, function.function.name
                )));
            }
        };

        let id = generate_guest_to_host_trampoline(compiler, &signature, &native);
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

    let mut constants = Vec::new();
    let mut layouts = Vec::new();

    let func_id = function.func_id.unwrap();

    let func_id = if function.function.is_native {
        func_id
    } else {
        tracing::debug!("compiling {}.{}", function.class, function.function.name);

        let builder = compiler.define_function(func_id);
        translate_function_body(
            data,
            class,
            &function.function,
            &props,
            builder,
            &mut constants,
            &mut layouts,
        );

        func_id
    };

    FunctionResult {
        id: func_id,
        signature,
        constants,
        layouts,
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
    function: &Function,
    props: &FunctionProps,
    mut builder: FunctionBuilder<'_>,
    constants: &mut Vec<ConstantData>,
    layouts: &mut Vec<Box<TypeLayout>>,
) {
    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx_ptr = builder.block_params(entry_block)[0];

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
        constants,
        layouts,
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

    resumse_stack(
        &mut translator,
        &props.stages,
        &resume_points,
        start_block,
        ctx_ptr,
    );
    translator.builder.switch_to_block(start_block);

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
                        ctx_ptr,
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

                let res = if translator.get_type(&op.lhs) == Type::Float {
                    let cmp_op = match instruction {
                        Instruction::CmpEq(_) => FloatCompareOp::Equal,
                        Instruction::CmpLt(_) => FloatCompareOp::LessThan,
                        Instruction::CmpLe(_) => FloatCompareOp::LessThanOrEqual,
                        Instruction::CmpGt(_) => FloatCompareOp::GreaterThan,
                        Instruction::CmpGe(_) => FloatCompareOp::GreaterThanOrEqual,
                        _ => unreachable!(),
                    };

                    translator.builder.fcmp(lhs, rhs, cmp_op)
                } else {
                    let cmp_op = match instruction {
                        Instruction::CmpEq(_) => IntCompareOp::Equal,
                        Instruction::CmpLt(_) => IntCompareOp::SignedLessThan,
                        Instruction::CmpLe(_) => IntCompareOp::SignedLessThanOrEqual,
                        Instruction::CmpGt(_) => IntCompareOp::SignedGreaterThan,
                        Instruction::CmpGe(_) => IntCompareOp::SignedGreaterThanOrEqual,
                        _ => unreachable!(),
                    };

                    translator.builder.icmp(lhs, rhs, cmp_op)
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
                    ctx_ptr,
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
                    ctx_ptr,
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
                                ctx_ptr,
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
                        ctx_ptr,
                    )
                    .unwrap()
                } else {
                    let class_name =
                        get_parent_class_with_property(&data.classes, &class_name, &prop_name)
                            .unwrap();

                    let class = data.get_class(&class_name).unwrap();
                    let layout = class.layout.as_ref().unwrap();

                    let prop = class.properties.get(&prop_name).unwrap();
                    let offset = layout.offset(&prop.variable.as_ref().unwrap()).unwrap();

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
                        ctx_ptr,
                    );
                } else {
                    let class_name =
                        get_parent_class_with_property(&data.classes, &class_name, &prop_name)
                            .unwrap();

                    let class = data.get_class(&class_name).unwrap();
                    let layout = class.layout.as_ref().unwrap();

                    let prop = class.properties.get(&prop_name).unwrap();
                    let offset = layout.offset(&prop.variable.as_ref().unwrap()).unwrap();

                    translator.builder.store(value, object, offset as u32);
                }
            }
            Instruction::Return(value) => {
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
                let res = perform_cast(
                    &mut translator,
                    ctx_ptr,
                    &op.src,
                    &op.dst,
                    &mut current_block,
                );
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

                let layout = compute_struct_type_layout(translator.data, st);
                let one = translator.builder.iconst(JitType::Pointer, 1);
                let ptr = call_malloc(&mut translator, ctx_ptr, layout, one);

                // Default-initialize all struct fields.
                let layout = st.layout(translator.data);
                for member in &st.s.members {
                    let ty = Lowercase::new(&*member.ty);
                    let name = Lowercase::new(&*member.name);

                    let offset = layout.offset(&name).unwrap();
                    let ty = translator.data.resolve_ty(&ty);
                    let value = create_default_value(&mut translator, &ty);
                    translator.builder.store(value, ptr, offset as u32);
                }

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

                let array_len = translator.builder.load(
                    JitType::Pointer,
                    src,
                    offset_of!(ArrayObject, len) as u32,
                );

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

                let layout = generate_type_layout(translator.data, &elem_ty);
                let array_len = translator.load(&op.src);
                let array_len = translator.builder.uextend(array_len, JitType::Pointer);
                let array_ptr = call_malloc(&mut translator, ctx_ptr, layout, array_len);

                let one_usize = translator.builder.iconst(JitType::Pointer, 1);
                let array_object_ptr =
                    call_malloc(&mut translator, ctx_ptr, ARRAY_LAYOUT.clone(), one_usize);

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

                let array = translator.load(&op.array);
                let value = translator.load(&op.value);
                let count = translator.load(&op.count);

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

                let layout = generate_type_layout(translator.data, &elem_ty);
                let stride = layout.size();
                let new_array_ptr = call_malloc(&mut translator, ctx_ptr, layout, new_len);

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
                let layout = generate_type_layout(data, &elem_ty);
                let stride = layout.size();
                let new_array_ptr = call_malloc(&mut translator, ctx_ptr, layout, new_len);

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
                let layout = generate_type_layout(translator.data, &elem_ty);
                let stride = layout.size();
                let new_array_ptr = call_malloc(&mut translator, ctx_ptr, layout, new_len);

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
                            let layout = struct_def.layout(translator.data);

                            let member = struct_def
                                .s
                                .members
                                .iter()
                                .find(|v| v.name.to_lowercase() == member_name.to_lowercase())
                                .unwrap();

                            let offset = *layout
                                .field_offsets
                                .get(&Lowercase::new(member_name.as_str()))
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
                let array_lhs = translator.load(&op.lhs);
                let array_rhs = translator.load(&op.rhs);

                let lhs_ptr = translator.builder.load(
                    JitType::Pointer,
                    array_lhs,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                let rhs_ptr = translator.builder.load(
                    JitType::Pointer,
                    array_rhs,
                    offset_of!(ArrayObject, ptr) as u32,
                );

                let lhs_len = translator.builder.load(
                    JitType::Pointer,
                    array_lhs,
                    offset_of!(ArrayObject, len) as u32,
                );
                let rhs_len = translator.builder.load(
                    JitType::Pointer,
                    array_rhs,
                    offset_of!(ArrayObject, len) as u32,
                );
                let new_len = translator.builder.iadd(lhs_len, rhs_len);

                // Allocate new string byte buffer and copy over bytes.
                let new_ptr = call_malloc(&mut translator, ctx_ptr, BYTE_LAYOUT.clone(), new_len);
                translator.builder.memcpy(lhs_ptr, new_ptr, lhs_len);
                let rhs_dst_ptr = translator.builder.iadd(new_ptr, lhs_len);
                translator.builder.memcpy(rhs_ptr, rhs_dst_ptr, rhs_len);

                // Allocate new string object.
                let one = translator.builder.iconst(JitType::Pointer, 1);
                let new_string_ptr =
                    call_malloc(&mut translator, ctx_ptr, ARRAY_LAYOUT.clone(), one);

                translator.builder.store(
                    new_ptr,
                    new_string_ptr,
                    offset_of!(ArrayObject, ptr) as u32,
                );
                translator.builder.store(
                    new_len,
                    new_string_ptr,
                    offset_of!(ArrayObject, len) as u32,
                );

                translator.store(&op.dst, new_string_ptr);
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
    constants: &'a mut Vec<ConstantData>,
    layouts: &'a mut Vec<Box<TypeLayout>>,
}

impl<'a> FunctionTranslator<'a> {
    pub fn load(&mut self, value: &Value) -> JitValue {
        match value {
            Value::Null => self.builder.iconst(JitType::I64, 0),
            Value::Bool(value) => self.builder.iconst(JitType::I32, u8::from(*value)),
            Value::Int(value) => self.builder.iconst(JitType::I32, *value),
            Value::Float(value) => self.builder.fconst(JitType::F32, *value),
            Value::String(value) => {
                // Empty strings must be represented as a zero pointer.
                if value.is_empty() {
                    self.builder.iconst(JitType::Pointer, 0)
                } else {
                    // TODO: We can deduplicate strings here, even across functions calls.
                    self.builder.iconst(JitType::Pointer, 0);
                    let ptr = create_string_constant(self.constants, &value);
                    self.builder.iconst(JitType::Pointer, ptr.addr() as i64)
                }
            }
            Value::Ident(name) => self.load_var(&Lowercase::new(name.as_str())),
        }
    }

    pub fn load_var(&mut self, name: &Lowercase<str>) -> JitValue {
        match name.as_inner() {
            "self" => return self.self_ptr.unwrap(),
            "::state" => {
                // TODO
                return self.builder.iconst(JitType::Pointer, 0);
            }
            _ => {}
        }

        if let Some(local) = self.locals.get(name) {
            return self.builder.get_var(local.variable);
        }

        let mut class = self.data.get_class(&self.self_ty).unwrap();
        loop {
            if let Some(field_ty) = class.fields.get(name) {
                let ty = self.data.resolve_ty_jit(&field_ty);
                let layout = class.layout.as_ref().unwrap();

                let offset = layout.offset(&name).unwrap();
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
            "::state" => {
                // TODO
                return;
            }
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
                let layout = class.layout.as_ref().unwrap();

                let offset = layout.offset(&name).unwrap();
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
    ctx_ptr: JitValue,
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
            let mut class_name = &src_class;
            loop {
                if class_name == &dst_class {
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
            let mut class_name = &dst_class;
            loop {
                if class_name == &src_class {
                    let is_ok = perform_typecheck(translator, src_val, &class_name, block);

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
        (Type::Var, Type::Bool) => perform_var_downcast(translator, src, ObjectTag::Bool, block),
        (Type::Var, Type::Int) => perform_var_downcast(translator, src, ObjectTag::Int, block),
        (Type::Var, Type::Float) => perform_var_downcast(translator, src, ObjectTag::Float, block),
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
        (src_ty, Type::Var) => {
            let tag = match src_ty {
                Type::None => ObjectTag::None,
                Type::Int => ObjectTag::Int,
                Type::Bool => ObjectTag::Bool,
                Type::Float => ObjectTag::Float,
                Type::String => ObjectTag::String,
                Type::Class(_) => ObjectTag::Class,
                Type::Var => unreachable!(),
                x => todo!("{:?}", x),
            };

            create_var(translator, ctx_ptr, src, tag)
        }
        // String formatting conversions
        (Type::Int, Type::String) => {
            let ptr = translator
                .builder
                .call(translator.data.runtime_funcs.i32_to_string, &[ctx_ptr, src])[0];
            ptr
        }
        (Type::Float, Type::String) => {
            let ptr = translator
                .builder
                .call(translator.data.runtime_funcs.f32_to_string, &[ctx_ptr, src])[0];
            ptr
        }
        (Type::Class(_), Type::String) => {
            let ptr = translator.builder.call(
                translator.data.runtime_funcs.object_to_string,
                &[ctx_ptr, src],
            )[0];
            ptr
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
        .iconst(JitType::Pointer, dst as usize as i64);
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
        ObjectTag::None => JitType::I32,
        ObjectTag::Bool => JitType::I32,
        ObjectTag::Int => JitType::I32,
        ObjectTag::Float => JitType::F32,
        ObjectTag::Class => JitType::Pointer,
        ObjectTag::String => JitType::Pointer,
    };

    translator
        .builder
        .load(ty, src, size_of::<ObjectTag>() as u32)
}

/// Creates a new `Var`.
fn create_var(
    translator: &mut FunctionTranslator<'_>,
    ctx_ptr: JitValue,
    value: JitValue,
    tag: ObjectTag,
) -> JitValue {
    // Allocate var object
    let one = translator.builder.iconst(JitType::Pointer, 1);
    let ptr = call_malloc(translator, ctx_ptr, VAR_LAYOUT.clone(), one);

    // Store tag
    let tag = translator.builder.iconst(JitType::Pointer, tag as i64);
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
    let dst_id = translator.builder.iconst(JitType::I64, dst_id as i64);

    let type_info_ptr = translator.builder.load(JitType::Pointer, src, 0);

    // The first word of an object contains a constant pointer to the type
    // info. The type info starts with a single word containing the number
    // of type ids that follow.
    // If any type id matches we have a valid object of that class.

    let count = translator.builder.load(JitType::Pointer, type_info_ptr, 0);

    let next_block = translator.builder.create_block(&[]);

    // Note that every object has at least one typeid. This means count is
    // always >= 1.
    let iter_block = translator.builder.create_block(&[JitType::I64]);

    translator.builder.jump(iter_block, &[count]);
    translator.builder.switch_to_block(iter_block);
    let offset = translator.builder.block_params(iter_block)[0];

    // Load and check current offset.
    let word_size = translator
        .builder
        .iconst(JitType::I64, JitType::I64.size() as i64);
    let byte_offset = translator.builder.imul(offset, word_size);
    let addr = translator.builder.iadd(type_info_ptr, byte_offset);
    let type_id = translator.builder.load(JitType::I64, addr, 0);

    // Decrement offset.
    let one = translator.builder.iconst(JitType::I64, 1);
    let next_offset = translator.builder.isub(offset, one);

    let is_eq = translator
        .builder
        .icmp(type_id, dst_id, IntCompareOp::Equal);

    // Equivalent to `next_offset != 0 && !is_eq`.
    let is_eq_wide = translator.builder.uextend(is_eq, JitType::I64);
    let is_neq = translator.builder.bnot(is_eq_wide);
    let should_continue = translator.builder.band(next_offset, is_neq);

    translator
        .builder
        .brif(should_continue, iter_block, &[next_offset], next_block, &[]);

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
    ctx_ptr: JitValue,
) -> Option<JitValue> {
    let func_id = callee.func_id.unwrap();

    // We need to be able to jump straight to the function call
    // to resume a suspended call.
    let call_block = translator.builder.create_block(&[]);
    translator.builder.jump(call_block, &[]);
    translator.builder.switch_to_block(call_block);

    // Allocate stack space for the callee.
    let stack_frame = &callee.props.as_ref().unwrap().stack_frame;
    push_stack_frame(translator, ctx_ptr, stack_frame);

    let res = translator.builder.call(func_id, args);

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

        // push_stack_frame might move our current stack pointer, so we need
        // to reload it, but the current stack pointer points to the end of the stack
        // for the callee. We must go backwards to find our stack frame.
        let stack_base = translator.builder.load(JitType::Pointer, ctx_ptr, 0);
        let stack_base = translator
            .builder
            .isub_imm(stack_base, stack_frame.size as i64);

        translator.builder.switch_to_block(suspend_block);
        suspend_stack(translator, stack_base, stage);
        translator.builder.return_(&rvals);

        translator.builder.switch_to_block(continue_block);

        // Restore stack pointer.
        pop_stack_frame(translator, ctx_ptr, stack_frame);

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
    ctx_ptr: JitValue,
) {
    let stack_base = translator.builder.load(JitType::Pointer, ctx_ptr, 0);

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

    let layout = st.layout(translator.data);

    let member_offset = layout
        .field_offsets
        .get(&Lowercase::new(member_name.as_str()))
        .unwrap();

    *member_offset as u32
}

fn call_malloc(
    translator: &mut FunctionTranslator<'_>,
    ctx_ptr: JitValue,
    layout: TypeLayout,
    array_len: JitValue,
) -> JitValue {
    let layout_boxed = Box::new(layout);
    let layout_ptr = Box::as_ref(&layout_boxed) as *const TypeLayout;
    translator.layouts.push(layout_boxed);

    let layout_ptr = translator
        .builder
        .iconst(JitType::Pointer, layout_ptr as i64);

    let res = translator.builder.call(
        translator.data.runtime_funcs.alloc_zeroed,
        &[ctx_ptr, layout_ptr, array_len],
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
    let abort_block = translator.builder.create_block(&[]);
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
        ty => todo!("{:?}", ty),
    }
}

fn push_stack_frame(
    translator: &mut FunctionTranslator<'_>,
    ctx_ptr: JitValue,
    frame: &StackFrame,
) -> JitValue {
    let size = translator
        .builder
        .iconst(JitType::Pointer, frame.size as i64);

    let stack_ptr = translator.builder.call(
        translator.data.runtime_funcs.stack_frame_alloc,
        &[ctx_ptr, size],
    )[0];

    translator.builder.store(stack_ptr, ctx_ptr, 0);
    stack_ptr
}

fn pop_stack_frame(translator: &mut FunctionTranslator<'_>, ctx_ptr: JitValue, frame: &StackFrame) {
    let size = translator
        .builder
        .iconst(JitType::Pointer, frame.size as i64);

    translator.builder.call(
        translator.data.runtime_funcs.stack_frame_free,
        &[ctx_ptr, size],
    );
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
