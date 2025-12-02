use std::collections::{HashMap, HashSet};

use openfo4_jit::{
    Block, Compiler, FunctionBuilder, FunctionId, IntCompareOp, Signature, StackSlot,
    Type as JitType, Value as JitValue,
};
use openfo4_pex::{Function, Instruction, Value};

use crate::compiler::codegen::trampoline::generate_guest_to_host_trampoline;
use crate::compiler::codegen::{ClassDef, CodegenData, FunctionDef, Type};

#[derive(Clone, Debug)]
pub struct FunctionResult {
    pub id: FunctionId,
    pub signature: Signature,
}

pub fn translate_function(
    data: &CodegenData,
    class: &ClassDef,
    function: &FunctionDef,
    compiler: &mut Compiler,
) -> FunctionResult {
    let signature = translate_function_signature(data, &function.function);

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
        translate_function_body(data, class, &function.function, builder);

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
        let ty = data.resolve_ty_jit(&param.ty);
        params.push(ty);
    }

    if !function.return_ty.is_empty() && function.return_ty.to_lowercase() != "none" {
        let ty = data.resolve_ty_jit(&function.return_ty);
        returns.push(ty);
    }

    Signature { params, returns }
}

fn translate_function_body(
    data: &CodegenData,
    class: &ClassDef,
    function: &Function,
    mut builder: FunctionBuilder<'_>,
) {
    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);

    let ctx_ptr = builder.block_params(entry_block)[0];
    let self_ptr = builder.block_params(entry_block).get(1).copied();

    let mut locals = HashMap::with_capacity(function.locals.len());

    // Create stack slots for local variables.
    for local in &function.locals {
        let ty = data.resolve_ty_jit(&local.ty);

        let slot = builder.create_typed_stack_slot(ty);
        locals.insert(
            local.name.to_lowercase(),
            Local {
                ty,
                slot,
                ty_name: local.ty.to_lowercase(),
            },
        );

        let value = builder.iconst(JitType::I32, 0);
        builder.stack_store(slot, value, 0);
    }

    // Move function parameters into stack slots.
    for (index, param) in function.params.iter().enumerate() {
        let ty = data.resolve_ty_jit(&param.ty);

        let slot = builder.create_typed_stack_slot(ty);

        locals.insert(
            param.name.to_lowercase(),
            Local {
                ty,
                slot,
                ty_name: param.ty.to_lowercase(),
            },
        );

        let value = builder.block_params(entry_block)[index];
        builder.stack_store(slot, value, 0);
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
        data,
        builder,
        locals,
        fields,
        self_ptr,
        self_ty: class.name.to_lowercase(),
    };

    // Setup blocks
    let mut blocks = HashMap::new();
    blocks.insert(0, entry_block);

    let mut unterminated_blocks = HashSet::new();
    unterminated_blocks.insert(entry_block);

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
                    unterminated_blocks.insert(block);
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
                    unterminated_blocks.insert(then_block);
                }

                let next_addr = index + 1;
                if !blocks.contains_key(&next_addr) {
                    let else_block = translator.builder.create_block(&[]);
                    blocks.insert(next_addr, else_block);
                    unterminated_blocks.insert(else_block);
                }
            }
            _ => {}
        }
    }

    let mut current_block = entry_block;
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
            Instruction::Store(op) => {
                let value = translator.load(&op.src);
                translator.store(&op.dst, value);
            }
            Instruction::CmpEq(op)
            | Instruction::CmpLt(op)
            | Instruction::CmpLe(op)
            | Instruction::CmpGt(op)
            | Instruction::CmpGe(op) => {
                let cmp_op = match instruction {
                    Instruction::CmpEq(_) => IntCompareOp::Equal,
                    Instruction::CmpLt(_) => IntCompareOp::SignedLessThan,
                    Instruction::CmpLe(_) => IntCompareOp::SignedLessThanOrEqual,
                    Instruction::CmpGt(_) => IntCompareOp::SignedGreaterThan,
                    Instruction::CmpGe(_) => IntCompareOp::SignedGreaterThanOrEqual,
                    _ => unreachable!(),
                };

                let lhs = translator.load(&op.lhs);
                let rhs = translator.load(&op.rhs);
                let res = translator.builder.icmp(lhs, rhs, cmp_op);
                translator.store(&op.dst, res);
            }
            Instruction::Branch(offset) => {
                unterminated_blocks.remove(&current_block);

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
                unterminated_blocks.remove(&current_block);

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
                unterminated_blocks.remove(&current_block);

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
                let func = func.func_id.unwrap();

                let mut args = op
                    .args
                    .iter()
                    .map(|v| translator.load(v))
                    .collect::<Vec<_>>();

                // In our ABI the first argument is the object pointer.
                let object = translator.load(&op.object);
                args.insert(0, object);

                args.insert(0, ctx_ptr);

                let returns = translator.builder.call(func, &args);
                if let Some(res) = returns.get(0).copied() {
                    translator.store(&op.return_assignment, res);
                }
            }
            Instruction::CallParent(op) => {}
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(class), Value::Ident(func)) => (class, func),
                    _ => unreachable!(),
                };

                let mut args = op
                    .args
                    .iter()
                    .map(|v| translator.load(v))
                    .collect::<Vec<_>>();

                // For some functions we can provide better directly translated
                // implementations. This avoid the need to go through guest->host->guest
                // shim and provides the optimizer more freedom.
                let returns = match (&*class_name.to_lowercase(), &*function_name.to_lowercase()) {
                    // Math Script
                    // Math.abs(float) -> float
                    ("math", "abs") => {
                        let src = args[0];
                        let res = translator.builder.fabs(src);
                        vec![res]
                    }
                    // Math.Floor(float) -> float
                    ("math", "floor") => {
                        let src = args[0];
                        let res = translator.builder.floor(src);
                        vec![res]
                    }
                    // Math.Ceiling(float) -> float
                    ("math", "ceiling") => {
                        let src = args[0];
                        let res = translator.builder.ceil(src);
                        vec![res]
                    }
                    // Math.DegreesToRadians(float) -> float
                    ("math", "degreestoradians") => {
                        // See f32::to_radians
                        const RADS_PER_DEG: f32 = core::f32::consts::PI / 180.0;

                        let lhs = args[0];
                        let rhs = translator.builder.fconst(JitType::F32, RADS_PER_DEG);
                        let res = translator.builder.fmul(lhs, rhs);
                        vec![res]
                    }
                    // Math.RadiansToDegrees(float) -> float
                    ("math", "radianstodegrees") => {
                        // See f32::to_degrees
                        const PIS_IN_180: f32 = 57.2957795130823208767981548141051703_f32;

                        let lhs = args[0];
                        let rhs = translator.builder.fconst(JitType::F32, PIS_IN_180);
                        let res = translator.builder.fmul(lhs, rhs);
                        vec![res]
                    }
                    // Form Script
                    // Not implemented by us.
                    ("form", "startobjectprofiling") => Vec::new(),
                    // Not implemented by us.
                    ("form", "stopobjectprofiling") => Vec::new(),
                    (class_name, function_name) => {
                        let func = data.get_function(class_name, function_name).unwrap();
                        let func = func.func_id.unwrap();

                        args.insert(0, ctx_ptr);

                        translator.builder.call(func, &args)
                    }
                };

                if let Some(res) = returns.get(0).copied() {
                    translator.store(&op.return_assignment, res);
                }
            }
            Instruction::PropertyGet(op) => {
                dbg!(&op);
                todo!();
            }
            Instruction::Return(value) => {
                unterminated_blocks.remove(&current_block);

                let rval = translator.load(value);
                translator.builder.return_(&[rval]);
            }
            Instruction::Cast(op) => {
                let res = perform_cast(&mut translator, &op.src, &op.dst);
                translator.store(&op.dst, res);
            }
            Instruction::Is(op) => {}
            op => todo!("{:?}", op),
        }
    }

    // Sometimes the final return instruction is missing and we must insert it.
    // This is only possible for functions that return "none" and the fact we
    // return "none" in this case is checked by the validation pass.
    for block in unterminated_blocks {
        translator.builder.switch_to_block(block);
        translator.builder.return_(&[]);
    }

    translator.builder.finalize();
}

struct FunctionTranslator<'a> {
    data: &'a CodegenData,
    builder: FunctionBuilder<'a>,
    locals: HashMap<String, Local>,
    fields: HashMap<String, Field>,
    self_ptr: Option<JitValue>,
    self_ty: String,
}

impl<'a> FunctionTranslator<'a> {
    fn load(&mut self, value: &Value) -> JitValue {
        match value {
            Value::Null => self.builder.iconst(JitType::I32, 0),
            Value::Bool(value) => self.builder.iconst(JitType::I32, u8::from(*value)),
            Value::Int(value) => self.builder.iconst(JitType::I32, *value),
            Value::Float(value) => self.builder.fconst(JitType::F32, *value),
            Value::String(value) => todo!(),
            Value::Ident(name) => {
                let name = name.to_lowercase();

                if name == "self" {
                    return self.self_ptr.unwrap();
                }

                if let Some(local) = self.locals.get(&name.to_lowercase()) {
                    return self.builder.stack_load(local.slot, local.ty, 0);
                }

                dbg!(&self.fields);
                if let Some(field) = self.fields.get(&name.to_lowercase()) {
                    // TODO
                    return self.builder.iconst(field.ty, 0);
                }

                panic!("undefined variable {}", name);
            }
        }
    }

    fn store(&mut self, slot: &Value, value: JitValue) {
        let Value::Ident(name) = slot else {
            return;
        };

        if let Some(local) = self.locals.get(&name.to_lowercase()) {
            self.builder.stack_store(local.slot, value, 0);
            return;
        }

        // TODO
    }

    fn get_type(&self, value: &Value) -> Type {
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
    slot: StackSlot,
}

fn perform_cast(translator: &mut FunctionTranslator<'_>, src: &Value, dst: &Value) -> JitValue {
    let src_ty = translator.get_type(&src);
    let dst_ty = translator.get_type(&dst);
    let src = translator.load(&src);

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
        (Type::Float, Type::Bool) => {
            let zero = translator.builder.fconst(JitType::F32, 0.0);
            todo!();
        }
        // The pointers are set to zero if they are semantically none/empty.
        // The pointers already represent valid boolean values as is.
        (Type::String, Type::Bool) => src,
        (Type::Class(_), Type::Bool) => src,
        (Type::Struct(_), Type::Bool) => src,
        (Type::Array(_), Type::Bool) => src,
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
                    todo!();
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
        _ => todo!(),
    }
}

fn perform_typecheck(
    translator: &mut FunctionTranslator<'_>,
    src: &Value,
    dst_class: &str,
    block: Block,
) -> JitValue {
    let src_ty = translator.get_type(src);

    let Type::Class(_) = src_ty else {
        return translator.builder.iconst(JitType::I32, 0);
    };

    let dst_id = translator
        .data
        .classes
        .get(&dst_class.to_lowercase())
        .unwrap()
        .type_id;
    let dst_id = translator.builder.iconst(JitType::I64, dst_id as i64);

    let src = translator.load(src);
    let type_info_ptr = translator.builder.load(JitType::Pointer, src, 0);

    // The first word of an object contains a constant pointer to the type
    // info. The type info starts with a single word containing the number
    // of type ids that follow.
    // If any type id matches we have a valid object of that class.

    let count = translator.builder.load(JitType::Pointer, type_info_ptr, 0);

    // Note that every object has at least one typeid. This means count is
    // always >= 1.
    let iter_block = translator.builder.create_block(&[count]);

    let one = translator.builder.iconst(JitType::I64, 1);
    let next_count = translator.builder.isub(count, one);

    let addr = translator.builder.iadd(type_info_ptr, count);
    let type_id = translator.builder.load(JitType::I64, addr, 0);

    let is_eq = translator
        .builder
        .icmp(type_id, dst_id, IntCompareOp::Equal);

    translator
        .builder
        .brif(count, iter_block, &[next_count], block, &[]);

    is_eq
}
