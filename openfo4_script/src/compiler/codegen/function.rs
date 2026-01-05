use std::collections::HashMap;
use std::mem::offset_of;
use std::sync::LazyLock;

use openfo4_jit::{
    Block, Compiler, FloatCompareOp, FunctionBuilder, FunctionId, IntCompareOp, Signature,
    StackSlot, Type as JitType, Value as JitValue, Variable as JitVariable,
};
use openfo4_pex::{Function, Instruction, Value};

use crate::compiler::TypeLayout;
use crate::compiler::codegen::builtin::call_builtin;
use crate::compiler::codegen::constant::{ConstantData, create_string_constant};
use crate::compiler::codegen::function_props::{
    FunctionProps, Stage, StageVariable, function_properties,
};
use crate::compiler::codegen::layout::{ARRAY_LAYOUT, generate_type_layout};
use crate::compiler::codegen::trampoline::generate_guest_to_host_trampoline;
use crate::compiler::codegen::{ClassDef, CodegenData, FunctionDef, Type};
use crate::runtime::{ArrayObject, Func, ObjectTag, RESULT_ERROR, RESULT_OK};

#[derive(Debug)]
pub struct FunctionResult {
    pub id: FunctionId,
    pub signature: Signature,
    pub constants: Vec<ConstantData>,
    pub layouts: Vec<Box<TypeLayout>>,
}

pub fn translate_function(
    data: &CodegenData,
    class: &ClassDef,
    function: &FunctionDef,
    compiler: &mut Compiler,
) -> FunctionResult {
    let signature = translate_function_signature(data, &function.function);
    let props = function_properties(data, function);

    debug_assert!(function.func_id.is_none());

    let mut constants = Vec::new();
    let mut layouts = Vec::new();

    let func_id = if function.function.is_native {
        let Some(native) = &function.native else {
            panic!("native function {} is not defined", function.function.name);
        };

        dbg!(&function.function.name);
        assert_eq!(signature, native.signature);

        let func_id = generate_guest_to_host_trampoline(compiler, &signature, native);

        func_id
    } else {
        let func_id = compiler.declare_function(signature.clone());

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
        let ty = data.resolve_ty_jit(&param.ty);
        params.push(ty);
    }

    if !function.return_ty.is_empty() && function.return_ty.to_lowercase() != "none" {
        let ty = data.resolve_ty_jit(&function.return_ty);
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

    let stack_base = builder.load(JitType::Pointer, ctx_ptr, 0);

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
        let ty = data.resolve_ty_jit(&param.ty);

        let value = builder.block_params(entry_block)[params_offset + index];
        let variable = builder.def_var(ty, value);

        locals.insert(
            param.name.to_lowercase(),
            Local {
                ty,
                ty_name: param.ty.to_lowercase(),
                variable,
            },
        );
    }

    // Create stack slots for local variables.
    for local in &function.locals {
        let ty = data.resolve_ty_jit(&local.ty);

        let value = match data.resolve_ty(&local.ty) {
            Type::None | Type::Bool | Type::Int => builder.iconst(JitType::I32, 0),
            Type::Float => builder.fconst(JitType::F32, 0.0),
            Type::String | Type::Class(_) | Type::Array(_) | Type::Struct(_) | Type::Var => {
                builder.iconst(JitType::Pointer, 0)
            }
        };

        let variable = builder.def_var(ty, value);

        locals.insert(
            local.name.to_lowercase(),
            Local {
                ty,
                ty_name: local.ty.to_lowercase(),
                variable,
            },
        );
    }

    let mut fields = HashMap::new();
    for (name, ty) in &class.fields {
        fields.insert(
            name.to_lowercase(),
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
        self_ty: class.name.to_lowercase(),
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
        stack_base,
        &props.stages,
        &resume_points,
        start_block,
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
                let src = translator.load(&op.src);
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
                let Value::Ident(object) = &op.object else {
                    unreachable!();
                };

                let function_name = match &op.function_name {
                    Value::Ident(v) => v,
                    _ => unreachable!(),
                };

                let find_var = |var_name: &str| -> String {
                    match &*var_name.to_lowercase() {
                        "self" => return class.name.clone(),
                        _ => (),
                    }

                    if let Some(local) = translator.locals.get(&var_name.to_lowercase()) {
                        return local.ty_name.clone();
                    }

                    if let Some(ty) = class.fields.get(&var_name.to_lowercase()) {
                        return ty.to_lowercase();
                    }

                    todo!()
                };

                let class_name = find_var(&object);

                let func = data.get_function(&class_name, &function_name).unwrap();

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
                    stack_base,
                ) {
                    translator.store(&op.return_assignment, res);
                }
            }
            Instruction::CallParent(op) => {}
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(class), Value::Ident(func)) => (class, func),
                    _ => unreachable!(),
                };

                let class_name = class_name.to_lowercase();
                let function_name = function_name.to_lowercase();

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
                                stack_base,
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
                        stack_base,
                    )
                    .unwrap()
                } else {
                    let class = data.get_class(&class_name).unwrap();
                    let layout = class.layout.as_ref().unwrap();

                    let prop = class.properties.get(&prop_name.to_lowercase()).unwrap();
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
                        stack_base,
                    );
                } else {
                    let class = data.get_class(&class_name).unwrap();
                    let layout = class.layout.as_ref().unwrap();

                    let prop = class.properties.get(&prop_name.to_lowercase()).unwrap();
                    let offset = layout.offset(&prop.variable.as_ref().unwrap()).unwrap();

                    translator.builder.store(value, object, offset as u32);
                }
            }
            Instruction::Return(value) => {
                if !function.return_ty.is_empty() && function.return_ty.to_lowercase() != "none" {
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

                let res =
                    perform_typecheck(&mut translator, &op.object, &class, &mut current_block);
                let res = translator.builder.uextend(res, JitType::I32);
                translator.store(&op.return_assignment, res);
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
                let src = translator.load(&op.lhs);
                let index = translator.load(&op.rhs);

                // Bounds check
                let array_len = translator.builder.load(
                    JitType::Pointer,
                    src,
                    offset_of!(ArrayObject, len) as u32,
                );

                // Papyrus integers are i32, but the array len is usize.
                let index_wide = translator.builder.uextend(index, JitType::Pointer);

                let is_ok =
                    translator
                        .builder
                        .icmp(index_wide, array_len, IntCompareOp::UnsignedLessThan);

                let continue_block = translator.builder.create_block(&[]);
                let abort_block = translator.builder.create_block(&[]);
                translator
                    .builder
                    .brif(is_ok, continue_block, &[], abort_block, &[]);

                translator.builder.switch_to_block(abort_block);
                abort_function(&mut translator);

                translator.builder.switch_to_block(continue_block);

                let array_ptr = translator.builder.load(
                    JitType::Pointer,
                    src,
                    offset_of!(ArrayObject, ptr) as u32,
                );

                let ptr_size = translator
                    .builder
                    .iconst(JitType::Pointer, JitType::Pointer.size() as i64);
                let offset = translator.builder.imul(index_wide, ptr_size);

                let addr = translator.builder.iadd(array_ptr, offset);
                translator.store(&op.dst, addr);
            }
            Instruction::ArrayCreate(op) => {
                let layout = generate_type_layout(translator.data, &translator.get_type(&op.src));
                let array_len = translator.load(&op.src);
                let array_len = translator.builder.uextend(array_len, JitType::Pointer);
                let array_ptr = call_malloc(&mut translator, ctx_ptr, layout, array_len);

                let one_usize = translator.builder.iconst(JitType::Pointer, 1);
                let array_object_ptr =
                    call_malloc(&mut translator, ctx_ptr, *ARRAY_LAYOUT.clone(), one_usize);

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
            op => todo!("{:?}", op),
        }
    }

    // Sometimes the final return instruction is missing and we must insert it.
    // This is only possible for functions that return "none" and the fact we
    // return "none" in this case is checked by the validation pass.
    // Other functions that don't return also can also sometimes refer to this
    // section due to weird pex codegen, however in that case this section
    // is statically unreachable.
    let return_ty = (!function.return_ty.is_empty() && function.return_ty.to_lowercase() != "none")
        .then(|| data.resolve_ty_jit(&function.return_ty));

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
    locals: HashMap<String, Local>,
    fields: HashMap<String, Field>,
    self_ptr: Option<JitValue>,
    self_ty: String,
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
            Value::Ident(name) => self.load_var(name),
        }
    }

    pub fn load_var(&mut self, name: &str) -> JitValue {
        let name = name.to_lowercase();

        if name == "self" {
            return self.self_ptr.unwrap();
        }

        if name == "::state" {
            // TODO
            return self.builder.iconst(JitType::Pointer, 0);
        }

        if let Some(local) = self.locals.get(&name.to_lowercase()) {
            return self.builder.get_var(local.variable);
        }

        if let Some(field) = self.fields.get(&name.to_lowercase()) {
            let class = self.data.get_class(&self.self_ty).unwrap();
            let layout = class.layout.as_ref().unwrap();

            let offset = layout.offset(&name).unwrap();
            return self
                .builder
                .load(field.ty, self.self_ptr.unwrap(), offset as u32);
        }

        panic!("undefined variable {}", name);
    }

    pub fn store(&mut self, slot: &Value, value: JitValue) {
        let Value::Ident(name) = slot else {
            return;
        };

        self.store_var(name, value);
    }

    pub fn store_var(&mut self, name: &str, value: JitValue) {
        if name.to_lowercase() == "::state" {
            // TODO
            return;
        }

        // ::nonevar is a magic variable that always holds none.
        // Writing to it discards the value.
        if name.to_lowercase() == "::nonevar" {
            return;
        }

        if let Some(local) = self.locals.get(&name.to_lowercase()) {
            self.builder.set_var(local.variable, value);
            return;
        }

        if let Some(_) = self.fields.get(&name.to_lowercase()) {
            let class = self.data.get_class(&self.self_ty).unwrap();
            let layout = class.layout.as_ref().unwrap();

            let offset = layout.offset(&name).unwrap();
            self.builder
                .store(value, self.self_ptr.unwrap(), offset as u32);

            return;
        }

        panic!("undefiend variable {}", name);
    }

    pub fn get_type(&self, value: &Value) -> Type {
        match value {
            Value::Null => Type::None,
            Value::Bool(_) => Type::Bool,
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::String(_) => Type::String,
            Value::Ident(name) => {
                let name = name.to_lowercase();

                if name == "self" {
                    return self.data.resolve_ty(&self.self_ty);
                };

                if let Some(local) = self.locals.get(&name.to_lowercase()) {
                    return self.data.resolve_ty(&local.ty_name);
                }

                if let Some(global) = self.fields.get(&name.to_lowercase()) {
                    return self.data.resolve_ty(&global.ty_name);
                }

                panic!("undefined variable {}", name);
            }
        }
    }

    pub fn get_ty_name(&self, value: &Value) -> String {
        match value {
            Value::Null => "none".to_owned(),
            Value::Bool(_) => "bool".to_owned(),
            Value::Int(_) => "int".to_owned(),
            Value::Float(_) => "float".to_owned(),
            Value::String(_) => "string".to_owned(),
            Value::Ident(name) => {
                if name == "self" {
                    return self.self_ty.to_lowercase();
                };

                if let Some(local) = self.locals.get(&name.to_lowercase()) {
                    return local.ty_name.to_lowercase();
                }

                if let Some(global) = self.fields.get(&name.to_lowercase()) {
                    return global.ty_name.to_lowercase();
                }

                panic!();
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Field {
    ty: JitType,
    ty_name: String,
}

#[derive(Clone, Debug)]
struct Local {
    ty: JitType,
    ty_name: String,
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
            let mut class_name = src_class.to_lowercase();
            loop {
                if class_name == dst_class.to_lowercase() {
                    return src;
                }

                let class = translator.data.classes.get(&class_name).unwrap();
                if let Some(parent) = &class.parent {
                    class_name = parent.to_lowercase();
                } else {
                    break;
                }
            }

            // Try downcasting.
            let mut class_name = dst_class.to_lowercase();
            loop {
                if class_name == src_class.to_lowercase() {
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

                let class = translator.data.classes.get(&class_name).unwrap();
                if let Some(parent) = &class.parent {
                    class_name = parent.to_lowercase();
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
            let is_ok = perform_class_typecheck(translator, res, &class_name.to_lowercase(), block);

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
        ObjectTag::Bool => JitType::I32,
        ObjectTag::Int => JitType::I32,
        ObjectTag::Float => JitType::F32,
        ObjectTag::Class => JitType::Pointer,
    };

    translator
        .builder
        .load(ty, src, size_of::<ObjectTag>() as u32)
}

/// Performs a runtime typecheck whether `src` is `dst_class` or a subtype of `dst_class`.
fn perform_typecheck(
    translator: &mut FunctionTranslator<'_>,
    src: &Value,
    dst_class: &str,
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
    dst_class: &str,
    block: &mut Block,
) -> JitValue {
    let dst_id = translator
        .data
        .classes
        .get(&dst_class.to_lowercase())
        .unwrap()
        .type_id;
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
    stack_base: JitValue,
) -> Option<JitValue> {
    let func_id = callee.func_id.unwrap();

    // We need to be able to jump straight to the function call
    // to resume a suspended call.
    let call_block = translator.builder.create_block(&[]);
    translator.builder.jump(call_block, &[]);
    translator.builder.switch_to_block(call_block);

    // Set the stack base pointer for the callee.
    // Note that we don't need to reset this value after the call because we use our
    // initially loaded local value for all future uses.
    let local_stack_size = translator.builder.iconst(JitType::I64, stage.size() as i64);
    let new_stack_base = translator.builder.iadd(stack_base, local_stack_size);
    translator.builder.store(new_stack_base, ctx_ptr, 0);

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

        if !caller.return_ty.is_empty() && caller.return_ty.to_lowercase() != "none" {
            let ty = translator.data.resolve_ty_jit(&caller.return_ty);
            rvals.push(translator.builder.uninit(ty));
        }

        rvals.push(*result);

        translator.builder.switch_to_block(suspend_block);
        suspend_stack(translator, stack_base, stage);
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
    stack_base: JitValue,
    stages: &[Stage],
    resume_points: &HashMap<usize, Block>,
    start_block: Block,
) {
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
                translator.builder.jump(resume_points[&stage_index], &[]);
                continue;
            }

            let stage_index_value = translator.builder.iconst(JitType::I64, *stage_index as i64);
            let is_eq = translator
                .builder
                .icmp(index, stage_index_value, IntCompareOp::Equal);

            let next_block = translator.builder.create_block(&[]);

            translator
                .builder
                .brif(is_eq, resume_points[&stage_index], &[], next_block, &[]);

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
        && translator.function.return_ty.to_lowercase() != "none"
    {
        let ty = translator
            .data
            .resolve_ty_jit(&translator.function.return_ty);
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
        .get(&member_name.to_lowercase())
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

    let res = translator
        .builder
        .call(translator.data.malloc, &[ctx_ptr, layout_ptr, array_len]);
    res[0]
}
