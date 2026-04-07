//! String operations

use std::mem::offset_of;

use openfo4_jit::IntCompareOp;
use openfo4_jit::Type as JitType;
use openfo4_jit::Value as JitValue;

use crate::compiler::codegen::function::{FunctionTranslator, call_malloc};
use crate::runtime::ArrayObject;

/// Creates a new constant string.
pub fn string_create(translator: &mut FunctionTranslator<'_>, s: &str) -> JitValue {
    let ptr = translator.data.resources.constants.alloc(s.as_bytes(), 1);
    let ptr = translator.builder.iconst(JitType::Pointer, ptr as i64);
    let len = translator.builder.iconst(JitType::Pointer, s.len() as i64);

    string_from_ptr_len(translator, ptr, len)
}

/// Concats two strings.
pub fn string_concat(
    translator: &mut FunctionTranslator<'_>,
    lhs: JitValue,
    rhs: JitValue,
) -> JitValue {
    let (lhs_ptr, lhs_len) = load_string_ptr_len(translator, lhs);
    let (rhs_ptr, rhs_len) = load_string_ptr_len(translator, rhs);

    let new_len = translator.builder.iadd(lhs_len, rhs_len);
    let new_ptr = call_malloc(
        translator,
        translator.data.resources.primitives.byte,
        new_len,
    );

    translator.builder.memcpy(lhs_ptr, new_ptr, lhs_len);
    let rhs_dst_ptr = translator.builder.iadd(new_ptr, lhs_len);
    translator.builder.memcpy(rhs_ptr, rhs_dst_ptr, rhs_len);

    string_from_ptr_len(translator, new_ptr, new_len)
}

fn string_from_ptr_len(
    translator: &mut FunctionTranslator<'_>,
    ptr: JitValue,
    len: JitValue,
) -> JitValue {
    let one = translator.builder.iconst(JitType::Pointer, 1);
    let string_ptr = call_malloc(translator, translator.data.resources.primitives.string, one);

    translator
        .builder
        .store(ptr, string_ptr, offset_of!(ArrayObject, ptr) as u32);
    translator
        .builder
        .store(len, string_ptr, offset_of!(ArrayObject, len) as u32);

    string_ptr
}

fn load_string_ptr_len(
    translator: &mut FunctionTranslator<'_>,
    object: JitValue,
) -> (JitValue, JitValue) {
    let ptr = translator.builder.load(
        JitType::Pointer,
        object,
        offset_of!(ArrayObject, ptr) as u32,
    );
    let len = translator.builder.load(
        JitType::Pointer,
        object,
        offset_of!(ArrayObject, len) as u32,
    );
    (ptr, len)
}

/// Returns a truthy value if both lhs and rhs are equal.
pub fn string_cmpeq(
    translator: &mut FunctionTranslator<'_>,
    lhs: JitValue,
    rhs: JitValue,
) -> JitValue {
    let (lhs_ptr, lhs_len) = load_string_ptr_len(translator, lhs);
    let (rhs_ptr, rhs_len) = load_string_ptr_len(translator, rhs);

    // FIXME: We can short circuit here; two strings with different
    // lengths are never equal.
    let len_is_eq = translator
        .builder
        .icmp(lhs_len, rhs_len, IntCompareOp::Equal);

    // If lhs_len == rhs_len we compare lhs_len * 1 = lhs_len bytes,
    // otherwise we compare lhs_len * 0 = 0 bytes.
    let len_is_eq_wide = translator.builder.uextend(len_is_eq, JitType::Pointer);
    let bytes_to_memcmp = translator.builder.imul(lhs_len, len_is_eq_wide);

    let bytes_not_eq = translator.builder.memcmp(lhs_ptr, rhs_ptr, bytes_to_memcmp);
    let zero = translator.builder.iconst(JitType::I32, 0);
    let bytes_eq = translator
        .builder
        .icmp(bytes_not_eq, zero, IntCompareOp::Equal);

    translator.builder.band(len_is_eq, bytes_eq)
}
