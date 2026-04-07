//! String operations

use std::mem::offset_of;

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
