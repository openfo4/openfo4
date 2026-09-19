use std::mem::offset_of;
use std::ptr;

use openfo4_jit::{IntCompareOp, Type as JitType, Value as JitValue};

use crate::compiler::codegen::hir::lowering::malloc::call_malloc;
use crate::compiler::codegen::hir::lowering::{
    BOOL_TYPE, FLOAT_TYPE, INT_TYPE, ISIZE_TYPE, PTR_TYPE, Translator,
};
use crate::compiler::codegen::hir::{ArrayAdd, ArrayInsert, ArrayRemove, Literal, Null, Value};
use crate::compiler::resources::{Type, TypeId, TypeRepr};
use crate::runtime::ArrayObject;

pub fn array_length(translator: &mut Translator<'_>, array: &Value<Null>, dst: &Value<i32>) {
    let array = translator.load(array);

    translator.assert_true(array);

    // A null array is considered to be of length 0.
    let array_non_null_block = translator.builder.create_block(&[]);
    let final_block = translator.builder.create_block(&[INT_TYPE]);

    let zero = translator.builder.iconst(INT_TYPE, 0);
    translator
        .builder
        .brif(array, array_non_null_block, &[], final_block, &[zero]);

    translator.builder.switch_to_block(array_non_null_block);

    let len = translator
        .builder
        .load(ISIZE_TYPE, array, offset_of!(ArrayObject, len));
    let len = translator.builder.ireduce(len, INT_TYPE);
    translator.builder.jump(final_block, &[len]);

    translator.builder.switch_to_block(final_block);
    translator.current_block = final_block;
    let len = translator.builder.block_params(final_block)[0];

    translator.store(dst, len);
}

pub fn array_get(
    translator: &mut Translator<'_>,
    array: &Value<Null>,
    index: &Value<i32>,
    dst: &Value<Literal>,
) {
    let array = translator.load(array);
    let index = translator.load(index);
    let count = translator.builder.iconst(INT_TYPE, 1);

    emit_array_bounds_check(translator, array, index, count);

    let elem_ty = translator.lowered_ty(&dst);
    let addr = array_addr_at_index(translator, array, index, elem_ty);

    let value = translator.builder.load(elem_ty, addr, 0);
    translator.store(&dst, value);
}

pub fn array_set(
    translator: &mut Translator<'_>,
    array: &Value<Null>,
    index: &Value<i32>,
    value: &Value<Literal>,
) {
    let array = translator.load(array);
    let index = translator.load(index);
    let count = translator.builder.iconst(INT_TYPE, 1);

    emit_array_bounds_check(translator, array, index, count);

    let elem_ty = translator.lowered_ty(value);
    let addr = array_addr_at_index(translator, array, index, elem_ty);

    let value = translator.load(value);
    translator.builder.store(value, addr, 0);
}

pub fn array_create(
    translator: &mut Translator<'_>,
    array: &Value<Null>,
    len: &Value<i32>,
    elem_ty: TypeId,
) {
    let len = translator.load(len);
    let len_wide = translator.builder.uextend(len, ISIZE_TYPE);
    let ptr = call_malloc(translator, elem_ty, len_wide);
    let arr = create_array_object(translator, ptr, len);
    translator.store(array, arr);
}

pub fn array_clear(translator: &mut Translator<'_>, array: &Value<Null>) {
    let array = translator.load(array);

    // Abort if array == null.
    translator.assert_true(array);

    let zero = translator.builder.iconst(ISIZE_TYPE, 0);
    translator
        .builder
        .store(zero, array, offset_of!(ArrayObject, len));
}

pub fn array_add(translator: &mut Translator<'_>, op: &ArrayAdd) {
    let array = translator.load(&op.array);
    let value = translator.load(&op.value);
    let count = translator.load(&op.count);

    // Abort if array == null.
    translator.assert_true(array);

    // Abort if count < 0.
    {
        let is_negative = translator
            .builder
            .icmp_imm(count, 0, IntCompareOp::SignedLessThan);
        translator.assert_false(is_negative);
    }

    let entry_block = translator.builder.create_block(&[]);
    let loop_block = translator.builder.create_block(&[ISIZE_TYPE]);
    let teminate_block = translator.builder.create_block(&[]);
    let exit_block = translator.builder.create_block(&[]);

    // Short-circuit if count == 0,
    // no need to reallocate our array.
    translator
        .builder
        .brif(count, entry_block, &[], exit_block, &[]);
    translator.builder.switch_to_block(entry_block);

    let count_wide = translator.builder.uextend(count, ISIZE_TYPE);

    let ptr = translator
        .builder
        .load(PTR_TYPE, array, offset_of!(ArrayObject, ptr));
    let len = translator
        .builder
        .load(ISIZE_TYPE, array, offset_of!(ArrayObject, len));

    let new_len = translator.builder.iadd(len, count_wide);
    let new_ptr = call_malloc(translator, op.elem_ty, new_len);

    let ty = translator.lowered_ty(&op.value);
    let stride = ty.size();
    let bytes_len = translator.builder.imul_imm(len, stride);
    translator.builder.memcpy(ptr, new_ptr, bytes_len);

    translator.builder.jump(loop_block, &[len]);

    // We have already short-circuit for count == 0, so we can do
    // a do-while loop here.
    {
        translator.builder.switch_to_block(loop_block);
        let index = translator.builder.block_params(loop_block)[0];

        let ty = translator.lowered_ty(&op.value);
        let stride = ty.size();
        let offset = translator.builder.imul_imm(index, stride);
        let addr = translator.builder.ptroffset(new_ptr, offset);

        translator.builder.store(value, addr, 0);

        let next_index = translator.builder.iadd_imm(index, 1);

        let is_end = translator
            .builder
            .icmp(next_index, new_len, IntCompareOp::Equal);
        translator
            .builder
            .brif(is_end, teminate_block, &[], loop_block, &[next_index]);
    }

    {
        translator.builder.switch_to_block(teminate_block);

        translator
            .builder
            .store(new_ptr, array, offset_of!(ArrayObject, ptr));
        translator
            .builder
            .store(new_len, array, offset_of!(ArrayObject, len));

        translator.builder.jump(exit_block, &[]);
    }

    translator.builder.switch_to_block(exit_block);
    translator.current_block = exit_block;
}

pub fn array_insert(translator: &mut Translator<'_>, op: &ArrayInsert) {
    let array = translator.load(&op.array);
    let index = translator.load(&op.index);
    let value = translator.load(&op.value);

    // We allow inserting at the last position of the array, e.g.
    // we can insert into an empty array at index 0.
    let zero = translator.builder.iconst(INT_TYPE, 0);
    emit_array_bounds_check(translator, array, index, zero);

    let old_ptr = translator
        .builder
        .load(PTR_TYPE, array, offset_of!(ArrayObject, ptr));
    let old_len = translator
        .builder
        .load(ISIZE_TYPE, array, offset_of!(ArrayObject, len));

    let new_len = translator.builder.iadd_imm(old_len, 1);
    let new_ptr = call_malloc(translator, op.elem_ty, new_len);

    let index_wide = translator.builder.uextend(index, ISIZE_TYPE);
    let ty = translator.lowered_ty(&op.value);
    let stride = ty.size();

    // Copy left region (all elements 0..index).
    let copy_leading_elems = index_wide;
    let copy_leading_bytes = translator.builder.imul_imm(copy_leading_elems, stride);
    translator
        .builder
        .memcpy(old_ptr, new_ptr, copy_leading_bytes);

    // Copy right region (all elements index..end to index+1..end).
    let copy_trailing_elems = translator.builder.isub(old_len, index_wide);
    let copy_trailing_bytes = translator.builder.imul_imm(copy_trailing_elems, stride);
    let index_plus_one = translator.builder.iadd_imm(index_wide, 1);
    let trailing_src_offset = translator.builder.imul_imm(index_wide, stride);
    let trailing_src_ptr = translator.builder.ptroffset(old_ptr, trailing_src_offset);
    let trailing_dst_offset = translator.builder.imul_imm(index_plus_one, stride);
    let trailing_dst_ptr = translator.builder.ptroffset(new_ptr, trailing_dst_offset);
    translator
        .builder
        .memcpy(trailing_src_ptr, trailing_dst_ptr, copy_trailing_bytes);

    // Insert new element.
    let offset = translator.builder.imul_imm(index_wide, stride);
    let addr = translator.builder.ptroffset(new_ptr, offset);
    translator.builder.store(value, addr, 0);

    // Update ArrayObject.
    translator
        .builder
        .store(new_ptr, array, offset_of!(ArrayObject, ptr));
    translator
        .builder
        .store(new_len, array, offset_of!(ArrayObject, len));
}

pub fn array_remove(translator: &mut Translator<'_>, op: &ArrayRemove) {
    let array = translator.load(&op.array);
    let index = translator.load(&op.index);
    let count = translator.load(&op.count);

    emit_array_bounds_check(translator, array, index, count);

    let exit_block = translator.builder.create_block(&[]);
    let entry_block = translator.builder.create_block(&[]);

    // Short-circuit if count == 0. This allows us to avoid the
    // array reallocation.
    translator
        .builder
        .brif(count, entry_block, &[], exit_block, &[]);

    translator.builder.switch_to_block(entry_block);

    let old_ptr = translator
        .builder
        .load(PTR_TYPE, array, offset_of!(ArrayObject, ptr));
    let old_len = translator
        .builder
        .load(ISIZE_TYPE, array, offset_of!(ArrayObject, len));

    let index_wide = translator.builder.uextend(index, ISIZE_TYPE);
    let count_wide = translator.builder.uextend(count, ISIZE_TYPE);
    let index_plus_count = translator.builder.iadd(index_wide, count_wide);

    let ty = translator.lowered_ty_from_id(op.elem_ty);
    let stride = ty.size();

    // Cannot overflow due to previous bounds check.
    let new_len = translator.builder.isub(old_len, count_wide);
    let new_ptr = call_malloc(translator, op.elem_ty, new_len);

    // Copy left region (0..index).
    let copy_leading_elems = index_wide;
    let copy_leading_bytes = translator.builder.imul_imm(copy_leading_elems, stride);
    translator
        .builder
        .memcpy(old_ptr, new_ptr, copy_leading_bytes);

    // Copy right region (index+count..len to index..len).
    let copy_trailing_elems = translator.builder.isub(old_len, index_plus_count);
    let copy_trailing_bytes = translator.builder.imul_imm(copy_trailing_elems, stride);
    let trailing_src_offset = translator.builder.imul_imm(index_plus_count, stride);
    let trailing_src_ptr = translator.builder.ptroffset(old_ptr, trailing_src_offset);
    let trailing_dst_offset = translator.builder.imul_imm(index_wide, stride);
    let trailing_dst_ptr = translator.builder.ptroffset(new_ptr, trailing_dst_offset);
    translator
        .builder
        .memcpy(trailing_src_ptr, trailing_dst_ptr, copy_trailing_bytes);

    // Update ArrayObject.
    translator
        .builder
        .store(new_ptr, array, offset_of!(ArrayObject, ptr));
    translator
        .builder
        .store(new_len, array, offset_of!(ArrayObject, len));

    translator.builder.jump(exit_block, &[]);

    translator.builder.switch_to_block(exit_block);
    translator.current_block = exit_block;
}

/// Returns the starting address of element `index` from the given `array`.
/// Bounds checking is not included.
///
/// - `array` is of type [`PTR_TYPE`] to an [`ArrayObject`].
/// - `index` is of type [`INT_TYPE`].
/// - `count` is of type [`INT_TYPE`].
fn array_addr_at_index(
    translator: &mut Translator<'_>,
    array: JitValue,
    index: JitValue,
    elem_ty: JitType,
) -> JitValue {
    let ptr = translator
        .builder
        .load(PTR_TYPE, array, offset_of!(ArrayObject, ptr));

    let index = translator.builder.uextend(index, ISIZE_TYPE);
    let offset = translator.builder.imul_imm(index, elem_ty.size());

    let addr = translator.builder.ptroffset(ptr, offset);
    addr
}

/// Inserts an array bounds check and aborts if `index+count` is out of bounds.
///
/// - `array` is of type [`PTR_TYPE`] to an [`ArrayObject`].
/// - `index` is of type [`INT_TYPE`].
/// - `count` is of type [`INT_TYPE`].
fn emit_array_bounds_check(
    translator: &mut Translator<'_>,
    array: JitValue,
    index: JitValue,
    count: JitValue,
) {
    let is_non_null_block = translator.builder.create_block(&[]);

    translator
        .builder
        .brif(array, is_non_null_block, &[], translator.abort_block, &[]);

    translator.builder.switch_to_block(is_non_null_block);

    // Bounds check
    let array_len = translator
        .builder
        .load(ISIZE_TYPE, array, offset_of!(ArrayObject, len));

    // Papyrus integers are i32, but the array len is usize.
    let index_wide = translator.builder.uextend(index, ISIZE_TYPE);
    let count_wide = translator.builder.uextend(count, ISIZE_TYPE);
    let index_plus_count = translator.builder.iadd(index_wide, count_wide);

    // Negative indices are always out of bounds.
    let is_not_negative =
        translator
            .builder
            .icmp_imm(index_plus_count, 0, IntCompareOp::SignedGreaterThanOrEqual);

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
        .brif(is_ok, continue_block, &[], translator.abort_block, &[]);

    translator.builder.switch_to_block(continue_block);
    translator.current_block = continue_block;
}

fn create_array_object(translator: &mut Translator<'_>, ptr: JitValue, len: JitValue) -> JitValue {
    let one = translator.builder.iconst(ISIZE_TYPE, 1);
    let array = call_malloc(translator, translator.data.resources.primitives.array, one);

    let len = translator.builder.uextend(len, ISIZE_TYPE);

    translator
        .builder
        .store(ptr, array, offset_of!(ArrayObject, ptr));
    translator
        .builder
        .store(len, array, offset_of!(ArrayObject, len));

    array
}

pub fn array_cast<F>(
    translator: &mut Translator<'_>,
    src: &Value<Null>,
    dst: &Value<Null>,
    src_ty: TypeId,
    dst_ty: TypeId,
    map_elem: F,
) where
    F: FnOnce(&mut Translator<'_>, JitValue) -> JitValue,
{
    let array = translator.load(src);

    let array_is_null_block = translator.builder.create_block(&[]);
    let array_is_not_null_block = translator.builder.create_block(&[]);
    let final_block = translator.builder.create_block(&[]);

    translator.builder.brif(
        array,
        array_is_not_null_block,
        &[],
        array_is_null_block,
        &[],
    );

    {
        translator.builder.switch_to_block(array_is_null_block);
        let null = translator.builder.ptrconst(ptr::null::<()>());
        translator.store(dst, null);
        translator.builder.jump(final_block, &[]);
    }

    translator.builder.switch_to_block(array_is_not_null_block);

    let src_base_ptr = translator
        .builder
        .load(PTR_TYPE, array, offset_of!(ArrayObject, ptr));
    let len = translator
        .builder
        .load(ISIZE_TYPE, array, offset_of!(ArrayObject, len));

    let dst_base_ptr = call_malloc(translator, dst_ty, len);

    let src_ty = lowered_elem_ty(&translator.data.resources.types.get(src_ty).unwrap());
    let dst_ty = lowered_elem_ty(&translator.data.resources.types.get(dst_ty).unwrap());

    let src_stride = src_ty.size();
    let dst_stride = dst_ty.size();

    let loop_cond_block = translator.builder.create_block(&[ISIZE_TYPE]);
    let loop_body_block = translator.builder.create_block(&[ISIZE_TYPE]);
    let terminator_block = translator.builder.create_block(&[]);

    let zero_usize = translator.builder.iconst(ISIZE_TYPE, 0);
    translator.builder.jump(loop_cond_block, &[zero_usize]);

    {
        translator.builder.switch_to_block(loop_cond_block);
        let index = translator.builder.block_params(loop_cond_block)[0];

        let is_end = translator.builder.icmp(index, len, IntCompareOp::Equal);
        translator
            .builder
            .brif(is_end, terminator_block, &[], loop_body_block, &[index]);
    }

    {
        translator.builder.switch_to_block(loop_body_block);
        translator.current_block = loop_body_block;
        let index = translator.builder.block_params(loop_body_block)[0];

        let src_offset = translator.builder.imul_imm(index, src_stride);
        let src_addr = translator.builder.ptroffset(src_base_ptr, src_offset);
        let src_value = translator.builder.load(src_ty, src_addr, 0);

        let dst_value = map_elem(translator, src_value);

        let dst_offset = translator.builder.imul_imm(index, dst_stride);
        let dst_addr = translator.builder.ptroffset(dst_base_ptr, dst_offset);
        translator.builder.store(dst_value, dst_addr, 0);

        let next_index = translator.builder.iadd_imm(index, 1);
        translator.builder.jump(loop_cond_block, &[next_index]);
    }

    {
        translator.builder.switch_to_block(terminator_block);

        let one = translator.builder.iconst(ISIZE_TYPE, 1);
        let dst_array = call_malloc(translator, translator.data.resources.primitives.array, one);

        translator
            .builder
            .store(dst_base_ptr, dst_array, offset_of!(ArrayObject, ptr));
        translator
            .builder
            .store(len, dst_array, offset_of!(ArrayObject, len));

        translator.store(dst, dst_array);
        translator.builder.jump(final_block, &[]);
    }

    translator.builder.switch_to_block(final_block);
    translator.current_block = final_block;
}

fn lowered_elem_ty(ty: &Type) -> JitType {
    match ty.repr {
        TypeRepr::Unit => PTR_TYPE,
        TypeRepr::Int { bits } if bits == 8 => BOOL_TYPE,
        TypeRepr::Int { bits } if bits == 32 => INT_TYPE,
        TypeRepr::Int { bits } if bits == usize::BITS => PTR_TYPE,
        TypeRepr::Float { bits } if bits == 32 => FLOAT_TYPE,
        TypeRepr::Composite(_) => PTR_TYPE,
        _ => unreachable!(),
    }
}
