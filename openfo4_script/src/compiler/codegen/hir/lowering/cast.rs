use std::mem::offset_of;

use openfo4_jit::{IntCompareOp, Type as JitType, Value as JitValue};

use crate::compiler::codegen::error::ErrorKind;
use crate::compiler::codegen::hir::lowering::malloc::call_malloc;
use crate::compiler::codegen::hir::lowering::{
    BOOL_TYPE, FLOAT_TYPE, INT_TYPE, ISIZE_TYPE, PTR_TYPE, Translator,
};
use crate::compiler::codegen::pex::registry::ClassId;
use crate::compiler::resources::TypeRepr;
use crate::runtime::object::{ObjectTag, VarObject};

/// Returns `src` is of type `class` or a subtype of `class`.
///
/// - `src` must be of type [`PTR_TYPE`] and must point to a class object, or be null.
pub fn is_class(translator: &mut Translator<'_>, src: JitValue, class: ClassId) -> JitValue {
    let zero = translator.builder.iconst(JitType::I8, 0);
    let non_null_block = translator.builder.create_block(&[]);
    let merge_block = translator.builder.create_block(&[JitType::I8]);
    translator
        .builder
        .brif(src, non_null_block, &[], merge_block, &[zero]);

    translator.builder.switch_to_block(non_null_block);
    let is_eq = is_class_non_null(translator, src, class);
    translator.builder.jump(merge_block, &[is_eq]);

    translator.builder.switch_to_block(merge_block);
    translator.current_block = merge_block;
    let is_eq = translator.builder.block_params(merge_block)[0];

    is_eq
}

fn is_class_non_null(translator: &mut Translator<'_>, src: JitValue, class: ClassId) -> JitValue {
    let dst_id = translator.data.layouts.class(class).ty;

    let type_info_ptr = translator.builder.load(PTR_TYPE, src, 0);

    // The first word of an object contains a constant pointer to the type
    // info. The type info starts with a single word containing the number
    // of type ids + vtable pairs that follow.
    // If any type id matches we have a valid object of that class.

    let count = translator.builder.load(ISIZE_TYPE, type_info_ptr, 0);

    let next_block = translator.builder.create_block(&[]);

    // Note that every object has at least one typeid. This means count is
    // always >= 1.
    let iter_block = translator.builder.create_block(&[ISIZE_TYPE]);

    let zero = translator.builder.iconst(ISIZE_TYPE, 0);
    translator.builder.jump(iter_block, &[zero]);

    translator.builder.switch_to_block(iter_block);
    let index = translator.builder.block_params(iter_block)[0];

    // Load and check current offset.
    let stride = size_of::<usize>() * 2;
    let offset = translator.builder.imul_imm(index, stride);
    let offset = translator.builder.iadd_imm(offset, size_of::<usize>());

    let addr = translator.builder.ptroffset(type_info_ptr, offset);
    let type_id = translator.builder.load(ISIZE_TYPE, addr, 0);

    let is_eq = translator
        .builder
        .icmp_imm(type_id, dst_id.as_bits(), IntCompareOp::Equal);

    // Increment index.
    let next_index = translator.builder.iadd_imm(index, 1);
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
    translator.current_block = next_block;

    is_eq
}

pub fn create_var(translator: &mut Translator<'_>, value: JitValue, tag: ObjectTag) -> JitValue {
    // Allocate var object.
    let one = translator.builder.iconst(ISIZE_TYPE, 1);
    let ptr = call_malloc(translator, translator.data.resources.primitives.var, one);

    let tag = translator.builder.iconst(ISIZE_TYPE, tag.to_bits());
    translator
        .builder
        .store(tag, ptr, offset_of!(VarObject, tag));

    translator
        .builder
        .store(value, ptr, offset_of!(VarObject, data));

    ptr
}

pub fn extract_var(
    translator: &mut Translator<'_>,
    object: JitValue,
    tag: ObjectTag,
    class: Option<ClassId>,
) -> JitValue {
    let wanted_tag = tag.to_bits();

    // Abort if object == null.
    translator.assert_true(object, ErrorKind::NullVar);

    let actual_tag = translator
        .builder
        .load(ISIZE_TYPE, object, offset_of!(VarObject, tag));
    let is_eq = translator
        .builder
        .icmp_imm(actual_tag, wanted_tag, IntCompareOp::Equal);

    // Abort if the actual tag doesn't match the requested tag.
    translator.assert_true(is_eq, ErrorKind::WrongVarType);

    let data = match tag {
        ObjectTag::Class => translator
            .builder
            .load(PTR_TYPE, object, offset_of!(VarObject, data)),
        ObjectTag::TypeId(id) => match translator.data.resources.types.get(id).unwrap().repr {
            TypeRepr::Unit => translator.builder.ptrconst(core::ptr::null::<()>()),
            TypeRepr::Int { bits } if bits == 8 => {
                translator
                    .builder
                    .load(BOOL_TYPE, object, offset_of!(VarObject, data))
            }
            TypeRepr::Int { bits } if bits == 32 => {
                translator
                    .builder
                    .load(INT_TYPE, object, offset_of!(VarObject, data))
            }
            TypeRepr::Float { bits } if bits == 32 => {
                translator
                    .builder
                    .load(FLOAT_TYPE, object, offset_of!(VarObject, data))
            }
            TypeRepr::Composite(_) => {
                translator
                    .builder
                    .load(PTR_TYPE, object, offset_of!(VarObject, data))
            }
            _ => unreachable!(),
        },
    };

    if let Some(class) = class {
        let is_class = is_class(translator, data, class);
        translator.assert_true(is_class, ErrorKind::WrongVarType);
    }

    data
}
