mod array;
mod call;
mod cast;
mod malloc;
mod stack;
mod string;

use std::mem::offset_of;
use std::num::NonZeroUsize;

use hashbrown::HashMap;
use openfo4_jit::{
    Block, FloatCompareOp, FunctionBuilder, IntCompareOp, SourceLoc, Type as JitType,
    Value as JitValue, Variable,
};

use crate::compiler::codegen::CodegenData;
use crate::compiler::codegen::hir::lowering::stack::{InstructionPosition, SuspensionPoint};
use crate::compiler::codegen::hir::{
    AddrCompareOp, Function, Instruction, Literal, Null, ScalarCompareOp, Value,
};
use crate::compiler::codegen::trace::Span;
use crate::compiler::resources::{TypeId, TypeRepr};
use crate::runtime::object::ObjectTag;
use crate::runtime::{RESULT_ERROR, RESULT_OK, RawContext, RawStack};

const BOOL_TYPE: JitType = JitType::I8;
const INT_TYPE: JitType = JitType::I32;
const FLOAT_TYPE: JitType = JitType::F32;
const PTR_TYPE: JitType = JitType::Pointer;
const ISIZE_TYPE: JitType = JitType::Isize;
const RESULT_TYPE: JitType = JitType::I32;

pub fn lower_function(
    function: &Function,
    data: &CodegenData,
    mut builder: FunctionBuilder<'_>,
    mut span: Span,
) {
    let entry_block = builder.create_entry_block();
    builder.switch_to_block(entry_block);
    let ctx_ptr = builder.block_params(entry_block)[0];

    span.enter(data, &mut builder, ctx_ptr);

    let mut variables = Vec::new();
    for ty in function.variables.iter() {
        let ty = data.resources.types.get(*ty).unwrap();
        let ty = match ty.repr {
            TypeRepr::Unit => PTR_TYPE,
            TypeRepr::Int { bits } if bits == 8 => BOOL_TYPE,
            TypeRepr::Int { bits } if bits == 32 => INT_TYPE,
            TypeRepr::Float { bits } if bits == 32 => FLOAT_TYPE,
            TypeRepr::Composite(_) => PTR_TYPE,
            _ => unreachable!(),
        };

        let default_value = match ty {
            JitType::I8 | JitType::I16 | JitType::I32 | JitType::I64 | JitType::Isize => {
                builder.iconst(ty, 0)
            }
            JitType::Pointer => builder.ptrconst(core::ptr::null::<()>()),
            JitType::F32 | JitType::F64 => builder.fconst(ty, 0.0f32),
        };

        let var = builder.def_var(ty, default_value);
        variables.push(Local { var, ty });
    }

    let func_args = &builder.block_params(entry_block)[1..].to_vec();
    for (var, value) in function.params.iter().zip(func_args) {
        let local = variables.get(*var).unwrap();
        builder.set_var(local.var, *value);
    }

    let return_block = if let Some(ret_ty) = function.returns {
        let ty = data.resources.types.get(ret_ty).unwrap();
        let ty = match ty.repr {
            TypeRepr::Unit => PTR_TYPE,
            TypeRepr::Int { bits } if bits == 8 => BOOL_TYPE,
            TypeRepr::Int { bits } if bits == 32 => INT_TYPE,
            TypeRepr::Float { bits } if bits == 32 => FLOAT_TYPE,
            TypeRepr::Composite(_) => PTR_TYPE,
            _ => unreachable!(),
        };
        builder.create_block(&[JitType::I32, ty])
    } else {
        builder.create_block(&[JitType::I32])
    };

    let abort_block = builder.create_block(&[]);

    // The entry block needs to be special cased.
    let blocks: Vec<_> = (0..function.blocks.len().saturating_sub(1))
        .map(|_| builder.create_block(&[]))
        .collect();

    let stack_frame = stack::compute_suspension_points(&mut builder, function, &variables);

    let stack_start_offset = builder.load(
        ISIZE_TYPE,
        ctx_ptr,
        (offset_of!(RawContext, stack_ptr) + offset_of!(RawStack, len)),
    );

    let mut translator = Translator {
        builder,
        locals: variables,
        return_block,
        abort_block,
        current_block: entry_block,
        data,
        ctx_ptr,
        stack_start_offset,
        span,
        suspend_blocks: HashMap::new(),
    };

    // Resume from stack.
    let start_block = translator.builder.create_block(&[]);
    stack::resume(&mut translator, &stack_frame, start_block);
    stack::generate_suspension_blocks(&mut translator, function, &stack_frame);

    translator.builder.switch_to_block(start_block);

    // This is first time we are being called.
    // Allocate enough stack space in case we have to suspend.
    // The caller already guarantees that we there is enough
    // free space for us to store our `usize` tag.
    translator.stack_alloc(stack_frame.size.saturating_sub(size_of::<usize>()));

    let mut next_srcloc = 0;

    for (block_index, block) in function.blocks.iter().enumerate() {
        if block_index != 0 {
            let block = blocks[block_index - 1];
            translator.builder.switch_to_block(block);
        }

        for (inst_index, instruction) in block.iter().enumerate() {
            let srcloc = SourceLoc::new(next_srcloc);
            next_srcloc += 1;
            translator.builder.begin_srcloc(srcloc);

            match instruction {
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
                Instruction::BNot(op) => {
                    let src = translator.load(&op.src);

                    // Invert the boolean value:
                    // 1 != 1 => 0
                    // 0 != 1 => 1
                    let res = translator.builder.icmp_imm(src, 1, IntCompareOp::NotEqual);

                    translator.store(&op.dst, res);
                }
                Instruction::Store { src, dst } => {
                    let value = translator.load(src);
                    translator.store(dst, value);
                }
                Instruction::CompareBool(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let mode = match op.op {
                        ScalarCompareOp::Equal => IntCompareOp::Equal,
                        ScalarCompareOp::LessThan => IntCompareOp::SignedLessThan,
                        ScalarCompareOp::LessThanOrEqual => IntCompareOp::SignedLessThanOrEqual,
                        ScalarCompareOp::GreaterThan => IntCompareOp::SignedGreaterThan,
                        ScalarCompareOp::GreaterThanOrEqual => {
                            IntCompareOp::SignedGreaterThanOrEqual
                        }
                    };

                    let res = translator.builder.icmp(lhs, rhs, mode);
                    translator.store(&op.dst, res);
                }
                Instruction::CompareInt(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let mode = match op.op {
                        ScalarCompareOp::Equal => IntCompareOp::Equal,
                        ScalarCompareOp::LessThan => IntCompareOp::SignedLessThan,
                        ScalarCompareOp::LessThanOrEqual => IntCompareOp::SignedLessThanOrEqual,
                        ScalarCompareOp::GreaterThan => IntCompareOp::SignedGreaterThan,
                        ScalarCompareOp::GreaterThanOrEqual => {
                            IntCompareOp::SignedGreaterThanOrEqual
                        }
                    };

                    let res = translator.builder.icmp(lhs, rhs, mode);
                    translator.store(&op.dst, res);
                }
                Instruction::CompareFloat(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let mode = match op.op {
                        ScalarCompareOp::Equal => FloatCompareOp::Equal,
                        ScalarCompareOp::LessThan => FloatCompareOp::LessThan,
                        ScalarCompareOp::LessThanOrEqual => FloatCompareOp::LessThanOrEqual,
                        ScalarCompareOp::GreaterThan => FloatCompareOp::GreaterThan,
                        ScalarCompareOp::GreaterThanOrEqual => FloatCompareOp::GreaterThanOrEqual,
                    };

                    let res = translator.builder.fcmp(lhs, rhs, mode);
                    translator.store(&op.dst, res);
                }
                Instruction::CompareString(op) => {
                    string::string_cmp(&mut translator, op);
                }
                Instruction::CompareAddr(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let mode = match op.op {
                        AddrCompareOp::Equal => IntCompareOp::Equal,
                        AddrCompareOp::NotEqual => IntCompareOp::NotEqual,
                    };

                    let res = translator.builder.icmp(lhs, rhs, mode);
                    translator.store(&op.dst, res);
                }
                Instruction::StringConcat(op) => {
                    string::string_concat(&mut translator, op);
                }
                Instruction::Return(value) => {
                    let result = translator.builder.iconst(RESULT_TYPE, RESULT_OK);

                    // Release our stack.
                    // The `usize` tag is released by our caller.
                    translator.stack_free(stack_frame.size.saturating_sub(size_of::<usize>()));

                    if function.returns.is_some() {
                        let value = translator.load(value);
                        translator
                            .builder
                            .jump(translator.return_block, &[result, value]);
                    } else {
                        translator.builder.jump(translator.return_block, &[result]);
                    }
                }
                Instruction::Jump { block } => {
                    let block = blocks[block.0 - 1];
                    translator.builder.jump(block, &[]);
                }
                Instruction::Brif {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let condition = translator.load(condition);

                    let true_block = blocks[then_block.0 - 1];
                    let false_block = blocks[else_block.0 - 1];

                    translator
                        .builder
                        .brif(condition, true_block, &[], false_block, &[]);
                }
                Instruction::Call(op) => {
                    let callee = &translator.data.registry.function(op.callee).unwrap();
                    let point = stack_frame
                        .call_points
                        .get(&InstructionPosition {
                            block: block_index,
                            instruction: inst_index,
                        })
                        .unwrap();

                    call::perform_checked_call(
                        &mut translator,
                        callee,
                        &op.args,
                        Some(&op.dst),
                        None,
                        point,
                        &stack_frame,
                    );
                }
                Instruction::CallIndirect(op) => {
                    let callee = &translator.data.registry.function(op.callee).unwrap();
                    let point = stack_frame
                        .call_points
                        .get(&InstructionPosition {
                            block: block_index,
                            instruction: inst_index,
                        })
                        .unwrap();

                    call::perform_checked_call(
                        &mut translator,
                        callee,
                        &op.args,
                        Some(&op.dst),
                        op.object.as_ref(),
                        point,
                        &stack_frame,
                    );
                }
                Instruction::StructCreate { dst, ty } => {
                    let one = translator.builder.iconst(ISIZE_TYPE, 1);

                    // Allocation will already be zeroed, which is the default
                    // value for all types.
                    let ptr = malloc::call_malloc(&mut translator, *ty, one);

                    translator.store(dst, ptr);
                }
                Instruction::StructLoad {
                    st,
                    dst,
                    struct_id,
                    field_name,
                } => {
                    let offset = *translator
                        .data
                        .struct_field_offsets
                        .get(struct_id)
                        .unwrap()
                        .get(field_name)
                        .unwrap();

                    let st = translator.load(st);
                    let ty = translator.lowered_ty(dst);

                    // Abort if struct == null
                    translator.assert_true(st);

                    let res = translator.builder.load(ty, st, offset);
                    translator.store(dst, res);
                }
                Instruction::StructStore {
                    st,
                    value,
                    struct_id,
                    field_name,
                } => {
                    let offset = *translator
                        .data
                        .struct_field_offsets
                        .get(struct_id)
                        .unwrap()
                        .get(field_name)
                        .unwrap();

                    let st = translator.load(st);
                    let value = translator.load(value);

                    // Abort if struct == null
                    translator.assert_true(st);

                    translator.builder.store(value, st, offset);
                }
                Instruction::ClassLoad {
                    object,
                    dst,
                    class_id,
                    field_name,
                } => {
                    let object = translator.load(object);
                    let class = translator.data.registry.class(*class_id).unwrap();
                    let offset = translator
                        .data
                        .class_field_offsets
                        .get(class_id)
                        .unwrap()
                        .get(field_name)
                        .unwrap();

                    let ty = class.fields.get(field_name).unwrap().ty.to_jit();

                    // Abort if object == null.
                    translator.assert_true(object);

                    let value = translator.builder.load(ty, object, *offset);
                    translator.store(dst, value);
                }
                Instruction::ClassStore {
                    object,
                    value,
                    class_id,
                    field_name,
                } => {
                    let object = translator.load(object);
                    let value = translator.load(value);
                    let offset = translator
                        .data
                        .class_field_offsets
                        .get(class_id)
                        .unwrap()
                        .get(field_name)
                        .unwrap();

                    // Abort if object == null.
                    translator.assert_true(object);

                    translator.builder.store(value, object, *offset);
                }
                Instruction::ClassUpcast {
                    src,
                    dst,
                    src_class: _,
                    dst_class: _,
                } => {
                    let addr = translator.load(src);
                    translator.store(dst, addr);
                }
                Instruction::ClassDowncast {
                    src,
                    dst,
                    src_class: _,
                    dst_class,
                } => {
                    let src = translator.load(src);

                    let next_block = translator.builder.create_block(&[JitType::Pointer]);
                    let null = translator.builder.ptrconst(core::ptr::null::<()>());

                    let is_eq = cast::is_class(&mut translator, src, *dst_class);
                    translator
                        .builder
                        .brif(is_eq, next_block, &[src], next_block, &[null]);

                    translator.builder.switch_to_block(next_block);
                    let res = translator.builder.block_params(next_block)[0];
                    translator.current_block = next_block;

                    translator.store(dst, res);
                }
                Instruction::ClassIsInstanceOf { src, dst, class } => {
                    let src = translator.load(src);
                    let is_eq = cast::is_class(&mut translator, src, *class);
                    translator.store(dst, is_eq);
                }
                Instruction::ArrayCreate {
                    array,
                    len,
                    elem_ty,
                } => {
                    array::array_create(&mut translator, array, len, *elem_ty);
                }
                Instruction::ArrayLength { array, dst } => {
                    array::array_length(&mut translator, array, dst);
                }
                Instruction::ArrayGet { array, index, dst } => {
                    array::array_get(&mut translator, array, index, dst);
                }
                Instruction::ArraySet {
                    array,
                    value,
                    index,
                } => {
                    array::array_set(&mut translator, array, index, value);
                }
                Instruction::ArrayClear { array } => {
                    array::array_clear(&mut translator, array);
                }
                Instruction::ArrayAdd(op) => {
                    array::array_add(&mut translator, op);
                }
                Instruction::ArrayInsert(op) => {
                    array::array_insert(&mut translator, op);
                }
                Instruction::ArrayRemove(op) => {
                    array::array_remove(&mut translator, op);
                }
                Instruction::CreateVar {
                    src,
                    dst,
                    tag,
                    cast_array,
                } => {
                    if *cast_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let src_ty = match tag {
                            ObjectTag::Class => translator.data.resources.primitives.pointer,
                            ObjectTag::TypeId(id) => *id,
                        };
                        let dst_ty = translator.data.resources.primitives.var;

                        array::array_cast(
                            &mut translator,
                            &src,
                            dst,
                            src_ty,
                            dst_ty,
                            |translator, value| cast::create_var(translator, value, *tag),
                        );
                    } else {
                        let src = translator.load(src);
                        let res = cast::create_var(&mut translator, src, *tag);
                        translator.store(dst, res);
                    }
                }
                Instruction::ExtractVar {
                    src,
                    dst,
                    tag,
                    class,
                    cast_array,
                } => {
                    if *cast_array {
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.var;
                        let dst_ty = match tag {
                            ObjectTag::Class => translator.data.resources.primitives.pointer,
                            ObjectTag::TypeId(id) => *id,
                        };

                        array::array_cast(
                            &mut translator,
                            src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| cast::extract_var(translator, value, *tag, *class),
                        );
                    } else {
                        let src = translator.load(src);
                        let res = cast::extract_var(&mut translator, src, *tag, *class);
                        translator.store(dst, res);
                    }
                }
                Instruction::BoolToString { src, dst } => {
                    let src = translator.load(src);
                    let res = translator.builder.call(
                        translator.data.runtime_funcs.bool_to_string,
                        &[translator.ctx_ptr, src],
                    )[0];
                    translator.store(dst, res);
                }
                Instruction::IntToString { src, dst } => {
                    let src = translator.load(src);
                    let res = translator.builder.call(
                        translator.data.runtime_funcs.i32_to_string,
                        &[translator.ctx_ptr, src],
                    )[0];
                    translator.store(dst, res);
                }
                Instruction::FloatToString { src, dst } => {
                    let src = translator.load(src);
                    let res = translator.builder.call(
                        translator.data.runtime_funcs.f32_to_string,
                        &[translator.ctx_ptr, src],
                    )[0];
                    translator.store(dst, res);
                }
                Instruction::ObjectToString {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.pointer;
                        let dst_ty = translator.data.resources.primitives.string;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| {
                                translator.builder.call(
                                    translator.data.runtime_funcs.object_to_string,
                                    &[translator.ctx_ptr, value],
                                )[0]
                            },
                        );
                    } else {
                        let src = translator.load(src);
                        let res = translator.builder.call(
                            translator.data.runtime_funcs.object_to_string,
                            &[translator.ctx_ptr, src],
                        )[0];
                        translator.store(dst, res);
                    }
                }
                Instruction::ConvertBoolToInt {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.bool;
                        let dst_ty = translator.data.resources.primitives.int;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| translator.builder.uextend(value, INT_TYPE),
                        );
                    } else {
                        let src = translator.load(src);
                        let res = translator.builder.uextend(src, INT_TYPE);
                        translator.store(dst, res);
                    }
                }
                Instruction::ConvertBoolToFloat {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.bool;
                        let dst_ty = translator.data.resources.primitives.int;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| {
                                translator.builder.convert_sint_to_float(value, FLOAT_TYPE)
                            },
                        );
                    } else {
                        let src = translator.load(src);
                        let res = translator.builder.convert_sint_to_float(src, FLOAT_TYPE);
                        translator.store(dst, res);
                    }
                }
                Instruction::ConvertIntToBool {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.bool;
                        let dst_ty = translator.data.resources.primitives.int;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| {
                                translator
                                    .builder
                                    .icmp_imm(value, 0, IntCompareOp::NotEqual)
                            },
                        );
                    } else {
                        let src = translator.load(src);
                        let res = translator.builder.icmp_imm(src, 0, IntCompareOp::NotEqual);
                        translator.store(dst, res);
                    }
                }
                Instruction::ConvertIntToFloat {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.int;
                        let dst_ty = translator.data.resources.primitives.float;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| {
                                translator.builder.convert_sint_to_float(value, FLOAT_TYPE)
                            },
                        );
                    } else {
                        let src = translator.load(src);
                        let res = translator.builder.convert_sint_to_float(src, FLOAT_TYPE);
                        translator.store(dst, res);
                    }
                }
                Instruction::ConvertFloatToBool {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.int;
                        let dst_ty = translator.data.resources.primitives.float;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| {
                                let zero = translator.builder.fconst(FLOAT_TYPE, 0.0f32);
                                translator
                                    .builder
                                    .fcmp(value, zero, FloatCompareOp::NotEqual)
                            },
                        );
                    } else {
                        let src = translator.load(src);
                        let zero = translator.builder.fconst(FLOAT_TYPE, 0.0f32);
                        let res = translator.builder.fcmp(src, zero, FloatCompareOp::NotEqual);
                        translator.store(dst, res);
                    }
                }
                Instruction::ConvertFloatToInt {
                    src,
                    dst,
                    conv_array,
                } => {
                    if *conv_array {
                        let src = src.clone().map(|v| v.to_null().unwrap());
                        let dst = dst.clone().map(|v| v.to_null().unwrap());
                        let src_ty = translator.data.resources.primitives.int;
                        let dst_ty = translator.data.resources.primitives.float;

                        array::array_cast(
                            &mut translator,
                            &src,
                            &dst,
                            src_ty,
                            dst_ty,
                            |translator, value| {
                                translator.builder.convert_float_to_sint(value, INT_TYPE)
                            },
                        );
                    } else {
                        let src = translator.load(src);
                        let res = translator.builder.convert_float_to_sint(src, INT_TYPE);
                        translator.store(dst, res);
                    }
                }
            }
        }

        translator.builder.end_srcloc();
    }

    generate_return_block(
        function,
        data,
        &mut translator.builder,
        &mut translator.span,
        ctx_ptr,
        translator.return_block,
    );
    generate_abort_block(
        function,
        data,
        &mut translator.builder,
        translator.abort_block,
        translator.return_block,
        ctx_ptr,
    );

    translator.builder.finalize();
}

struct Translator<'a> {
    builder: FunctionBuilder<'a>,
    data: &'a CodegenData,
    locals: Vec<Local>,
    abort_block: Block,
    return_block: Block,
    current_block: Block,
    ctx_ptr: JitValue,
    /// The starting offset where the stack for our current function starts.
    stack_start_offset: JitValue,
    span: Span,
    suspend_blocks: HashMap<NonZeroUsize, Block>,
}

impl<'a> Translator<'a> {
    pub fn load<T>(&mut self, value: &Value<T>) -> JitValue
    where
        T: LowerLiteral,
    {
        match value {
            Value::Const(val) => val.lower_literal(self),
            Value::Variable(var) => {
                let local = *self.locals.get(var.0).unwrap();
                self.builder.get_var(local.var)
            }
            Value::ClassVar {
                object,
                class,
                field,
            } => {
                let object = self.locals.get(object.0).unwrap();
                let object = self.builder.get_var(object.var);

                // Abort if object == null.
                self.assert_true(object);

                let ty = self.lowered_ty(value);

                let offset = self
                    .data
                    .class_field_offsets
                    .get(class)
                    .unwrap()
                    .get(field)
                    .unwrap();

                self.builder.load(ty, object, *offset)
            }
        }
    }

    pub fn store<T>(&mut self, slot: &Value<T>, value: JitValue)
    where
        T: LowerLiteral,
    {
        let Value::Variable(var) = slot else {
            return;
        };

        let local = *self.locals.get(var.0).unwrap();
        self.builder.set_var(local.var, value);
    }

    /// Returns the lowered type of a [`Value`].
    pub fn lowered_ty<T>(&mut self, value: &Value<T>) -> JitType
    where
        T: LowerLiteral,
    {
        match value {
            Value::Const(v) => v.ty(),
            Value::Variable(var) => {
                let local = self.locals.get(var.0).unwrap();
                local.ty
            }
            Value::ClassVar {
                object: _,
                class,
                field,
            } => {
                let class = self.data.registry.class(*class).unwrap();
                let field = class.fields.get(field).unwrap();
                field.ty.to_jit()
            }
        }
    }

    pub fn lowered_ty_from_id(&mut self, ty: TypeId) -> JitType {
        let ty = self.data.resources.types.get(ty).unwrap();
        match ty.repr {
            TypeRepr::Unit => PTR_TYPE,
            TypeRepr::Int { bits } if bits == 8 => BOOL_TYPE,
            TypeRepr::Int { bits } if bits == 32 => INT_TYPE,
            TypeRepr::Float { bits } if bits == 32 => FLOAT_TYPE,
            TypeRepr::Composite(_) => PTR_TYPE,
            _ => unreachable!(),
        }
    }

    pub fn assert_true(&mut self, condition: JitValue) {
        let block = self.builder.create_block(&[]);
        self.builder
            .brif(condition, block, &[], self.abort_block, &[]);
        self.current_block = block;
        self.builder.switch_to_block(block);
    }

    pub fn assert_false(&mut self, condition: JitValue) {
        let block = self.builder.create_block(&[]);
        self.builder
            .brif(condition, self.abort_block, &[], block, &[]);
        self.current_block = block;
        self.builder.switch_to_block(block);
    }

    /// Allocates `size` bytes on the stack.
    ///
    /// The initial contents of the allocated bytes are undefined.
    ///
    /// Note that this function MAY invalidate the current stack base and stack pointer.
    ///
    /// [`stack_ptr`]: Self::stack_ptr
    pub fn stack_alloc(&mut self, size: usize) {
        if size == 0 {
            return;
        }

        let size = self.builder.iconst(ISIZE_TYPE, size);
        let stack_base = self.builder.call(
            self.data.runtime_funcs.stack_frame_alloc,
            &[self.ctx_ptr, size],
        )[0];

        self.builder.store(
            stack_base,
            self.ctx_ptr,
            (offset_of!(RawContext, stack_ptr) + offset_of!(RawStack, ptr)),
        );
    }

    /// Frees the top `size` bytes from the stack.
    ///
    /// Note that this function MAY invalidate the current stack base and stack pointer.
    ///
    /// [`stack_ptr`]: Self::stack_ptr
    pub fn stack_free(&mut self, size: usize) {
        if size == 0 {
            return;
        }

        let size = self.builder.iconst(ISIZE_TYPE, size);
        let stack_base = self.builder.call(
            self.data.runtime_funcs.stack_frame_free,
            &[self.ctx_ptr, size],
        )[0];

        self.builder.store(
            stack_base,
            self.ctx_ptr,
            (offset_of!(RawContext, stack_ptr) + offset_of!(RawStack, ptr)),
        );
    }

    /// Loads the current stack base pointer from memory.
    pub fn load_stack_base(&mut self) -> JitValue {
        self.builder.load(
            PTR_TYPE,
            self.ctx_ptr,
            (offset_of!(RawContext, stack_ptr) + offset_of!(RawStack, ptr)),
        )
    }

    pub fn load_stack_ptr(&mut self) -> JitValue {
        let base = self.load_stack_base();
        self.builder.ptroffset(base, self.stack_start_offset)
    }

    pub fn suspend(&mut self, point: &SuspensionPoint, stack_ptr: JitValue, result: JitValue) {
        let block = self.suspend_blocks.get(&point.tag).unwrap();
        self.builder.jump(*block, &[stack_ptr, result]);
    }
}

trait LowerLiteral {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue;

    fn ty(&self) -> JitType;
}

impl LowerLiteral for bool {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue {
        translator.builder.iconst(BOOL_TYPE, u8::from(*self))
    }

    fn ty(&self) -> JitType {
        BOOL_TYPE
    }
}

impl LowerLiteral for i32 {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue {
        translator.builder.iconst(INT_TYPE, *self)
    }

    fn ty(&self) -> JitType {
        INT_TYPE
    }
}

impl LowerLiteral for f32 {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue {
        translator.builder.fconst(FLOAT_TYPE, *self)
    }

    fn ty(&self) -> JitType {
        FLOAT_TYPE
    }
}

impl LowerLiteral for Null {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue {
        translator.builder.ptrconst(core::ptr::null::<()>())
    }

    fn ty(&self) -> JitType {
        PTR_TYPE
    }
}

impl LowerLiteral for String {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue {
        string::string_create(translator, self)
    }

    fn ty(&self) -> JitType {
        PTR_TYPE
    }
}

impl LowerLiteral for Literal {
    fn lower_literal(&self, translator: &mut Translator<'_>) -> JitValue {
        match self {
            Self::Null => translator.builder.ptrconst(core::ptr::null::<()>()),
            Self::Bool(v) => translator.builder.iconst(BOOL_TYPE, u8::from(*v)),
            Self::Int(v) => translator.builder.iconst(INT_TYPE, *v),
            Self::Float(v) => translator.builder.fconst(FLOAT_TYPE, *v),
            Self::String(v) => string::string_create(translator, v),
        }
    }

    fn ty(&self) -> JitType {
        match self {
            Self::Null => PTR_TYPE,
            Self::Bool(_) => BOOL_TYPE,
            Self::Int(_) => INT_TYPE,
            Self::Float(_) => FLOAT_TYPE,
            Self::String(_) => PTR_TYPE,
        }
    }
}

fn generate_return_block(
    function: &Function,
    data: &CodegenData,
    builder: &mut FunctionBuilder<'_>,
    span: &mut Span,
    ctx_ptr: JitValue,
    block: Block,
) {
    builder.switch_to_block(block);

    span.exit(data, builder, ctx_ptr);

    if function.returns.is_some() {
        let result = builder.block_params(block)[0];
        let value = builder.block_params(block)[1];

        builder.return_(&[result, value]);
    } else {
        let result = builder.block_params(block)[0];
        builder.return_(&[result]);
    }
}

fn generate_abort_block(
    function: &Function,
    data: &CodegenData,
    builder: &mut FunctionBuilder<'_>,
    block: Block,
    return_block: Block,
    ctx_ptr: JitValue,
) {
    builder.switch_to_block(block);

    let result = builder.iconst(RESULT_TYPE, RESULT_ERROR);

    builder.call(data.runtime_funcs.emit_error, &[ctx_ptr]);

    if let Some(ty) = function.returns {
        let ty = data.resources.types.get(ty).unwrap();
        let ty = match ty.repr {
            TypeRepr::Unit => PTR_TYPE,
            TypeRepr::Int { bits } if bits == 8 => BOOL_TYPE,
            TypeRepr::Int { bits } if bits == 32 => INT_TYPE,
            TypeRepr::Float { bits } if bits == 32 => FLOAT_TYPE,
            TypeRepr::Composite(_) => PTR_TYPE,
            _ => unreachable!(),
        };

        let value = builder.uninit(ty);
        builder.jump(return_block, &[result, value]);
    } else {
        builder.jump(return_block, &[result]);
    }
}

#[derive(Copy, Clone, Debug)]
struct Local {
    var: Variable,
    ty: JitType,
}
