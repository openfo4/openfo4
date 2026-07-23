use std::mem::offset_of;

use crate::compiler::codegen::hir::lowering::malloc::call_malloc;
use crate::compiler::codegen::hir::lowering::{INT_TYPE, PTR_TYPE, Translator};
use crate::compiler::codegen::hir::{Binop, Compare, StringCompareOp};
use crate::runtime::ArrayObject;

use openfo4_jit::{IntCompareOp, Value as JitValue};

pub fn string_create(translator: &mut Translator<'_>, s: &str) -> JitValue {
    let ptr = translator.data.resources.constants.alloc(s.as_bytes(), 1);

    let object = ArrayObject {
        ptr: ptr.cast_mut(),
        len: s.len(),
    };

    let object_ptr = translator.data.resources.constants.alloc_object(
        object.as_bytes(),
        align_of::<ArrayObject>(),
        translator.data.resources.primitives.string,
    );

    translator
        .builder
        .iconst(PTR_TYPE, object_ptr.expose_provenance())
}

/// Concats two strings.
pub fn string_concat(translator: &mut Translator<'_>, op: &Binop<String>) {
    let lhs = translator.load(&op.lhs);
    let rhs = translator.load(&op.rhs);

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

    let string = string_from_ptr_len(translator, new_ptr, new_len);

    translator.store(&op.dst, string);
}

pub fn string_cmp(translator: &mut Translator<'_>, op: &Compare<String, StringCompareOp>) {
    let lhs = translator.load(&op.lhs);
    let rhs = translator.load(&op.rhs);
    let is_eq = string_cmpeq(translator, lhs, rhs);

    match op.op {
        StringCompareOp::Equal => {
            translator.store(&op.dst, is_eq);
        }
    }
}

fn string_from_ptr_len(translator: &mut Translator<'_>, ptr: JitValue, len: JitValue) -> JitValue {
    let one = translator.builder.iconst(PTR_TYPE, 1);
    let string_ptr = call_malloc(translator, translator.data.resources.primitives.string, one);

    translator
        .builder
        .store(ptr, string_ptr, offset_of!(ArrayObject, ptr));
    translator
        .builder
        .store(len, string_ptr, offset_of!(ArrayObject, len));

    string_ptr
}

fn load_string_ptr_len(translator: &mut Translator<'_>, object: JitValue) -> (JitValue, JitValue) {
    let ptr = translator
        .builder
        .load(PTR_TYPE, object, offset_of!(ArrayObject, ptr));
    let len = translator
        .builder
        .load(PTR_TYPE, object, offset_of!(ArrayObject, len));
    (ptr, len)
}

/// Returns a truthy value if both lhs and rhs are equal.
pub fn string_cmpeq(translator: &mut Translator<'_>, lhs: JitValue, rhs: JitValue) -> JitValue {
    let (lhs_ptr, lhs_len) = load_string_ptr_len(translator, lhs);
    let (rhs_ptr, rhs_len) = load_string_ptr_len(translator, rhs);

    // FIXME: We can short circuit here; two strings with different
    // lengths are never equal.
    let len_is_eq = translator
        .builder
        .icmp(lhs_len, rhs_len, IntCompareOp::Equal);

    // If lhs_len == rhs_len we compare lhs_len * 1 = lhs_len bytes,
    // otherwise we compare lhs_len * 0 = 0 bytes.
    let len_is_eq_wide = translator.builder.uextend(len_is_eq, PTR_TYPE);
    let bytes_to_memcmp = translator.builder.imul(lhs_len, len_is_eq_wide);

    let bytes_not_eq = translator.builder.memcmp(lhs_ptr, rhs_ptr, bytes_to_memcmp);
    let zero = translator.builder.iconst(INT_TYPE, 0);
    let bytes_eq = translator
        .builder
        .icmp(bytes_not_eq, zero, IntCompareOp::Equal);

    translator.builder.band(len_is_eq, bytes_eq)
}
