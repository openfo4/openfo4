use game_common::string::Lowercase;
use openfo4_pex::{ArrayFind, ArrayFindStruct, Unop};

use crate::compiler::codegen::function::FunctionTranslator;
use crate::compiler::codegen::hir::{
    AddrCompareOp, Literal, Null, ScalarCompareOp, StringCompareOp, Value, Variable,
};
use crate::compiler::codegen::pex::registry::Type;
use crate::compiler::resources::TypeId;

pub fn lower_array_len(translator: &mut FunctionTranslator<'_>, op: &Unop) {
    let array = translator.load(&op.src);
    let dst = translator.load(&op.dst);

    let non_null_block = translator.builder.create_block();
    let merge_block = translator.builder.create_block();

    let is_null = translator
        .builder
        .declare_variable(translator.ty(&Type::Bool));
    translator
        .builder
        .store(Value::Const(Literal::Int(0)), dst.clone());
    translator.builder.compare_addr(
        array.clone(),
        Value::Const(Null),
        Value::Variable(is_null),
        AddrCompareOp::Equal,
    );
    translator
        .builder
        .brif_true(Value::Variable(is_null), merge_block, non_null_block);

    translator.builder.switch_to_block(non_null_block);
    translator
        .builder
        .array_length(array, dst.map(|lit| lit.to_int().unwrap()));
    translator.builder.jump(merge_block);

    translator.builder.switch_to_block(merge_block);
}

pub fn lower_array_find(translator: &mut FunctionTranslator<'_>, op: &ArrayFind, reverse: bool) {
    array_find_inner(translator, op, reverse, |translator, array, index, ty| {
        let value = translator.builder.declare_variable(translator.ty(ty));
        translator
            .builder
            .array_get(array, index, Value::Variable(value));
        Value::Variable(value)
    });
}

pub fn lower_array_struct(
    translator: &mut FunctionTranslator<'_>,
    op: &ArrayFindStruct,
    reverse: bool,
) {
    array_find_inner(
        translator,
        &ArrayFind {
            array_value: op.array_value.clone(),
            return_index: op.return_index.clone(),
            value: op.value.clone(),
            start_index: op.start_index.clone(),
        },
        reverse,
        |translator, array, index, ty| {
            let Type::Struct(id) = ty else { unreachable!() };

            let struct_ptr = translator.builder.declare_variable(translator.ty(ty));
            translator
                .builder
                .array_get(array, index, Value::Variable(struct_ptr));

            let value = translator
                .builder
                .declare_variable(translator.ty_of_val(&op.value));
            translator.builder.struct_load(
                Value::Variable(struct_ptr),
                Value::Variable(value),
                *id,
                Lowercase::new(&*op.member_name),
            );

            Value::Variable(value)
        },
    );
}

fn array_find_inner<F>(
    translator: &mut FunctionTranslator<'_>,
    op: &ArrayFind,
    reverse: bool,
    get_elem: F,
) where
    F: FnOnce(
        &mut FunctionTranslator<'_>,
        /* array: */ Value<Null>,
        /* index: */ Value<i32>,
        /* elem_ty: */ &Type,
    ) -> Value<Literal>,
{
    if reverse {
        todo!();
    }

    let int_ty = translator.ty(&Type::Int);
    let bool_ty = translator.ty(&Type::Bool);

    let Type::Array(elem_ty) = translator.type_of_value(&op.array_value) else {
        unreachable!();
    };

    let array = translator.load(&op.array_value);
    let start_index = translator.load(&op.start_index);
    let searched_value = translator.load(&op.value);
    let write_index = translator.load(&op.return_index);

    let value_ty_id = translator.ty_of_val(&op.value);

    let len = translator.builder.declare_variable(int_ty);
    translator
        .builder
        .array_length(array.clone(), Value::Variable(len));
    let index = translator.builder.declare_variable(int_ty);
    translator
        .builder
        .store(start_index, Value::Variable(index));

    let loop_block = translator.builder.create_block();
    let loop_body_block = translator.builder.create_block();
    let found_block = translator.builder.create_block();
    let not_found_block = translator.builder.create_block();
    let merge_block = translator.builder.create_block();

    translator
        .builder
        .store(Value::Const(Literal::Int(-1)), write_index.clone());

    translator.builder.jump(loop_block);
    translator.builder.switch_to_block(loop_block);
    let exit_loop = translator.builder.declare_variable(bool_ty);
    translator.builder.compare_int(
        Value::Variable(index),
        Value::Variable(len),
        Value::Variable(exit_loop),
        ScalarCompareOp::Equal,
    );
    translator
        .builder
        .brif_true(Value::Variable(exit_loop), merge_block, loop_body_block);

    translator.builder.switch_to_block(loop_body_block);
    let value = get_elem(translator, array, Value::Variable(index), &elem_ty);
    let is_eq = check_elem_equality(translator, value, searched_value, value_ty_id);
    translator
        .builder
        .brif_true(Value::Variable(is_eq), found_block, not_found_block);

    translator.builder.switch_to_block(found_block);
    translator
        .builder
        .store(Value::Variable(index), write_index);
    translator.builder.jump(merge_block);

    translator.builder.switch_to_block(not_found_block);
    if reverse {
        translator.builder.isub(
            Value::Variable(index),
            Value::Const(1),
            Value::Variable(index),
        );
    } else {
        translator.builder.iadd(
            Value::Variable(index),
            Value::Const(1),
            Value::Variable(index),
        );
    }

    translator.builder.jump(loop_block);

    translator.builder.switch_to_block(merge_block);
}

fn check_elem_equality(
    translator: &mut FunctionTranslator<'_>,
    lhs: Value<Literal>,
    rhs: Value<Literal>,
    elem_ty: TypeId,
) -> Variable {
    let result = translator
        .builder
        .declare_variable(translator.ty(&Type::Bool));
    let dst = Value::Variable(result);
    if elem_ty == translator.data.resources.primitives.bool {
        let lhs = lhs.map(|lit| lit.to_bool().unwrap());
        let rhs = rhs.map(|lit| lit.to_bool().unwrap());

        translator
            .builder
            .compare_bool(lhs, rhs, dst, ScalarCompareOp::Equal);
    } else if elem_ty == translator.data.resources.primitives.int {
        let lhs = lhs.map(|lit| lit.to_int().unwrap());
        let rhs = rhs.map(|lit| lit.to_int().unwrap());

        translator
            .builder
            .compare_int(lhs, rhs, dst, ScalarCompareOp::Equal);
    } else if elem_ty == translator.data.resources.primitives.float {
        let lhs = lhs.map(|lit| lit.to_float().unwrap());
        let rhs = rhs.map(|lit| lit.to_float().unwrap());

        translator
            .builder
            .compare_float(lhs, rhs, dst, ScalarCompareOp::Equal);
    } else if elem_ty == translator.data.resources.primitives.string {
        let lhs = lhs.map(|lit| lit.to_string().unwrap());
        let rhs = rhs.map(|lit| lit.to_string().unwrap());

        translator
            .builder
            .compare_string(lhs, rhs, dst, StringCompareOp::Equal);
    } else {
        // Object/Struct/Array
        let lhs = lhs.map(|lit| lit.to_null().unwrap());
        let rhs = rhs.map(|lit| lit.to_null().unwrap());
        translator
            .builder
            .compare_addr(lhs, rhs, dst, AddrCompareOp::Equal);
    }

    result
}
